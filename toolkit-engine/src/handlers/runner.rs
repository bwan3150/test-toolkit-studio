// Runner 命令处理器

use tke::{Result, Runner, JsonOutput, ScriptInterpreter};
use std::path::PathBuf;

/// Runner 命令枚举
#[derive(clap::Subcommand)]
pub enum RunCommands {
    /// 运行单个脚本文件
    Script {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 运行项目中所有脚本
    Project,
    /// 运行指定内容的脚本 (Toolkit Studio专用，返回JSON)
    Content {
        /// 脚本内容
        content: String,
    },
    /// 执行单个步骤 (Toolkit Studio专用)
    Step {
        /// 脚本内容
        content: String,
        /// 要执行的步骤索引 (0开始)
        step_index: usize,
    },
}

/// 处理 Runner 相关命令
pub async fn handle(action: RunCommands, project_path: PathBuf, device_id: Option<String>) -> Result<()> {
    let mut runner = Runner::new(project_path.clone(), device_id.clone());

    match action {
        RunCommands::Script { script_path } => {
            println!("开始执行脚本: {:?}", script_path);
            let result = runner.run_script_file(&script_path).await?;

            println!("\n执行结果:");
            println!("  状态: {}", if result.success { "成功 ✓" } else { "失败 ✗" });
            println!("  用例ID: {}", result.case_id);
            println!("  脚本名: {}", result.script_name);
            println!("  开始时间: {}", result.start_time);
            println!("  结束时间: {}", result.end_time);
            println!("  总步骤数: {}", result.steps.len());

            let successful_steps = result.steps.iter().filter(|s| s.success).count();
            println!("  成功步骤: {}", successful_steps);

            if let Some(ref error) = result.error {
                println!("  错误信息: {}", error);
            }

            // 显示失败的步骤
            let failed_steps: Vec<_> = result.steps.iter().filter(|s| !s.success).collect();
            if !failed_steps.is_empty() {
                println!("\n失败的步骤:");
                for step in failed_steps {
                    println!("  步骤{}: {}", step.index + 1, step.command);
                    if let Some(ref error) = step.error {
                        println!("    错误: {}", error);
                    }
                }
            }
        }
        RunCommands::Project => {
            println!("开始执行项目中的所有脚本...");
            let results = runner.run_project_scripts().await?;

            println!("\n项目执行结果:");
            println!("  总脚本数: {}", results.len());

            let successful_scripts = results.iter().filter(|r| r.success).count();
            println!("  成功脚本: {}", successful_scripts);
            println!("  失败脚本: {}", results.len() - successful_scripts);

            // 显示各个脚本的结果
            for result in results {
                let status = if result.success { "✓" } else { "✗" };
                println!("  {} {} ({})", status, result.script_name, result.case_id);
                if let Some(ref error) = result.error {
                    println!("      错误: {}", error);
                }
            }
        }
        RunCommands::Content { content } => {
            // 为 Toolkit Studio 专用：返回 JSON 格式的结果
            let result = runner.run_script_content(&content).await?;

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
        RunCommands::Step { content, step_index } => {
            // 为 Toolkit Studio 专用：执行单个步骤并返回 JSON 结果
            let script = runner.parser.parse(&content)?;

            if step_index >= script.steps.len() {
                JsonOutput::print(serde_json::json!({
                    "success": false,
                    "error": format!("步骤索引超出范围: {} >= {}", step_index, script.steps.len()),
                    "step_index": step_index
                }));
                return Ok(());
            }

            let step = &script.steps[step_index];

            // 初始化解释器
            let mut interpreter = ScriptInterpreter::new(
                project_path.clone(),
                device_id
            )?;

            let start_time = std::time::Instant::now();

            // 执行单个步骤
            let step_result = match interpreter.interpret_step(step).await {
                Ok(()) => serde_json::json!({
                    "success": true,
                    "step_index": step_index,
                    "command": step.raw,
                    "line_number": step.line_number,
                    "duration_ms": start_time.elapsed().as_millis(),
                    "error": null
                }),
                Err(e) => serde_json::json!({
                    "success": false,
                    "step_index": step_index,
                    "command": step.raw,
                    "line_number": step.line_number,
                    "duration_ms": start_time.elapsed().as_millis(),
                    "error": e.to_string()
                })
            };

            JsonOutput::print(step_result);
        }
    }

    Ok(())
}
