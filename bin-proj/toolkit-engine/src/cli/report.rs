// Report 命令：把一次检查里散落的多个批次汇总成一份全流程报告
//
// 为什么需要：AI 做一次检查要调很多次 `tke steps`（看页面→操作→再看→再操作），每次留下
// 一个独立目录和独立报告。人要审核时面对十几份碎报告——**没法读**。
// 正常情况下 `steps` 每批跑完会自动重建总报告，这条命令用于：
//   - 跨设备检查（证据分在 `web/` 与 `phone/` 子目录，要在更上层汇总）
//   - `--embed` 出一份可以单独发给别人的单文件

use std::path::PathBuf;

use tke::{JsonOutput, Result};

/// Report 命令参数
#[derive(clap::Args)]
pub struct ReportArgs {
    /// 检查目录（含 steps_*/ 的那层，如 ~/.tke/logs/<任务简称>/）
    pub dir: PathBuf,

    /// 把截图内嵌进 HTML，产出可单独发送的单文件（默认走相对链接，快且小）
    #[arg(long)]
    pub embed: bool,
}

pub async fn handle(args: ReportArgs) -> Result<()> {
    if !args.dir.is_dir() {
        JsonOutput::error(format!("目录不存在: {}", args.dir.display()));
    }
    match tke::workflow::report::write_session_report(&args.dir, args.embed) {
        Ok(p) => {
            println!("{}", p.display());
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
