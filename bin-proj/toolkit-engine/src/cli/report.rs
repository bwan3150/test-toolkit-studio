// Report 命令：重建一份检查报告
//
// 正常情况下 `tke steps` 每批跑完会自动重建报告，这条命令用于：
//   - 手工重建（报告被删了 / 想换图片模式）
//   - `--full-image` 出一份原图版，逐像素复核用
//   - 汇总**旧布局**的碎批次目录（`steps_*/` 那种，含跨设备分 `web/` `phone/` 的）
//
// 两种布局自动识别：目录里有 log.json = 任务布局(新)，否则去子目录里搜集批次(旧)。

use std::path::PathBuf;

use tke::{JsonOutput, Result};

/// Report 命令参数
#[derive(clap::Args)]
pub struct ReportArgs {
    /// 任务目录（`--log` 指的那个，如 ~/.tke/logs/<任务简称>/）
    pub dir: PathBuf,

    /// 用**原图**内嵌（默认压缩内嵌：缩到宽 1280 + JPEG85，体积约 1/4，控件与文字仍清晰）
    #[arg(long)]
    pub full_image: bool,

    /// 仅对**旧布局**（碎批次目录）有效：把截图内嵌成单文件而不是相对链接
    #[arg(long)]
    pub embed: bool,
}

pub async fn handle(args: ReportArgs) -> Result<()> {
    if !args.dir.is_dir() {
        JsonOutput::error(format!("目录不存在: {}", args.dir.display()));
    }
    // 任务布局：目录里直接躺着 log.json
    let r = if args.dir.join("log.json").is_file() {
        tke::workflow::report::write_task_report_with(&args.dir, args.full_image)
    } else {
        tke::workflow::report::write_session_report(&args.dir, args.embed || args.full_image)
    };
    match r {
        Ok(p) => {
            println!("{}", p.display());
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
