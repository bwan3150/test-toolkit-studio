// 命令执行器模块 - 执行各种 TKS 命令

use crate::{Result, TkeError, TksParam, Point, Controller, Recognizer, LocatorStrategy};
use crate::atomic::control::{execute_action, ControlAction};
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
}

impl<'a> CommandExecutor<'a> {
    pub fn new(
        workarea: &'a Workarea,
        controller: &'a mut Controller,
        recognizer: &'a Recognizer,
        trace: &'a mut ActionTrace,
        launched: &'a mut Vec<String>,
    ) -> Self {
        Self {
            workarea,
            controller,
            recognizer,
            trace,
            launched,
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

    /// 输入文本（execute_action 的 Input 已含「点击聚焦 + 等待键盘 + 输入」）
    pub async fn execute_input(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("输入命令需要目标和文本".to_string()));
        }

        let point = self.resolve_target(&params[0]).await?;
        let text = ParamExtractor::extract_text(&params[1])?;
        execute_action(&*self.controller, ControlAction::Input { text, point: Some(point) }).await.map(|_| ())
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
                // 数字参数：如果小于等于3600，当作秒数；否则当作毫秒数
                if *num <= 3600 {
                    debug!("等待 {} 秒", num);
                    tokio::time::sleep(tokio::time::Duration::from_secs(*num as u64)).await;
                } else {
                    debug!("等待 {} 毫秒", num);
                    tokio::time::sleep(tokio::time::Duration::from_millis(*num as u64)).await;
                }
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
                // 支持文本参数的等待
                if let Ok(seconds) = text.parse::<u64>() {
                    debug!("等待 {} 秒 (文本解析)", seconds);
                    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
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

    /// 断言操作
    pub async fn execute_assert(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("断言命令需要目标和条件".to_string()));
        }

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
        );
        resolver.resolve(param).await
    }
}
