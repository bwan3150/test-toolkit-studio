// Script 工作流 - 执行单个 .tks 脚本
// 逐行执行：实时回调 step_start/step_end 事件（NDJSON 输出由 handler 负责）
// 指定 --log 时每步保存标注截图 + 页面结构文件，结束后写入完整 log.json
// 工作区使用系统缓存临时目录，运行结束即删除；web 会话由脚本的 关闭 指令控制

use crate::{Result, TkeError, ExecutionResult, StepResult, ScriptParser, ScriptInterpreter, Params};
use crate::utils::Workarea;
use super::{RunEvent, RunArtifacts};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 脚本运行器（持有参数表，device/element 查表取得）
pub struct ScriptRunner {
    params: Arc<Params>,
}

impl ScriptRunner {
    pub fn new(params: Arc<Params>) -> Self {
        Self { params }
    }

    /// 执行脚本文件
    ///
    /// - log_root: 产物根目录；None 时不保存任何产物（纯执行 + 事件流）
    /// - on_event: 实时事件回调（逐行输出）
    pub async fn run(
        &self,
        script_path: &Path,
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        // 解析脚本文件
        let parser = ScriptParser::new();
        let script = parser.parse_file(&script_path.to_path_buf())?;

        let script_stem = script_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("script")
            .to_string();
        let display_path = script_path.to_string_lossy().to_string();

        self.run_script(script, &display_path, &script_stem, log_root, on_event).await
    }

    /// 不落文件执行一串 .tks 指令（tke steps）
    pub async fn run_lines(
        &self,
        lines: &[String],
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        // 拼成最小脚本交给解析器
        let content = format!("步骤:\n{}", lines.join("\n"));
        let parser = ScriptParser::new();
        let script = parser.parse(&content)?;

        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("没有可执行的有效指令".to_string()));
        }

        self.run_script(script, "<steps>", "steps", log_root, on_event).await
    }

    /// 内部统一执行逻辑
    async fn run_script(
        &self,
        script: crate::TksScript,
        display_path: &str,
        script_stem: &str,
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {

        // 产物目录（仅 --log 时创建）
        let artifacts = match log_root {
            Some(root) => Some(RunArtifacts::create(root, script_stem)?),
            None => None,
        };
        let run_dir_str = artifacts
            .as_ref()
            .map(|a| a.run_dir.to_string_lossy().to_string())
            .unwrap_or_default();

        // 3. 临时工作区 + 解释器
        let workarea = Workarea::temp_for_run()?;
        let element = self.params.element_lib();
        let mut interpreter = ScriptInterpreter::new(
            self.params.device(),
            element.as_deref(),
            workarea.clone(),
        )?;

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
            script_path: Some(display_path.to_string()),
            run_dir: artifacts.as_ref().map(|_| run_dir_str.clone()),
            launched_packages: Vec::new(),
        };

        on_event(&RunEvent::RunStart {
            script: display_path.to_string(),
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
            // 每步硬超时：元素反复重试/页面无响应时不再无限卡（实测能卡几分钟），
            // 超时即判该步失败——回放上层据此停止或交由 AI 介入修复。
            const STEP_TIMEOUT_SECS: u64 = 45;
            let exec_result = match tokio::time::timeout(
                std::time::Duration::from_secs(STEP_TIMEOUT_SECS),
                interpreter.interpret_step(step),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => Err(TkeError::DeviceError(format!(
                    "执行超时（>{}s）：元素反复重试或页面无响应",
                    STEP_TIMEOUT_SECS
                ))),
            };
            let duration_ms = step_start.elapsed().as_millis() as u64;

            // settle：执行后给页面切换/渲染留时间，避免下一步在动画中/旧页面上执行
            // （元素类步骤的解析还有隐式等待兜底；这里主要稳住纯坐标/滑动后的渲染）
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let (success, error) = match exec_result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            };

            // 保存本步产物（仅 --log 时）
            let (screenshot, xml) = if let Some(artifacts) = &artifacts {
                // 本步执行中若未采集过页面状态（如纯坐标点击/等待/返回），补采一次，
                // 保证每一步都留有截图和结构文件
                if !interpreter.last_trace.captured {
                    let _ = interpreter.capture_state().await;
                }
                artifacts.save_step(
                    &workarea,
                    index,
                    &interpreter.last_trace,
                    &step.raw,
                    success,
                )
            } else {
                (None, None)
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
        result.launched_packages = interpreter.launched_packages.clone();

        // 5. 写入完整运行日志（仅 --log 时）
        let log_path = match &artifacts {
            Some(a) => a.write_log(&result)?.to_string_lossy().to_string(),
            None => String::new(),
        };

        // 6. 清理临时工作区
        // 注: web 会话不自动销毁——生命周期由脚本的 `关闭` 指令控制,
        //     不写则保留会话供后续脚本/指令复用; 新建会话时会收割历史孤儿
        workarea.cleanup();

        on_event(&RunEvent::RunEnd {
            success: result.success,
            total_steps: result.steps.len(),
            successful_steps: result.steps.iter().filter(|s| s.success).count(),
            error: result.error.clone(),
            run_dir: run_dir_str,
            log: log_path,
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
