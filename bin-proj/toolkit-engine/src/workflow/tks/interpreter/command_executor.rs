// 命令执行器模块 - 执行各种 TKS 命令

use crate::{Result, TkeError, TksParam, Point, Controller, Recognizer, LocatorStrategy};
use crate::atomic::control::{execute_action, ControlAction, DialogAction};
use crate::utils::Workarea;
use tracing::{debug, info};

use super::param_extractor::ParamExtractor;
use super::target_resolver::TargetResolver;
use super::ActionTrace;

/// 命令执行器
pub struct CommandExecutor<'a> {
    workarea: &'a Workarea,
    controller: &'a mut Controller,
    recognizer: &'a Recognizer,
    trace: &'a mut ActionTrace,
    /// 本次运行启动过的 Android 包名（flow 收尾关闭用）
    launched: &'a mut Vec<String>,
    /// 定位自愈钩子（None=关闭）
    healer: Option<std::sync::Arc<dyn super::super::ElementHealer>>,
    /// 元素库路径（断言页面用）
    element_path: Option<&'a std::path::Path>,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(
        workarea: &'a Workarea,
        controller: &'a mut Controller,
        recognizer: &'a Recognizer,
        trace: &'a mut ActionTrace,
        launched: &'a mut Vec<String>,
        healer: Option<std::sync::Arc<dyn super::super::ElementHealer>>,
        element_path: Option<&'a std::path::Path>,
    ) -> Self {
        Self {
            workarea,
            controller,
            recognizer,
            trace,
            launched,
            healer,
            element_path,
        }
    }

    /// 启动: Android = [包名, Activity]；Web = [URL]
    pub async fn execute_launch(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument(
                "启动命令需要目标: [包名, Activity] (Android) / [BundleID] (iOS) / [URL] (Web)".to_string()));
        }

        let package = ParamExtractor::extract_text(&params[0])?;
        let activity = if params.len() > 1 {
            ParamExtractor::extract_text(&params[1])?
        } else {
            String::new()
        };

        // 设备操作走统一执行器；启动后的「记录包名 + 等待 + 刷新」是工作流额外动作，保留
        execute_action(&*self.controller, ControlAction::Launch { package: package.clone(), activity }).await?;

        // 记录启动过的包/BundleID（flow 收尾统一关闭; web 的会话销毁不走这里）
        if self.recognizer.platform() != crate::Platform::Web
            && !self.launched.contains(&package)
        {
            self.launched.push(package.clone());
        }

        // 等待应用启动
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        // 刷新UI状态
        self.controller.capture_ui_state(self.workarea).await?;
        self.trace.captured = true;

        Ok(())
    }

    /// 关闭: Android = [包名]；Web = [URL 或任意值]（销毁浏览器会话，与 URL 无关）
    pub async fn execute_close(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument(
                "关闭命令需要目标: [包名] (Android) / [BundleID] (iOS) / [URL] (Web)".to_string()));
        }
        let package = ParamExtractor::extract_text(&params[0])?;
        execute_action(&*self.controller, ControlAction::Close { package }).await.map(|_| ())
    }

    /// 切换：web=标签序号 或 用新标签打开 URL；移动端=切到目标 App 包名
    pub async fn execute_switch(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument(
                "切换命令需要目标: [标签序号] / [URL] (Web) / [包名] (App)".to_string()));
        }
        let target = ParamExtractor::extract_text(&params[0])?;
        execute_action(&*self.controller, ControlAction::Switch { target }).await.map(|_| ())
    }

    /// 点击操作
    pub async fn execute_click(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("点击命令需要目标参数".to_string()));
        }

        let point = self.resolve_target(&params[0]).await?;
        execute_action(&*self.controller, ControlAction::Click { point }).await.map(|_| ())
    }

    /// 悬停操作（web 独有）：把鼠标移到目标元素上触发 hover、展开悬停下拉/菜单，不点击
    pub async fn execute_hover(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("悬停命令需要目标参数".to_string()));
        }

        let point = self.resolve_target(&params[0]).await?;
        execute_action(&*self.controller, ControlAction::Hover { point }).await.map(|_| ())
    }

    /// 选择下拉项（web 独有）：`选择 [{下拉框}, "日间模式"]` 或 `选择 ["模式", "日间模式"]`
    pub async fn execute_select(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument(
                "选择命令需要两个参数：目标下拉框 + 选项文字，如 选择 [{模式}, \"日间模式\"]".to_string(),
            ));
        }
        let point = self.resolve_target(&params[0]).await?;
        let label = match &params[1] {
            TksParam::Text(t) => t.clone(),
            other => return Err(TkeError::InvalidArgument(format!(
                "选择的第二个参数要是选项文字（双引号），收到：{:?}", other
            ))),
        };
        let picked = self.controller.select_option(point.x, point.y, &label)?;
        tracing::info!("已选中「{}」", picked);
        Ok(())
    }

    /// 长按操作
    pub async fn execute_press(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("按压命令需要目标参数".to_string()));
        }

        let point = self.resolve_target(&params[0]).await?;
        let duration = if params.len() > 1 {
            ParamExtractor::extract_duration(&params[1])?
        } else {
            1000 // 默认1秒
        };

        execute_action(&*self.controller, ControlAction::Press { point, duration_ms: duration }).await.map(|_| ())
    }

    /// 滑动操作
    pub async fn execute_swipe(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("滑动命令需要起点和终点".to_string()));
        }

        let from_point = self.resolve_target(&params[0]).await?;
        let to_point = self.resolve_target(&params[1]).await?;
        let duration = if params.len() > 2 {
            ParamExtractor::extract_duration(&params[2])?
        } else {
            300 // 默认300ms
        };

        execute_action(&*self.controller, ControlAction::Swipe { from: from_point, to: to_point, duration_ms: duration }).await.map(|_| ())
    }

    /// 定向滑动操作
    pub async fn execute_directional_swipe(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 3 {
            return Err(TkeError::InvalidArgument("定向滑动命令需要起点、方向和距离".to_string()));
        }

        let from_point = self.resolve_target(&params[0]).await?;
        let direction = ParamExtractor::extract_direction(&params[1])?;
        let distance = ParamExtractor::extract_number(&params[2])?;
        let duration = if params.len() > 3 {
            ParamExtractor::extract_duration(&params[3])?
        } else {
            300 // 默认300ms
        };

        // 方向→终点的换算由 execute_action 内部统一处理
        execute_action(&*self.controller, ControlAction::SwipeDir { from: from_point, direction, distance, duration_ms: duration }).await.map(|_| ())
    }

    /// 滚动查找：朝指定方向滚动，直到目标文字在页面出现（可复现地"滚到目标可见"，
    /// 替代固定距离盲滑——固定滑动受加载时机影响、回放与探索的滚动位置常不一致）。
    /// 参数：[目标文字(Text), 方向(Direction)]。后面通常跟一条 click 点该目标。
    pub async fn execute_scroll_find(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("滚动查找命令需要目标文字和方向: [\"文字\", 方向]".to_string()));
        }
        let target_raw = ParamExtractor::extract_text(&params[0])?;
        // 多候选关键词（`|` 分隔）：任一命中即算找到。手写脚本/未收敛的多候选都支持。
        let cands = crate::utils::scroll::targets(&target_raw);
        if cands.is_empty() {
            return Err(TkeError::InvalidArgument("滚动查找需要目标文字".to_string()));
        }
        let direction = ParamExtractor::extract_direction(&params[1])?;
        // 每滚一步都查（含 OCR：DATA SHEET 这类图片/图标文字只有 OCR 能识别）。滚动幅度取**近全屏**
        // （少重复检查同一批元素/OCR）；不定死次数，而是**滚一次→查→比上一屏是否大部分相同**：相同
        // 就是滚到底/到顶、滚不动了，立刻停（不再卡在边界空转）。安全上限防病态死循环。
        const SAFETY_MAX: usize = 40;
        let (sw, sh) = image::image_dimensions(self.workarea.screenshot_path()).unwrap_or((1080, 1920));
        let vertical = !matches!(direction.as_str(), "left" | "right");
        // 主手势从屏幕中心；"没动"时换个起点重试，避开中间嵌套可滚动区（图片/轮播）吃掉手势——
        // 竖向改从下方 3/4 处、横向改从右侧 3/4 处发起，更可能作用到主页面。
        let from = Point::new(sw as i32 / 2, sh as i32 / 2);
        let alt = if vertical { Point::new(sw as i32 / 2, sh as i32 * 3 / 4) } else { Point::new(sw as i32 * 3 / 4, sh as i32 / 2) };
        let dim = if vertical { sh as i32 } else { sw as i32 };
        let step = (dim * 4 / 5).max(200);
        let ocr_src = crate::engines::ocr::ocr_source();
        let mut prev_texts: Option<Vec<String>> = None;
        let mut stuck = 0u32; // 连续"页面没变"的次数
        for i in 0..SAFETY_MAX {
            // 中断（Ctrl+C）/软停（Esc）：每滚一次前都查，立即停下滚动查找（不必等滚到底/到顶）。
            if crate::utils::interrupt::aborted() || crate::utils::interrupt::pause_requested() {
                return Err(TkeError::DeviceError("已中断（用户 Ctrl+C）".to_string()));
            }
            self.controller.capture_ui_state(self.workarea).await?;
            self.trace.captured = true;
            let mut els = crate::Fetcher::new().fetch_elements_from_file(&self.workarea.ui_tree_path()).unwrap_or_default();
            if let Some(src) = ocr_src.as_ref() {
                if let Ok(bytes) = std::fs::read(self.workarea.screenshot_path()) {
                    let _ = crate::engines::ocr::enrich_with_ocr(&mut els, &bytes, src).await;
                }
            }
            let texts: Vec<String> = els.iter().map(|e| e.to_ai_text()).collect();
            if let Some(hit) = crate::utils::scroll::first_hit(&texts, &cands) {
                info!("滚动查找：目标「{}」已可见（滚动 {} 次）", hit, i);
                return Ok(());
            }
            // 与上一屏大部分相同 = 这次没滚动。连续 **4 次**没动才算真到底/到顶（个别可能是中间
            // 嵌套区吃了手势）；第一次没动就换起点(alt)再试。
            if prev_texts.as_ref().map(|p| crate::utils::scroll::page_stuck(p, &texts)).unwrap_or(false) {
                stuck += 1;
                if stuck >= 4 {
                    break;
                }
            } else {
                stuck = 0;
            }
            prev_texts = Some(texts);
            let origin = if stuck > 0 { alt } else { from };
            execute_action(
                &*self.controller,
                ControlAction::SwipeDir { from: origin, direction: direction.clone(), distance: step, duration_ms: 300 },
            )
            .await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err(TkeError::ElementNotFound(format!("滚动查找：滚到底/到顶仍未出现「{}」", target_raw)))
    }

    /// 输入文本（execute_action 的 Input 已含「点击聚焦 + 等待键盘 + 输入」）
    pub async fn execute_input(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("输入命令需要目标和文本".to_string()));
        }

        let point = self.resolve_target(&params[0]).await?;
        let text = ParamExtractor::extract_text(&params[1])?;
        let r = execute_action(&*self.controller, ControlAction::Input { text, point: Some(point) }).await?;
        // 写进的是密码框 → 标记这一步敏感，命令原文在落盘前会被打码。
        // 判据取自**焦点所在的元素**：光看"解析出的坐标上有什么"会漏——
        // `输入 ["密码", …]` 常常命中的是 <label>，点它同样能聚焦到密码框（实测撞过）
        if r.get("sensitive").and_then(|v| v.as_bool()).unwrap_or(false) {
            self.trace.sensitive = true;
        }
        Ok(())
    }

    /// 清理输入框
    pub async fn execute_clear(&mut self, params: &[TksParam]) -> Result<()> {
        if !params.is_empty() {
            // 先点击聚焦输入框 + 等待键盘（工作流额外动作）
            let point = self.resolve_target(&params[0]).await?;
            execute_action(&*self.controller, ControlAction::Click { point }).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        execute_action(&*self.controller, ControlAction::Clear).await.map(|_| ())
    }

    /// 隐藏键盘
    pub async fn execute_hide_keyboard(&mut self) -> Result<()> {
        execute_action(&*self.controller, ControlAction::HideKeyboard).await.map(|_| ())
    }

    /// 启动环境：`启动环境` / `启动环境 [有头]` / `启动环境 [无头]`
    ///
    /// 放在脚本第一行,读的人一眼知道这个脚本需要什么环境——
    /// 早先浏览器是被第一条 web 命令**顺带**起来的,脚本里看不出这件事
    pub async fn execute_boot(&mut self, params: &[TksParam]) -> Result<()> {
        // 参数可省。写了就必须认得,别把 `启动环境 [有头]` 里的错字静默当成默认
        let headed = match params.first() {
            None => None,
            Some(p) => {
                let v = ParamExtractor::extract_text(&p.clone())?;
                match v.trim() {
                    "有头" | "有窗口" | "headed" => Some(true),
                    "无头" | "headless" => Some(false),
                    other => {
                        return Err(TkeError::InvalidArgument(format!(
                            "启动环境的参数只能是 有头 / 无头（收到 {}）",
                            other
                        )))
                    }
                }
            }
        };
        execute_action(&*self.controller, ControlAction::Boot { headed }).await.map(|_| ())
    }

    /// 关闭环境：关浏览器进程 / 模拟器关机
    pub async fn execute_shutdown(&mut self) -> Result<()> {
        execute_action(&*self.controller, ControlAction::Shutdown).await.map(|_| ())
    }

    // 对话框三条都走 execute_action：那里是「动作 → 设备」的唯一映射，
    // 直接调 controller 的话 CLI / tks / agent 就各走各的了

    /// 确认对话框：原生 alert/confirm 的「确定」
    pub async fn execute_dialog_accept(&mut self) -> Result<()> {
        execute_action(&*self.controller, ControlAction::Dialog { action: DialogAction::Accept })
            .await
            .map(|_| ())
    }

    /// 取消对话框：原生 confirm 的「取消」
    pub async fn execute_dialog_dismiss(&mut self) -> Result<()> {
        execute_action(&*self.controller, ControlAction::Dialog { action: DialogAction::Dismiss })
            .await
            .map(|_| ())
    }

    /// 对话框输入：往 prompt 里填字**并确定**
    pub async fn execute_dialog_input(&mut self, params: &[TksParam]) -> Result<()> {
        let text = ParamExtractor::extract_text(&params.first().ok_or_else(|| {
            TkeError::InvalidArgument("对话框输入需要文本，如 [\"张三\"]".to_string())
        })?.clone())?;
        execute_action(&*self.controller, ControlAction::Dialog { action: DialogAction::Input(text) })
            .await
            .map(|_| ())
    }

    /// 按键：`按键 ["ENTER"]` / `["TAB"]` 等（归一大写传给驱动 key_event，web 映射为 WebDriver 键码）
    pub async fn execute_key(&mut self, params: &[TksParam]) -> Result<()> {
        let code = ParamExtractor::extract_text(&params.first().ok_or_else(|| {
            TkeError::InvalidArgument("按键命令需要键名，如 [\"ENTER\"]".to_string())
        })?.clone())?
        .trim()
        .to_uppercase();
        execute_action(&*self.controller, ControlAction::Key { code }).await.map(|_| ())
    }

    /// 返回操作
    pub async fn execute_back(&mut self) -> Result<()> {
        execute_action(&*self.controller, ControlAction::Back).await.map(|_| ())
    }

    /// 等待操作
    pub async fn execute_wait(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            // 默认等待1秒
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            return Ok(());
        }

        match &params[0] {
            TksParam::Duration(ms) => {
                debug!("等待 {} 毫秒", ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms as u64)).await;
            }
            TksParam::Number(num) => {
                // 裸数字一律按**毫秒**——必须与产出方同口径：探索/动作写脚本就是 `等待 [毫秒数]`
                // （execution 里实时 sleep(from_millis(ms))）。要表达秒请用 `Ns` 形式（解析为 Duration）。
                // 曾经"≤3600 当秒"的启发式会把 `等待 1000`(本意 1 秒) 在回放里睡成 1000 秒。
                let ms = (*num).max(0) as u64;
                debug!("等待 {} 毫秒", ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            }
            TksParam::Element { name, strategy } => {
                // 可选第二参数 = 超时秒数: 等待 [{元素}, 90]（缺省 30s）
                let timeout_secs = match params.get(1) {
                    Some(TksParam::Number(n)) => *n as u64,
                    Some(TksParam::Duration(ms)) => (*ms as u64 / 1000).max(1),
                    Some(TksParam::Text(t)) => t.parse().unwrap_or(30),
                    _ => 30,
                };
                debug!("等待元素出现: {}, 策略: {:?}, 超时: {}s", name, strategy, timeout_secs);
                self.wait_for_element(name, strategy, timeout_secs).await?;
            }
            TksParam::Text(text) => {
                // 与裸数字同口径：毫秒
                if let Ok(ms) = text.parse::<u64>() {
                    debug!("等待 {} 毫秒 (文本解析)", ms);
                    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
                } else {
                    return Err(TkeError::InvalidArgument(format!("无法解析等待参数: {}", text)));
                }
            }
            _ => {
                return Err(TkeError::InvalidArgument("等待命令参数无效".to_string()));
            }
        }

        Ok(())
    }

    /// 等待元素出现
    async fn wait_for_element(&mut self, name: &str, strategy: &LocatorStrategy, timeout_secs: u64) -> Result<()> {
        let timeout = tokio::time::Duration::from_secs(timeout_secs);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            // 刷新UI状态
            if let Err(e) = self.controller.capture_ui_state(self.workarea).await {
                debug!("刷新UI状态失败: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            self.trace.captured = true;

            // 尝试查找元素
            if self.recognizer.find_element(name, strategy.clone()).await.is_ok() {
                info!("找到元素: {}", name);
                return Ok(());
            } else {
                debug!("未找到元素: {}", name);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(TkeError::ScriptExecuteError(format!("等待元素超时: {}", name)))
    }

    /// 页面断言：当前页与元素包里存的「页面」特征集做命中率匹配（≥60% 判在此页）。
    /// 起始/终点校验的规范形式——真实执行、失败信息带页面 desc 与命中率，人和 AI 都能看懂差在哪。
    pub async fn execute_assert_page(&mut self, params: &[TksParam]) -> Result<()> {
        let name = match params.first() {
            Some(TksParam::Text(t)) => t.clone(),
            _ => return Err(TkeError::InvalidArgument("断言页面需要页面名，如 断言页面 [\"起始页\"]".to_string())),
        };
        let lib = self.element_path.ok_or_else(|| {
            TkeError::InvalidArgument("断言页面需要元素库（.tklib）——库里存着页面特征".to_string())
        })?;
        let (desc, signature) = crate::tools::element::get_page(lib, &name).ok_or_else(|| {
            TkeError::InvalidArgument(format!("元素包里没有页面「{}」——请检查 .tklib 是否完整", name))
        })?;
        if signature.is_empty() {
            return Err(TkeError::InvalidArgument(format!("页面「{}」的特征集为空，无法匹配", name)));
        }
        // 刷新并解析当前页
        self.controller.capture_ui_state(self.workarea).await?;
        self.trace.captured = true;
        let elements = crate::Fetcher::new().fetch_elements_from_file(&self.workarea.ui_tree_path())?;
        let texts: Vec<String> = elements
            .iter()
            .filter_map(|e| e.text.clone().or_else(|| e.content_desc.clone()))
            .filter(|t| !t.trim().is_empty())
            .collect();
        let (hit, total) = crate::tools::element::page_match_score(&signature, &texts);
        let ratio = hit as f32 / total as f32;
        if ratio >= crate::tools::element::PAGE_MATCH_THRESHOLD {
            Ok(())
        } else {
            let seen = texts.iter().take(6).cloned().collect::<Vec<_>>().join("、");
            Err(TkeError::ScriptExecuteError(format!(
                "页面断言失败：期望在「{}」{}，特征命中 {}/{}（需≥{:.0}%）。当前页面前几项：{}",
                name,
                if desc.is_empty() { String::new() } else { format!("（{}）", desc) },
                hit,
                total,
                crate::tools::element::PAGE_MATCH_THRESHOLD * 100.0,
                seen
            )))
        }
    }

    /// 断言操作。
    ///
    /// 第三个参数（可选）是**最长等待时间**：`断言 [{提示}, 存在, 10s]` —— 在这段时间内
    /// 反复采集页面重判，一成立就通过。用于等异步结果（后台下发、跨设备同步、请求返回）——
    /// 固定 `等待 [Ns]` 快了假失败、慢了浪费，轮询才是对的。
    /// 不给这个参数则行为不变：采一次、判一次。
    pub async fn execute_assert(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("断言命令需要目标和条件".to_string()));
        }

        // 最长等待（第三参数）；没有就是 0 = 只判一次
        let wait_ms: u64 = params
            .iter()
            .skip(2)
            .find_map(|p| match p {
                TksParam::Duration(ms) => Some(*ms as u64),
                _ => None,
            })
            .unwrap_or(0);

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(wait_ms);
        loop {
            match self.assert_once(params).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // 超时到点、或用户中断 → 把最后一次的失败原因原样抛出
                    if std::time::Instant::now() >= deadline || crate::utils::interrupt::aborted() {
                        return Err(e);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// 采一次页面、判一次断言（execute_assert 的单次形态）
    async fn assert_once(&mut self, params: &[TksParam]) -> Result<()> {
        // 刷新UI状态
        self.controller.capture_ui_state(self.workarea).await?;
        self.trace.captured = true;

        let (element_exists, element_name) = match &params[0] {
            TksParam::Element { name, strategy } => {
                // 记录断言目标到执行轨迹（用于截图标注，bounds 为实时匹配结果）
                self.trace.element_name = Some(name.clone());
                let exists = match self.recognizer.find_element_detailed(name, strategy.clone()).await {
                    Ok((point, bounds)) => {
                        self.trace.points.push(point);
                        self.trace.bounds = Some(bounds);
                        true
                    }
                    Err(_) => false,
                };
                (exists, name.as_str())
            }
            _ => return Err(TkeError::InvalidArgument("断言目标必须是元素".to_string())),
        };

        let expected = match &params[1] {
            TksParam::Boolean(b) => *b,
            TksParam::Text(t) => t == "存在",
            _ => return Err(TkeError::InvalidArgument("断言条件无效".to_string())),
        };

        if element_exists != expected {
            return Err(TkeError::ScriptExecuteError(
                format!("断言失败: 元素 '{}' {}，但期望{}",
                       element_name,
                       if element_exists { "存在" } else { "不存在" },
                       if expected { "存在" } else { "不存在" })
            ));
        }

        Ok(())
    }

    /// 解析目标位置（内部调用 TargetResolver，并记录到 trace）
    async fn resolve_target(&mut self, param: &TksParam) -> Result<Point> {
        let mut resolver = TargetResolver::new(
            self.workarea,
            self.controller,
            self.recognizer,
            self.trace,
            self.healer.clone(),
        );
        resolver.resolve(param).await
    }
}
