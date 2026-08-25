//! `tke task` —— 起一个测试会话（ui / security 共享的生命周期层，ADR-0021）。
//!
//! 做的事很轻：建任务目录 + 写 `task.json` 标记（kind/target/mode）。之后各轨的命令
//! （`tke http`/`recon`/`steps`/`control`）都 `--log <这个目录>` 把证据往里放，
//! 最后 `tke report <这个目录>` 按标记自动分派出对应报告。

use std::path::PathBuf;
use std::sync::Arc;

use tke::workflow::task::{write_marker, TaskMeta};
use tke::{JsonOutput, Params, Result};

#[derive(clap::Subcommand)]
pub enum TaskCommands {
    /// 新建测试会话：建目录 + 写 task.json 标记
    New {
        /// 测试轨：ui（设备/UI）/ security（安全）
        #[arg(long, value_parser = ["ui", "security"])]
        kind: String,
        /// 目标（URL / 应用 / 说明），可选
        #[arg(long)]
        target: Option<String>,
        /// 强度档（安全轨用：passive/safe/aggressive/red-team），可选
        #[arg(long)]
        mode: Option<String>,
        /// 任务目录（不给则用全局 `--log`，再不给用临时目录）
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

pub async fn handle(cmd: TaskCommands, params: Arc<Params>) -> Result<()> {
    match cmd {
        TaskCommands::New { kind, target, mode, dir } => {
            let dir = dir.or_else(|| params.log.clone()).unwrap_or_else(|| {
                std::env::temp_dir().join(format!("tke-task-{}", std::process::id()))
            });
            let meta = TaskMeta { kind: kind.clone(), target: target.clone(), mode: mode.clone() };
            write_marker(&dir, &meta)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "kind": kind,
                "target": target,
                "mode": mode,
                "dir": dir.to_string_lossy(),
                "hint": "接下来把这个目录当 --log，最后 tke report <dir> 出报告",
            }));
            Ok(())
        }
    }
}
