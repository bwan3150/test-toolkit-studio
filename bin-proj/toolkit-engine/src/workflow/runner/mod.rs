// Runner模块 - 脚本解析与单步执行
// 完整脚本/flow 的执行在 workflow 模块（带实时事件和产物）

// 子模块
mod parser;
mod interpreter;

// 导出
pub use parser::{ScriptParser, script_to_source, step_to_source};
pub use interpreter::{ScriptInterpreter, ActionTrace};

use crate::{Result, TkeError, StepResult};
use crate::utils::Workarea;
use std::path::Path;
use std::time::Instant;

/// 单步执行器（编辑器调试用）
pub struct Runner {
    device_id: Option<String>,
    pub parser: ScriptParser,
}

impl Runner {
    pub fn new(device_id: Option<String>) -> Self {
        Self {
            device_id,
            parser: ScriptParser::new(),
        }
    }

    /// 运行单行脚本指令
    /// element_path: 元素库路径（None 按默认路径查找）
    pub async fn run_single_step(
        &mut self,
        line: &str,
        element_path: Option<&Path>,
    ) -> Result<StepResult> {
        // 构造一个最小的脚本来解析单行指令
        let minimal_script = format!("用例: 单步执行\n脚本名: 单步\n\n步骤:\n{}", line);

        // 解析脚本
        let script = self.parser.parse(&minimal_script)?;

        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("无效的脚本指令".to_string()));
        }

        // 单步执行使用设备缓存工作区（与 fetch/recognize 共享）
        let workarea = Workarea::for_device(self.device_id.as_deref())?;

        // 初始化解释器
        let mut interpreter = ScriptInterpreter::new(
            self.device_id.clone(),
            element_path,
            workarea,
        )?;

        let step = &script.steps[0];
        let start_time = Instant::now();

        // 执行单个步骤
        match interpreter.interpret_step(step).await {
            Ok(()) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: true,
                error: None,
                duration_ms: start_time.elapsed().as_millis() as u64,
                line: None,
                screenshot: None,
                xml: None,
            }),
            Err(e) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: false,
                error: Some(e.to_string()),
                duration_ms: start_time.elapsed().as_millis() as u64,
                line: None,
                screenshot: None,
                xml: None,
            })
        }
    }
}
