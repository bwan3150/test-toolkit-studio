// Steps 命令处理器（③ 工作流）
// tke steps "<指令1>" "<指令2>" ... 不落文件执行一串 .tks 指令
// 输出与 run 一致的 NDJSON 事件流；--log 时同样保存完整产物

use tke::{Result, ScriptRunner, JsonOutput};
use std::sync::Arc;

use super::EventPrinter;

/// Steps 命令参数
#[derive(clap::Args)]
pub struct StepsArgs {
    /// 依次执行的 .tks 指令（可多条，如: "点击 [{登录按钮}]" "等待 [2s]"）
    #[arg(required = true)]
    pub lines: Vec<String>,
}

/// 处理 Steps 命令
pub async fn handle(
    args: StepsArgs,
    params: Arc<tke::Params>,
) -> Result<()> {
    let mut printer = EventPrinter::auto(params.json);
    let mut emit = move |e: &tke::RunEvent| printer.print(e);
    let runner = ScriptRunner::new(params.clone());
    let result = runner
        .run_lines(&args.lines, params.log.as_deref(), &mut emit)
        .await
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    std::process::exit(if result.success { 0 } else { 1 });
}
