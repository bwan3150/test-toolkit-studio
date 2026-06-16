// 脚本解释器模块 - 将TKS指令转换为可执行的ADB指令

mod command_executor;
mod param_extractor;
mod target_resolver;

use crate::{Result, TksStep, TksCommand, Controller, Recognizer, Platform, Point, Bounds};
use crate::utils::Workarea;
use std::path::Path;
use tracing::debug;

use command_executor::CommandExecutor;

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
            last_trace: ActionTrace::default(),
            launched_packages: Vec::new(),
        })
    }

    /// 解释并执行单个步骤
    pub async fn interpret_step(&mut self, step: &TksStep) -> Result<()> {
        debug!("执行步骤: {} (行号: {})", step.raw, step.line_number);

        self.last_trace.reset();

        let mut executor = CommandExecutor::new(
            &self.workarea,
            &mut self.controller,
            &self.recognizer,
            &mut self.last_trace,
            &mut self.launched_packages,
        );

        match step.command {
            TksCommand::Launch => executor.execute_launch(&step.params).await,
            TksCommand::Close => executor.execute_close(&step.params).await,
            TksCommand::Click => executor.execute_click(&step.params).await,
            TksCommand::Press => executor.execute_press(&step.params).await,
            TksCommand::Swipe => executor.execute_swipe(&step.params).await,
            TksCommand::DirectionalSwipe => executor.execute_directional_swipe(&step.params).await,
            TksCommand::Input => executor.execute_input(&step.params).await,
            TksCommand::Clear => executor.execute_clear(&step.params).await,
            TksCommand::HideKeyboard => executor.execute_hide_keyboard().await,
            TksCommand::Back => executor.execute_back().await,
            TksCommand::Wait => executor.execute_wait(&step.params).await,
            TksCommand::Assert => executor.execute_assert(&step.params).await,
        }
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
