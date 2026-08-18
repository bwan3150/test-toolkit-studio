// 脚本解释器模块 - 将TKS指令转换为可执行的ADB指令

mod command_executor;
mod param_extractor;
mod target_resolver;

use crate::{Result, TkeError, TksStep, TksCommand, Controller, Recognizer, Platform, Point, Bounds};
use crate::utils::Workarea;
use std::path::Path;
use tracing::debug;

use command_executor::CommandExecutor;

use super::ElementHealer;
use std::sync::Arc;

/// 单步执行轨迹 - 记录定位结果，供工作流标注截图/定位问题用
#[derive(Debug, Default, Clone)]
pub struct ActionTrace {
    /// 本步执行中是否已采集过页面状态（截图+XML）
    pub captured: bool,
    /// 本步解析出的目标坐标（点击点/滑动起终点）
    pub points: Vec<Point>,
    /// 目标元素的实时边界框（来自当前页面实际匹配到的元素，用于画框）
    pub bounds: Option<Bounds>,
    /// 目标元素名称
    pub element_name: Option<String>,
    /// 本步碰的是**密码框**——命令原文里那个值必须在落盘前打码。
    /// `--log` 会把命令写进 log.json、报告，还会**烧进标注截图的顶部横幅**
    /// （实测：代填的密码就那样明晃晃印在图上，而报告是拿去分享的）。
    pub sensitive: bool,
}

impl ActionTrace {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 脚本解释器
pub struct ScriptInterpreter {
    workarea: Workarea,
    #[allow(dead_code)]
    device_id: Option<String>,
    controller: Controller,
    recognizer: Recognizer,
    /// 定位自愈钩子（None=关闭；replay/repair 装配时注入）
    healer: Option<Arc<dyn ElementHealer>>,
    /// 元素库路径（断言页面从库里读页面实体；None=无库）
    element_path: Option<std::path::PathBuf>,
    /// 最近一步的执行轨迹
    pub last_trace: ActionTrace,
    /// 本次运行中启动过的 Android 包名（flow 收尾统一关闭用）
    pub launched_packages: Vec<String>,
}

impl ScriptInterpreter {
    /// 创建新的解释器实例
    /// element_path: 元素库路径（None 按默认路径查找）
    /// workarea: 页面采集工作区
    pub fn new(
        device_id: Option<String>,
        element_path: Option<&Path>,
        workarea: Workarea,
    ) -> Result<Self> {
        let platform = Platform::from_device(device_id.as_deref());
        let controller = Controller::new(device_id.clone())?;
        let recognizer = Recognizer::new(element_path, workarea.clone(), platform)?;

        Ok(Self {
            workarea,
            device_id,
            controller,
            recognizer,
            healer: None,
            element_path: element_path.map(|p| p.to_path_buf()),
            last_trace: ActionTrace::default(),
            launched_packages: Vec::new(),
        })
    }

    /// 注入定位自愈钩子
    pub fn set_healer(&mut self, healer: Arc<dyn ElementHealer>) {
        self.healer = Some(healer);
    }

    /// 解释并执行单个步骤
    pub async fn interpret_step(&mut self, step: &TksStep) -> Result<()> {
        debug!("执行步骤: {} (行号: {})", step.raw, step.line_number);

        self.last_trace.reset();

        // 有对话框挂着时,除了处理对话框本身,**什么都干不了**：WebDriver 会对每条命令
        // 回一句 `unexpected alert open`——那串错跟"元素找不到"长得差不多,AI 多半会
        // 去改定位、重试、绕路,而真正该做的只是先把对话框点掉。把话说清楚（P-37）
        if !matches!(step.command,
            TksCommand::DialogAccept | TksCommand::DialogDismiss | TksCommand::DialogInput)
        {
            if let Some(d) = self.controller.dialog_text() {
                return Err(TkeError::ScriptExecuteError(format!(
                    "页面上有个对话框还没处理：「{}」——它挡着所有操作。\
                     先用 `确认对话框` 或 `取消对话框`（prompt 用 `对话框输入 [\"文本\"]`）",
                    d
                )));
            }
        }

        let mut executor = CommandExecutor::new(
            &self.workarea,
            &mut self.controller,
            &self.recognizer,
            &mut self.last_trace,
            &mut self.launched_packages,
            self.healer.clone(),
            self.element_path.as_deref(),
        );

        match step.command {
            TksCommand::Launch => executor.execute_launch(&step.params).await,
            TksCommand::Close => executor.execute_close(&step.params).await,
            TksCommand::Click => executor.execute_click(&step.params).await,
            TksCommand::Hover => executor.execute_hover(&step.params).await,
            TksCommand::Select => executor.execute_select(&step.params).await,
            TksCommand::Press => executor.execute_press(&step.params).await,
            TksCommand::Swipe => executor.execute_swipe(&step.params).await,
            TksCommand::DirectionalSwipe => executor.execute_directional_swipe(&step.params).await,
            TksCommand::Input => executor.execute_input(&step.params).await,
            TksCommand::Clear => executor.execute_clear(&step.params).await,
            TksCommand::HideKeyboard => executor.execute_hide_keyboard().await,
            TksCommand::Back => executor.execute_back().await,
            TksCommand::Wait => executor.execute_wait(&step.params).await,
            TksCommand::Assert => executor.execute_assert(&step.params).await,
            TksCommand::AssertPage => executor.execute_assert_page(&step.params).await,
            TksCommand::Switch => executor.execute_switch(&step.params).await,
            TksCommand::ScrollFind => executor.execute_scroll_find(&step.params).await,
            TksCommand::Key => executor.execute_key(&step.params).await,
            TksCommand::DialogAccept => executor.execute_dialog_accept().await,
            TksCommand::DialogDismiss => executor.execute_dialog_dismiss().await,
            TksCommand::DialogInput => executor.execute_dialog_input(&step.params).await,
        }
    }

    /// 当前有没有未处理的原生对话框（alert/confirm/prompt）；有则返回它的文字。
    /// **只有 web 会返回 Some**。见 drivers::Controller::dialog_text
    pub fn dialog_text(&self) -> Option<String> {
        self.controller.dialog_text()
    }

    /// 取走浏览器日志里的错误（只有 web 有）。见 drivers::Controller::console_errors
    pub fn console_errors(&self) -> Vec<String> {
        self.controller.console_errors()
    }

    /// 主动采集一次页面状态（截图+XML 到工作区），供工作流补采产物
    pub async fn capture_state(&self) -> Result<()> {
        self.controller.capture_ui_state(&self.workarea).await
    }


    /// 工作区引用
    pub fn workarea(&self) -> &Workarea {
        &self.workarea
    }
}
