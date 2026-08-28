// `tke sandbox` —— 源码沙盒的自查入口（ADR-0025 P1）。
//
// **为什么要有它**：P1 的判据是"探索轮数 / token 有没有降"，而降幅要能量。
// 量之前得先能看见 `changed_surfaces` 到底会给 AI 什么 ——
// 一条命令就能看，不必先起一次真实探索。
//
// 它也是 harness 之外的第二个用途：人自己想知道"这个分支改了哪些界面"。

use std::path::PathBuf;

use clap::{Args, Subcommand};

use tke::{JsonOutput, Result};

#[derive(Subcommand)]
pub enum SandboxCommands {
    /// 这次改动碰了哪些界面（按改动规模排序）
    Surfaces(SurfacesArgs),
}

#[derive(Args)]
pub struct SurfacesArgs {
    /// 源码工作树（已经 checkout 到待测分支）。默认当前目录
    #[arg(long)]
    pub tree: Option<PathBuf>,
    /// 对照基线（主干分支名或 sha）
    #[arg(long)]
    pub base: String,
    /// 也把相关文件列出来（默认只给界面名，那正是 AI 看到的）
    #[arg(long)]
    pub files: bool,
}

pub async fn handle(cmd: SandboxCommands, json: bool) -> Result<()> {
    match cmd {
        SandboxCommands::Surfaces(a) => {
            let tree = a.tree.unwrap_or_else(|| PathBuf::from("."));
            let list = tke::sandbox::changed_surfaces(&tree, &a.base)?;
            if json {
                println!("{}", serde_json::to_string(&list)?);
                return Ok(());
            }
            if list.is_empty() {
                // 空不是错 —— 可能这条分支就没改界面，也可能 base 不存在
                println!("没有识别出界面变更（基线 {}）", a.base);
                return Ok(());
            }
            for s in &list {
                println!("{:<28} {:<16} {:>6} 行", s.name, s.kind, s.churn);
                if a.files {
                    for f in &s.files {
                        println!("    {f}");
                    }
                }
            }
            Ok(())
        }
    }
}

// JsonOutput 在 handle 里没直接用到（surfaces 自己序列化），保留导入会告警
const _: fn() = || {
    let _ = std::marker::PhantomData::<JsonOutput>;
};
