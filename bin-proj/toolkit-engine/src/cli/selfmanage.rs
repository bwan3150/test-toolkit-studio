// 【tke update / tke uninstall】把官方安装脚本包一层，人不必记那串 URL、也不必记参数。
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
//
// 参数刻意保持**极少**：`tke update` 零参数、`tke uninstall` 只有一个 --all。
// 「更新哪一套依赖」自动按**已经装了什么**推断——用户当初装的时候已经选过一次了，
// 没道理每次更新再问一遍；「卸载前想看会删什么」由确认提示直接列出来，
// 不该为此单开一个 --dry-run 模式（`curl | bash` 时 stdin 被管道占着，
// 脚本自己问不了，所以确认必须留在这一层）。

use std::path::{Path, PathBuf};
use std::process::Command;

use tke::utils::deps;
use tke::{JsonOutput, Result, TkeError};

const DEFAULT_BASE_URL: &str = "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

#[derive(clap::Args)]
pub struct UpdateArgs {
    /// 分发源地址（自建源/内网镜像才需要；也可用环境变量 TKE_BASE_URL）
    #[arg(long, hide = true)]
    pub base_url: Option<String>,

    /// 更新哪一套依赖（默认按已装的自动推断，一般不用管）
    #[arg(long, hide = true, value_parser = ["web", "android", "ios", "all"])]
    pub profile: Option<String>,
}

#[derive(clap::Args)]
pub struct UninstallArgs {
    /// 连日志与 Chrome 一起删（默认都保留：日志是你跑过的证据，Chrome 有几百 MB）
    #[arg(long)]
    pub all: bool,

    /// 不询问直接卸载（脚本/CI 用）
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// 只删日志 / 只删 Chrome（细分场景，一般用 --all 就够）
    #[arg(long, hide = true)]
    pub logs: bool,
    #[arg(long, hide = true)]
    pub chrome: bool,

    #[arg(long, hide = true)]
    pub base_url: Option<String>,
}

pub async fn update(args: UpdateArgs) -> Result<()> {
    let base = base_url(args.base_url);
    // 装的时候已经选过一次 profile 了，更新时按**现场装了什么**推断，别再问人
    let profile = args.profile.unwrap_or_else(|| installed_profile());
    let extra = vec![
        "--profile".to_string(),
        profile,
        // 让脚本装回同一个分发源（自建源/内网镜像的人不该被打回官方源）
        "--base-url".to_string(),
        base.clone(),
    ];
    run_official_script(&base, Script::Install, &extra)
}

pub async fn uninstall(args: UninstallArgs) -> Result<()> {
    let base = base_url(args.base_url);
    let del_logs = args.all || args.logs;
    let del_chrome = args.all || args.chrome;

    if !args.yes {
        // 把"会删什么、留什么"直接摆出来——这就是 --dry-run 想解决的问题，
        // 与其让人先跑一遍预览、再跑一遍真删，不如在唯一那次确认里说清楚
        println!("将删除：");
        if let Some(d) = tke::utils::update::skill_dir() {
            println!("  skill      {}", d.display());
        }
        println!("  tke 与驱动  {}", tke_home().display());
        // WebDriverAgent 跟 tke 一起装的（install.sh 的 3.5 段），就跟 tke 一起删。
        // 只在**装了**的时候提——没装的东西列出来是噪音
        if std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".tke/wda/WebDriverAgentRunner-Runner.app"))
            .is_some_and(|p| p.exists())
        {
            println!("  WebDriverAgent（iOS 模拟器用）");
        }
        println!("  PATH 里 tke 的那一行");
        if del_logs {
            println!("  日志        {}", logs_dir().display());
        }
        if del_chrome {
            println!("  Chrome for Testing");
        }
        let kept: Vec<&str> = [(!del_logs).then_some("日志"), (!del_chrome).then_some("Chrome")]
            .into_iter()
            .flatten()
            .collect();
        if !kept.is_empty() {
            println!("保留：{}（要一并删除：--all）", kept.join(" · "));
        }

        eprint!("\n继续？[y/N] ");
        use std::io::Write;
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        // 等输入期间 Ctrl+C 立即退出，不然它要等到用户敲回车才生效
        let _g = tke::utils::interrupt::prompting();
        if std::io::stdin().read_line(&mut line).is_err()
            || !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
        {
            println!("已取消");
            return Ok(());
        }
    }

    let mut extra = Vec::new();
    if del_logs && del_chrome {
        extra.push("--all".to_string());
    } else {
        if del_logs {
            extra.push("--logs".to_string());
        }
        if del_chrome {
            extra.push("--chrome".to_string());
        }
    }
    run_official_script(&base, Script::Uninstall, &extra)
}

/// 安装器的落点。**必须与 install.sh / uninstall.sh 同一口径**（`$TKE_HOME` 或 `~/.tke/bin`），
/// 不能用"当前 tke 在哪"——从别处（比如源码构建目录）跑 tke 时，那两个值不一样，
/// 会出现"确认提示说要删 A、脚本实际删了 B"。
fn tke_home() -> PathBuf {
    if let Some(h) = std::env::var_os("TKE_HOME") {
        return PathBuf::from(h);
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".tke").join("bin"))
        .unwrap_or_else(|| PathBuf::from("~/.tke/bin"))
}

fn logs_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".tke").join("logs"))
        .unwrap_or_else(|| PathBuf::from("~/.tke/logs"))
}

/// 按**现在装了哪些驱动**推断该更新哪一套。
/// 只装了 adb 的人不该因为一次 update 就被拖 600MB 的 Chrome 下来。
/// 装了多套（或一套都没有）时才用 all。
fn installed_profile() -> String {
    let dir = tke_home();
    let web = deps::present_in(&dir, "chromedriver");
    let android = deps::present_in(&dir, "adb");
    let ios = deps::present_in(&dir, "go-ios");
    match (web, android, ios) {
        (true, false, false) => "web",
        (false, true, false) => "android",
        (false, false, true) => "ios",
        _ => "all",
    }
    .to_string()
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
fn exec_script(path: &Path, extra: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let err = Command::new("bash").arg(path).args(extra).exec();
    // exec 成功就永远不会走到这里
    Err(TkeError::InvalidArgument(format!("无法执行安装脚本：{}", err)))
}

/// Windows：没有 exec 语义，只能起子进程等它跑完。
/// 此刻 tke.exe 仍在运行、被系统锁着，所以 install.ps1 备了"删不掉就改名"的兜底
/// （Windows 允许重命名运行中的文件，改开后原位就空出来了）。
#[cfg(not(unix))]
fn exec_script(path: &Path, extra: &[String]) -> Result<()> {
    let st = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(path)
        .args(extra)
        .status()
        .map_err(|e| TkeError::InvalidArgument(format!("powershell 起不来：{}", e)))?;
    let _ = std::fs::remove_file(path);
    std::process::exit(st.code().unwrap_or(1));
}
