// Runner 命令处理器

use tke::{Result, Runner, JsonOutput};
use std::path::PathBuf;

/// Runner 命令枚举
#[derive(clap::Subcommand)]
pub enum RunCommands {
    /// 执行单个 .tks 脚本文件（返回JSON格式）
    Script {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 执行项目中所有 .tks 脚本文件（返回JSON格式）
    Project,
    /// 执行单行脚本指令（返回JSON格式）
    Step {
        /// 单行脚本指令内容（例如: "点击 [{100, 200}]"）
        line: String,
    },
}

/// 处理 Runner 相关命令
pub async fn handle(action: RunCommands, project_path: PathBuf, device_id: Option<String>) -> Result<()> {
    let mut runner = Runner::new(project_path.clone(), device_id.clone());

    match action {
        RunCommands::Script { script_path } => {
            // 执行单个脚本文件，返回 JSON 格式
            let result = runner.run_script_file(&script_path).await?;

            // 输出 JSON 格式的执行结果
            JsonOutput::print(serde_json::json!({
                "success": result.success,
                "case_id": result.case_id,
                "script_name": result.script_name,
                "start_time": result.start_time,
                "end_time": result.end_time,
                "error": result.error,
                "steps": result.steps.iter().map(|step| serde_json::json!({
                    "index": step.index,
                    "command": step.command,
                    "success": step.success,
                    "error": step.error,
                    "duration_ms": step.duration_ms
                })).collect::<Vec<_>>()
            }));
        }
        RunCommands::Project => {
            // 执行项目中所有脚本，返回 JSON 格式
            let results = runner.run_project_scripts().await?;

            // 输出 JSON 格式的项目执行结果
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
        }
        RunCommands::Step { line } => {
            // 执行单行脚本指令，返回 JSON 结果
            let result = runner.run_single_step(&line).await;

            // 输出 JSON 格式的单步执行结果
            match result {
                Ok(step_result) => {
                    // 检查 step_result.success 字段来判断实际执行结果
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
        }
    }

    Ok(())
}
