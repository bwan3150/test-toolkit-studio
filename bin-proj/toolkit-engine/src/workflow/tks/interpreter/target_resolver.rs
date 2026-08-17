// 目标解析模块 - 将参数解析为坐标点
// 解析结果（坐标 + 实时元素边界框）记录到 ActionTrace，供工作流标注截图用

use crate::{Result, TkeError, TksParam, Point, Controller, Recognizer, LocatorStrategy};
use crate::utils::Workarea;
use tracing::{debug, info, error};

use super::ActionTrace;

/// 目标解析器
pub struct TargetResolver<'a> {
    workarea: &'a Workarea,
    controller: &'a mut Controller,
    recognizer: &'a Recognizer,
    trace: &'a mut ActionTrace,
    healer: Option<std::sync::Arc<dyn super::super::ElementHealer>>,
}

impl<'a> TargetResolver<'a> {
    pub fn new(
        workarea: &'a Workarea,
        controller: &'a mut Controller,
        recognizer: &'a Recognizer,
        trace: &'a mut ActionTrace,
        healer: Option<std::sync::Arc<dyn super::super::ElementHealer>>,
    ) -> Self {
        Self {
            workarea,
            controller,
            recognizer,
            trace,
            healer,
        }
    }

    /// 解析目标位置
    pub async fn resolve(&mut self, param: &TksParam) -> Result<Point> {
        let point = match param {
            TksParam::Coordinate(point) => {
                debug!("使用坐标: ({}, {})", point.x, point.y);
                *point
            }
            TksParam::Element { name, strategy } => {
                debug!("查找元素: {}, 策略: {:?}", name, strategy);
                self.resolve_element(name, strategy).await?
            }
            TksParam::Text(text) => {
                debug!("查找文本元素: {}", text);
                self.resolve_text(text).await?
            }
            _ => {
                error!("无效的目标类型: {:?}", param);
                return Err(TkeError::InvalidArgument("无效的目标类型".to_string()));
            }
        };

        // 落在密码框上就标记：命令原文里的值要在落盘前打码（log/报告/截图横幅）。
        // 按**当前页面结构**判断而不是猜命令文本里有没有"密码"二字——
        // 三个平台同一条路（安卓 uiautomator 原生就有 password 属性，web 侧已对齐）
        if self.hits_password(point) {
            self.trace.sensitive = true;
        }

        // 记录解析出的坐标到执行轨迹
        self.trace.points.push(point);

        Ok(point)
    }

    /// 解析元素定位（带隐式等待：找不到就重新采集重试，应对页面尚未加载完）
    /// 该坐标是不是落在密码框上（用刚采集的页面结构判断）
    fn hits_password(&self, p: Point) -> bool {
        crate::Fetcher::new()
            .fetch_elements_from_file(&self.workarea.ui_tree_path())
            .map(|els| els.iter().any(|e| e.is_password && e.bounds.contains(p)))
            .unwrap_or(false)
    }

    async fn resolve_element(&mut self, name: &str, strategy: &LocatorStrategy) -> Result<Point> {
        self.trace.element_name = Some(name.to_string());
        // 最多 ~6s（12 次 × 500ms），覆盖慢加载/切换动画
        const MAX_TRIES: usize = 12;
        let mut last_err = None;
        let mut heal_tried = false;
        for attempt in 0..MAX_TRIES {
            // 中断检查点：Ctrl+C 后不再继续 ~6s 的重试，立即返回（否则按了得等这步重试/超时跑完）
            if crate::utils::interrupt::aborted() {
                return Err(TkeError::DeviceError("已中断（用户 Ctrl+C）".to_string()));
            }
            if let Err(e) = self.controller.capture_ui_state(self.workarea).await {
                error!("刷新UI状态失败: {}", e);
                return Err(e);
            }
            self.trace.captured = true;
            match self.recognizer.find_element_detailed(name, strategy.clone()).await {
                Ok((point, bounds)) => {
                    info!("找到元素 '{}' 位置: ({}, {})", name, point.x, point.y);
                    self.trace.bounds = Some(bounds);
                    return Ok(point);
                }
                Err(e) => {
                    last_err = Some(e);
                    // 定位自愈（Healenium 式）：重试几次仍找不到 → 让 healer 基于**刚采集的
                    // 当前页面**判断"哪个元素其实就是它"（改版/文字微调）。命中=本步当场救活
                    // （healer 同时把修正持久化进元素库，供以后回放）；救不了继续原重试路径。
                    if !heal_tried && attempt >= 2 {
                        heal_tried = true;
                        if let Some(h) = self.healer.clone() {
                            info!("元素 '{}' 连续定位失败，尝试自愈…", name);
                            if let Some((point, bounds)) = h.heal(name, self.workarea).await {
                                info!("自愈成功：'{}' → ({}, {})", name, point.x, point.y);
                                self.trace.bounds = Some(bounds);
                                return Ok(point);
                            }
                        }
                    }
                    if attempt + 1 < MAX_TRIES {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }
        let e = last_err.unwrap();
        error!("查找元素 '{}' 失败（重试 {} 次后）: {}", name, MAX_TRIES, e);
        Err(e)
    }

    /// 解析文本查找（**带隐式等待**，与元素定位同一套策略）
    ///
    /// 为什么必须有：文字定位是首选写法，如果找不到就立刻失败，调用方（AI）只能在每步后面
    /// 撒 `等待 [1s]` 来兜底——实测一次检查 47 步里有 22 步是等待，**近一半时间花在死等上**。
    /// 有了隐式等待就反过来了：**元素已经在就立刻返回**（不浪费一毫秒），没渲染完才等，
    /// 而且能等够 6 秒（比死等 1 秒更可靠）。
    async fn resolve_text(&mut self, text: &str) -> Result<Point> {
        const MAX_TRIES: usize = 12; // 最多 ~6s（12 × 500ms），与 resolve_element 保持一致
        self.trace.element_name = Some(text.to_string());
        let mut last_err = None;

        for attempt in 0..MAX_TRIES {
            // 中断检查点：Ctrl+C 后不再耗完整轮重试
            if crate::utils::interrupt::aborted() {
                return Err(TkeError::DeviceError("已中断（用户 Ctrl+C）".to_string()));
            }
            if let Err(e) = self.controller.capture_ui_state(self.workarea).await {
                error!("刷新UI状态失败: {}", e);
                return Err(e);
            }
            self.trace.captured = true;

            match self.recognizer.find_element_by_text(text) {
                Ok((point, bounds)) => {
                    if attempt > 0 {
                        info!("文本 '{}' 在第 {} 次重试后出现", text, attempt + 1);
                    }
                    info!("找到文本元素 '{}' 位置: ({}, {})", text, point.x, point.y);
                    self.trace.bounds = Some(bounds);
                    return Ok(point);
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < MAX_TRIES {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }
        let e = last_err.unwrap();
        error!("查找文本元素 '{}' 失败（重试 {} 次后）: {}", text, MAX_TRIES, e);
        Err(e)
    }
}
