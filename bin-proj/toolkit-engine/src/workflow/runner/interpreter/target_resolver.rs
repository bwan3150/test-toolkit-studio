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
}

impl<'a> TargetResolver<'a> {
    pub fn new(
        workarea: &'a Workarea,
        controller: &'a mut Controller,
        recognizer: &'a Recognizer,
        trace: &'a mut ActionTrace,
    ) -> Self {
        Self {
            workarea,
            controller,
            recognizer,
            trace,
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

        // 记录解析出的坐标到执行轨迹
        self.trace.points.push(point);

        Ok(point)
    }

    /// 解析元素定位（带隐式等待：找不到就重新采集重试，应对页面尚未加载完）
    async fn resolve_element(&mut self, name: &str, strategy: &LocatorStrategy) -> Result<Point> {
        self.trace.element_name = Some(name.to_string());
        // 最多 ~6s（12 次 × 500ms），覆盖慢加载/切换动画
        const MAX_TRIES: usize = 12;
        let mut last_err = None;
        for attempt in 0..MAX_TRIES {
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

    /// 解析文本查找
    async fn resolve_text(&mut self, text: &str) -> Result<Point> {
        // 刷新UI状态
        if let Err(e) = self.controller.capture_ui_state(self.workarea).await {
            error!("刷新UI状态失败: {}", e);
            return Err(e);
        }
        self.trace.captured = true;
        self.trace.element_name = Some(text.to_string());

        match self.recognizer.find_element_by_text(text) {
            Ok((point, bounds)) => {
                info!("找到文本元素 '{}' 位置: ({}, {})", text, point.x, point.y);
                self.trace.bounds = Some(bounds);
                Ok(point)
            }
            Err(e) => {
                error!("查找文本元素 '{}' 失败: {}", text, e);
                Err(e)
            }
        }
    }
}
