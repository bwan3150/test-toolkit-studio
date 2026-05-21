// 脚本解释器模块 - 将TKS指令转换为可执行的ADB指令

mod command_executor;
mod param_extractor;
mod target_resolver;

use crate::{Result, TksStep, TksCommand, Controller, Recognizer};
use std::path::PathBuf;
use tracing::debug;

use command_executor::CommandExecutor;

/// 脚本解释器
pub struct ScriptInterpreter {
    project_path: PathBuf,
    #[allow(dead_code)]
    device_id: Option<String>,
    controller: Controller,
    recognizer: Recognizer,
}

impl ScriptInterpreter {
    /// 创建新的解释器实例
    pub fn new(project_path: PathBuf, device_id: Option<String>) -> Result<Self> {
        let controller = Controller::new(device_id.clone())?;
        let recognizer = Recognizer::new(project_path.clone())?;

        Ok(Self {
            project_path,
            device_id,
            controller,
            recognizer,
        })
    }

    /// 解释并执行单个步骤
    pub async fn interpret_step(&mut self, step: &TksStep) -> Result<()> {
        debug!("执行步骤: {} (行号: {})", step.raw, step.line_number);

        let mut executor = CommandExecutor::new(
            &self.project_path,
            &mut self.controller,
            &self.recognizer,
        );

        match step.command {
            TksCommand::Launch => executor.execute_launch(&step.params).await,
            TksCommand::Close => executor.execute_close(&step.params),
            TksCommand::Click => executor.execute_click(&step.params).await,
            TksCommand::Press => executor.execute_press(&step.params).await,
            TksCommand::Swipe => executor.execute_swipe(&step.params).await,
            TksCommand::DirectionalSwipe => executor.execute_directional_swipe(&step.params).await,
            TksCommand::Input => executor.execute_input(&step.params).await,
            TksCommand::Clear => executor.execute_clear(&step.params).await,
            TksCommand::HideKeyboard => executor.execute_hide_keyboard(),
            TksCommand::Back => executor.execute_back(),
            TksCommand::Wait => executor.execute_wait(&step.params).await,
            TksCommand::Assert => executor.execute_assert(&step.params).await,
        }
    }

    /// 重新加载 locator 定义
    pub fn reload_locators(&mut self) -> Result<()> {
        self.recognizer.reload_locators()
    }
}
