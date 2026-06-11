// Run 命令处理器（③ 工作流）
// script: 逐行执行 .tks，实时输出 NDJSON 事件 + 完整产物（run.json/标注截图/结构文件）
// flow:   依次执行一组 .tks 脚本
// ai:     AI 探索生成 .tks（透传 tester-ai）
// step:   执行单行指令（编辑器调试用）
// project:(legacy) 执行项目 cases 下所有脚本

use tke::{Result, Runner, ScriptRunner, FlowRunner, RunEvent, ToolManager, JsonOutput};
use std::io::Write;
use std::path::PathBuf;

/// Run 命令枚举
#[derive(clap::Subcommand)]
pub enum RunCommands {
    /// 执行单个 .tks 脚本（逐行实时输出 NDJSON 事件，产物保存到 runs/）
    Script {
        /// 脚本文件路径
        script_path: PathBuf,
        /// 产物根目录（默认 <project>/runs）
        #[arg(long)]
        runs_dir: Option<PathBuf>,
    },
    /// 依次执行 flow 中的一组 .tks 脚本（flow 为 JSON: {"name":..., "scripts":[...]}）
    Flow {
        /// flow 文件路径
        flow_path: PathBuf,
    },
    /// AI 根据用例探索并生成 .tks 脚本（透传 tester-ai，参数原样传递）
    Ai {
        /// 透传给 tester-ai 的参数
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 执行单行脚本指令（例如: "点击 [{100, 200}]"）
    Step {
        /// 单行脚本指令内容
        line: String,
    },
    /// (legacy) 执行项目 cases 目录下所有 .tks 脚本
    Project,
}

/// 输出一行 NDJSON 事件并立即 flush（保证实时性）
fn emit(event: &RunEvent) {
    if let Ok(json) = serde_json::to_string(event) {
        println!("{}", json);
        let _ = std::io::stdout().flush();
    }
}

/// 处理 Run 相关命令
pub async fn handle(action: RunCommands, project_path: PathBuf, device_id: Option<String>) -> Result<()> {
    match action {
        RunCommands::Script { script_path, runs_dir } => {
            tke::workflow::script_runner::validate_script_path(&script_path)
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            let runner = ScriptRunner::new(project_path, device_id);
            let result = runner
                .run(&script_path, runs_dir.as_deref(), &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            // 退出码反映执行结果（事件流中已包含完整信息）
            std::process::exit(if result.success { 0 } else { 1 });
        }
        RunCommands::Flow { flow_path } => {
            if !flow_path.exists() {
                JsonOutput::error(format!("flow 文件不存在: {}", flow_path.display()));
            }

            let runner = FlowRunner::new(project_path, device_id);
            let result = runner
                .run(&flow_path, &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            std::process::exit(if result.success { 0 } else { 1 });
        }
        RunCommands::Ai { args } => {
            // 透传给 tester-ai（与 tke 同目录），AI 工作流由其实现
            ToolManager::passthrough("tester-ai", args, device_id)
        }
        RunCommands::Step { line } => {
            let mut runner = Runner::new(project_path, device_id);
            let result = runner.run_single_step(&line).await;

            match result {
                Ok(step_result) => {
                    JsonOutput::print(serde_json::json!({
                        "success": step_result.success,
                        "command": line,
                        "duration_ms": step_result.duration_ms,
                        "error": step_result.error
                    }));
                }
                Err(e) => {
                    JsonOutput::print(serde_json::json!({
                        "success": false,
                        "command": line,
                        "error": e.to_string()
                    }));
                }
            }
            Ok(())
        }
        RunCommands::Project => {
            let mut runner = Runner::new(project_path, device_id);
            let results = runner.run_project_scripts().await?;

            JsonOutput::print(serde_json::json!({
                "success": true,
                "total_scripts": results.len(),
                "successful_scripts": results.iter().filter(|r| r.success).count(),
                "failed_scripts": results.iter().filter(|r| !r.success).count(),
                "scripts": results.iter().map(|result| serde_json::json!({
                    "success": result.success,
                    "case_id": result.case_id,
                    "script_name": result.script_name,
                    "start_time": result.start_time,
                    "end_time": result.end_time,
                    "error": result.error,
                    "total_steps": result.steps.len(),
                    "successful_steps": result.steps.iter().filter(|s| s.success).count(),
                })).collect::<Vec<_>>()
            }));
            Ok(())
        }
    }
}
