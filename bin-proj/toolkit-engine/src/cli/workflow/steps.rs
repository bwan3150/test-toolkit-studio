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

    warn_if_stale(&params);
    std::process::exit(if result.success { 0 } else { 1 });
}

/// 本地这套东西落后于分发源就缀一行提醒（Q-11）。
///
/// 为什么挂在 `steps` 上：调用方 AI 每次操作设备都会走这条命令，**这是唯一能保证被看见的
/// 地方**——指望人或 AI 想起来跑体检，正是上次踩坑的原因（用户抱着两天前的 SKILL.md
/// 重跑，必然得出"没改善"的结论）。
///
/// 三条克制：
///   - 结果**缓存 4 小时**，所以每批都问、每 4 小时才真联网一次（联网时超时只给 5s）
///   - 打到 **stderr**：stdout 是给 Electron 解析的 NDJSON，不能混进人话
///   - `--json` 时闭嘴：那是程序在读
fn warn_if_stale(params: &tke::Params) {
    if params.json {
        return;
    }
    if let Some(hint) = tke::utils::update::check(tke::utils::update::MAX_AGE_SECS)
        .as_ref()
        .and_then(|s| s.hint())
    {
        eprintln!("! {}", hint);
    }
}
