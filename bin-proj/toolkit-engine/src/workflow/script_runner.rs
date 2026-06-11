// Script 工作流 - 执行单个 .tks 脚本
// 逐行执行：实时回调 step_start/step_end 事件（NDJSON 输出由 handler 负责），
// 每步保存标注截图 + UI 结构文件，结束后写入完整 run.json

use crate::{Result, TkeError, ExecutionResult, StepResult, ScriptParser, ScriptInterpreter};
use super::{RunEvent, RunArtifacts};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// 脚本运行器
pub struct ScriptRunner {
    project_path: PathBuf,
    device_id: Option<String>,
}

impl ScriptRunner {
    pub fn new(project_path: PathBuf, device_id: Option<String>) -> Self {
        Self { project_path, device_id }
    }

    /// 执行脚本文件
    ///
    /// - runs_root: 产物根目录（缺省 <project>/runs；flow 传入自己的运行目录）
    /// - on_event: 实时事件回调（逐行输出）
    pub async fn run(
        &self,
        script_path: &Path,
        runs_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        // 1. 解析脚本
        let parser = ScriptParser::new();
        let script = parser.parse_file(&script_path.to_path_buf())?;

        let script_stem = script_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("script");

        // 2. 创建产物目录
        let artifacts = RunArtifacts::create(&self.project_path, runs_root, script_stem)?;
        let run_dir_str = artifacts.run_dir.to_string_lossy().to_string();

        // 3. 初始化解释器
        let mut interpreter =
            ScriptInterpreter::new(self.project_path.clone(), self.device_id.clone())?;

        let start_time = chrono::Local::now().to_rfc3339();

        let mut result = ExecutionResult {
            success: true,
            case_id: script.case_id.clone(),
            script_name: if script.script_name.is_empty() {
                script_stem.to_string()
            } else {
                script.script_name.clone()
            },
            start_time: start_time.clone(),
            end_time: String::new(),
            steps: Vec::new(),
            error: None,
            script_path: Some(script_path.to_string_lossy().to_string()),
            run_dir: Some(run_dir_str.clone()),
        };

        on_event(&RunEvent::RunStart {
            script: script_path.to_string_lossy().to_string(),
            script_name: result.script_name.clone(),
            total_steps: script.steps.len(),
            run_dir: run_dir_str.clone(),
            start_time,
        });

        // 4. 逐步执行
        for (index, step) in script.steps.iter().enumerate() {
            on_event(&RunEvent::StepStart {
                index,
                line: step.line_number,
                command: step.raw.clone(),
            });

            let step_start = Instant::now();
            let exec_result = interpreter.interpret_step(step).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            // 本步执行中若未采集过页面状态（如纯坐标点击/等待/返回），补采一次，
            // 保证每一步都留有截图和结构文件
            if !interpreter.last_trace.captured {
                let _ = interpreter.capture_state().await;
            }

            // 保存本步产物：标注截图 + XML
            let (screenshot, xml) =
                artifacts.save_step(&self.project_path, index, &interpreter.last_trace);

            let (success, error) = match exec_result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            };

            let step_result = StepResult {
                index,
                command: step.raw.clone(),
                success,
                error: error.clone(),
                duration_ms,
                line: Some(step.line_number),
                screenshot: screenshot.clone(),
                xml: xml.clone(),
            };

            on_event(&RunEvent::StepEnd {
                index,
                line: step.line_number,
                command: step.raw.clone(),
                success,
                error: error.clone(),
                duration_ms,
                screenshot,
                xml,
            });

            result.steps.push(step_result);

            // 步骤失败即停止
            if !success {
                result.success = false;
                result.error = error;
                break;
            }

            // 步骤间短暂延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        result.end_time = chrono::Local::now().to_rfc3339();

        // 5. 写入完整运行日志
        let log_path = artifacts.write_log(&result)?;

        on_event(&RunEvent::RunEnd {
            success: result.success,
            total_steps: result.steps.len(),
            successful_steps: result.steps.iter().filter(|s| s.success).count(),
            error: result.error.clone(),
            run_dir: run_dir_str,
            log: log_path.to_string_lossy().to_string(),
        });

        Ok(result)
    }
}

/// 校验脚本文件存在且为 .tks
pub fn validate_script_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(TkeError::InvalidArgument(format!(
            "脚本文件不存在: {}",
            path.display()
        )));
    }
    if path.extension().and_then(|s| s.to_str()) != Some("tks") {
        return Err(TkeError::InvalidArgument(format!(
            "不是 .tks 脚本文件: {}",
            path.display()
        )));
    }
    Ok(())
}
