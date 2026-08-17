// 【tke update / tke uninstall】把官方安装脚本包一层，人不必记那串 URL。
//
// 不自己实现升级/卸载逻辑——**就是去跑 install.sh / uninstall.sh**。那两个脚本已经踩平了
// 一堆坑（文件头校验、CDN 缓存键、先删后拷、PATH 写入、Windows 的 .exe 扩展名…），
// 在 Rust 里再实现一遍只会多一套要维护、且必然分叉的路径。
//
// **Unix 用 exec 把自己替换掉**（用户原话："执行这个 curl 指令然后立刻放手，让 sh 脚本
// 来替换自己"）。exec 之后 tke 进程就此消失、bash 接管同一个 PID：
//   - 输出、Ctrl+C、退出码全部照常，不是后台任务那种输出乱飞
//   - tke 自己的可执行文件不再被任何进程占用，脚本可以随便覆盖
// Windows 没有 exec 语义，只能 spawn 等待；它锁着正在运行的 tke.exe，
// 所以 install.ps1 里备了"删不掉就改名"的兜底（Windows 允许重命名运行中的文件）。
//
// 安全：**不用 `curl … | bash`**。分发平台对不存在的路径回落 200 + 一段 HTML（P-19），
// 管道执行会把网页喂给 bash。这里先落地、验文件头，再执行。

use std::path::PathBuf;
use std::process::Command;

use tke::{JsonOutput, Result, TkeError};

const DEFAULT_BASE_URL: &str = "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

#[derive(clap::Args)]
pub struct UpdateArgs {
    /// 更新哪一套依赖：web / android / ios / all（默认 all，与安装时一致）
    #[arg(long, default_value = "all", value_parser = ["web", "android", "ios", "all"])]
    pub profile: String,

    /// 分发源地址（默认官方源；也可用环境变量 TKE_BASE_URL）
    #[arg(long)]
    pub base_url: Option<String>,
}

#[derive(clap::Args)]
pub struct UninstallArgs {
    /// 连检查日志一起删（默认保留 ~/.tke/logs）
    #[arg(long)]
    pub logs: bool,

    /// 连 Chrome for Testing 一起删（默认保留，几百 MB 重下很慢）
    #[arg(long)]
    pub chrome: bool,

    /// 全删：等于 --logs --chrome
    #[arg(long)]
    pub all: bool,

    /// 只看会删什么，不真删
    #[arg(long)]
    pub dry_run: bool,

    /// 不询问直接卸载
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// 分发源地址（默认官方源）
    #[arg(long)]
    pub base_url: Option<String>,
}

pub async fn update(args: UpdateArgs) -> Result<()> {
    let base = base_url(args.base_url);
    let mut extra = vec!["--profile".to_string(), args.profile.clone()];
    // 让脚本装回同一个分发源（自建源/内网镜像的人不该被打回官方源）
    extra.push("--base-url".to_string());
    extra.push(base.clone());
    run_official_script(&base, Script::Install, &extra)
}

pub async fn uninstall(args: UninstallArgs) -> Result<()> {
    let base = base_url(args.base_url);

    if !args.yes && !args.dry_run {
        let kept = if args.all || (args.logs && args.chrome) {
            "全部删除（含日志与 Chrome）"
        } else if args.logs {
            "删除日志，保留 Chrome"
        } else if args.chrome {
            "删除 Chrome，保留日志"
        } else {
            "保留日志与 Chrome"
        };
        eprint!("将卸载 tke 与 skill（{}）。继续？[y/N] ", kept);
        use std::io::Write;
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err()
            || !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
        {
            println!("已取消");
            return Ok(());
        }
    }

    let mut extra = Vec::new();
    if args.all {
        extra.push("--all".to_string());
    } else {
        if args.logs {
            extra.push("--logs".to_string());
        }
        if args.chrome {
            extra.push("--chrome".to_string());
        }
    }
    if args.dry_run {
        extra.push("--dry-run".to_string());
    }
    run_official_script(&base, Script::Uninstall, &extra)
}

#[derive(Clone, Copy)]
enum Script {
    Install,
    Uninstall,
}

impl Script {
    /// 分发源上的文件名。Windows 走 PowerShell 版——那边跑不了 bash
    fn remote_name(self) -> &'static str {
        match (self, cfg!(windows)) {
            (Script::Install, false) => "install.sh",
            (Script::Install, true) => "install.ps1",
            (Script::Uninstall, false) => "uninstall.sh",
            (Script::Uninstall, true) => "uninstall.ps1",
        }
    }

    /// 文件头特征：bash 脚本以 `#!` 开头，PowerShell 版以块注释 `<#` 开头。
    /// **必须验**——分发平台对不存在的路径回落 200 + 一段 HTML（P-19），
    /// 不验就会把 `<!DOCTYPE html>` 交给 bash 执行
    fn magic(self) -> &'static [&'static str] {
        if cfg!(windows) { &["<#"] } else { &["#!"] }
    }
}

fn base_url(cli: Option<String>) -> String {
    cli.or_else(|| std::env::var("TKE_BASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

/// 下载官方脚本 → 验文件头 → 交给它接管
fn run_official_script(base: &str, which: Script, extra: &[String]) -> Result<()> {
    let name = which.remote_name();
    // 带随机参数破 CDN 缓存：Cloudflare 缓存 4h 且不认 no-cache 请求头（P-19）
    let url = format!("{}/{}?t={}", base, name, std::process::id());

    let tmp = std::env::temp_dir().join(format!("tke-{}-{}", name, std::process::id()));
    let out = Command::new("curl")
        .args(["-fsSL", "--retry", "2", "--max-time", "120", &url, "-o"])
        .arg(&tmp)
        .status()
        .map_err(|e| TkeError::InvalidArgument(format!("curl 起不来（没装？）：{}", e)))?;
    if !out.success() {
        let _ = std::fs::remove_file(&tmp);
        JsonOutput::error(format!("取不到 {}：{}", name, url));
    }

    let head = std::fs::read_to_string(&tmp).unwrap_or_default();
    let head_ok = which.magic().iter().any(|m| head.trim_start().starts_with(m));
    if !head_ok {
        let _ = std::fs::remove_file(&tmp);
        // 拿到的多半是 SPA 兜底页而不是脚本——**绝不能执行**
        JsonOutput::error(format!(
            "{} 下载到的不是脚本（多半是这个路径还没上传，分发源回了网页）：{}",
            name, url
        ));
    }

    exec_script(&tmp, extra)
}

/// Unix：**exec 替换本进程**——tke 就此消失，脚本接管同一个 PID 与前台。
/// 这正是"立刻放手"：脚本要覆盖 tke 自己的二进制时，没有任何进程还占着它。
#[cfg(unix)]
fn exec_script(path: &PathBuf, extra: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let err = Command::new("bash").arg(path).args(extra).exec();
    // exec 成功就永远不会走到这里
    Err(TkeError::InvalidArgument(format!("无法执行安装脚本：{}", err)))
}

/// Windows：没有 exec 语义，只能起子进程等它跑完。
/// 此刻 tke.exe 仍在运行、被系统锁着，所以 install.ps1 备了"删不掉就改名"的兜底
/// （Windows 允许重命名运行中的文件，改开后原位就空出来了）。
#[cfg(not(unix))]
fn exec_script(path: &PathBuf, extra: &[String]) -> Result<()> {
    let st = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(path)
        .args(extra)
        .status()
        .map_err(|e| TkeError::InvalidArgument(format!("powershell 起不来：{}", e)))?;
    let _ = std::fs::remove_file(path);
    std::process::exit(st.code().unwrap_or(1));
}
