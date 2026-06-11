// Run 命令处理器（③ 工作流）
// tke run <path>  按扩展名分发: .tks=单脚本 / .toml=flow(多脚本顺序执行)
// --log <dir> 时保存完整产物，否则只输出 NDJSON 事件流

use tke::{Result, ScriptRunner, FlowRunner, JsonOutput};
use std::path::PathBuf;

use super::emit;

/// Run 命令参数
#[derive(clap::Args)]
pub struct RunArgs {
    /// 执行的文件路径: .tks 单脚本 / .toml flow
    pub path: PathBuf,
}

/// 处理 Run 命令
pub async fn handle(
    run_args: RunArgs,
    device_id: Option<String>,
    element: Option<PathBuf>,
    log: Option<PathBuf>,
) -> Result<()> {
    let path = run_args.path;

    match path.extension().and_then(|s| s.to_str()) {
        Some("tks") => {
            tke::workflow::script_runner::validate_script_path(&path)
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            let runner = ScriptRunner::new(device_id, element);
            let result = runner
                .run(&path, log.as_deref(), &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            // 退出码反映执行结果（事件流中已包含完整信息）
            std::process::exit(if result.success { 0 } else { 1 });
        }
        Some("toml") => {
            if !path.exists() {
                JsonOutput::error(format!("flow 文件不存在: {}", path.display()));
            }

            let runner = FlowRunner::new(device_id, element);
            let result = runner
                .run(&path, log.as_deref(), &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            std::process::exit(if result.success { 0 } else { 1 });
        }
        _ => JsonOutput::error(format!(
            "无法识别的文件类型: {} (支持 .tks 单脚本 / .toml flow)",
            path.display()
        )),
    }
}
