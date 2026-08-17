// 【tke fix】把跑起来所缺的依赖补齐——chromedriver / Chrome for Testing / adb / go-ios。
//
// **下载只在这条命令里发生**（ADR-0012）。普通命令（run/steps/fetch…）缺东西时只报错并
// 指路，绝不自己联网：一条 CLI 命令突然静默拖 600MB，在内网、离线、CI、按流量计费的机器
// 上都是灾难，企业环境还有合规问题。要不要下、什么时候下，是使用者的决定。
//
// 下载走 `curl` 子进程而不是 Rust HTTP 客户端，有意为之：
//   - reqwest 在本项目是 ocr-online 的可选依赖，CI 的 `--no-default-features` 构建里没有；
//     tke fix 必须在**任何**构建下都能用
//   - tke 本来就是"调外部工具"的架构（adb/chromedriver/go-ios 都是子进程），install.sh
//     也依赖 curl，环境要求一致
//
// 两个分发源的坑（P-19，install.sh 踩过一遍，这里同样要防）：
//   - 存储平台对**不存在的路径回落 200 + 一段 HTML**（SPA 兜底），curl 的 -f 拦不住，
//     所以每个文件都要**验文件头**，否则会把网页当二进制装进去
//   - Cloudflare 缓存 4h 且不认 no-cache 请求头，只有**变化的查询参数**能破缓存，
//     所以先取 VERSION 拿 build 戳，再用它当所有下载的键

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use tke::utils::deps;
use tke::{Result, TkeError};

// ── 外观 ──（与 install.sh / install.ps1 同一套：符号 + 颜色，**不用 emoji**——
// 等宽终端里对不齐，SSH/CI 日志里还常变成方块）
fn tty() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}
fn c(code: &str) -> String {
    if tty() { format!("\x1b[{}m", code) } else { String::new() }
}
fn sym_ok() -> String { format!("{}✓{}", c("38;5;42"), c("0")) }
fn sym_warn() -> String { format!("{}!{}", c("38;5;214"), c("0")) }
fn sym_err() -> String { format!("{}✗{}", c("38;5;203"), c("0")) }
fn sym_dot() -> String { format!("{}·{}", c("38;5;245"), c("0")) }
fn dim(s: &str) -> String { format!("{}{}{}", c("38;5;245"), s, c("0")) }
fn section(title: &str) {
    println!("\n{}{}▸ {}{}", c("1"), c("38;5;39"), title, c("0"));
}

const DEFAULT_BASE_URL: &str = "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

/// Doctor 命令参数（`tke fix` 是它的别名，见 main.rs）
#[derive(clap::Args)]
pub struct FixArgs {
    /// 只看这一类：web（浏览器）/ android / ios / all（默认）
    #[arg(long, default_value = "all", value_parser = ["web", "android", "ios", "all"])]
    pub profile: String,

    /// 补齐缺失的依赖（**会联网下载**）。不加就只体检，一个字节都不下
    #[arg(long)]
    pub fix: bool,

    /// 只检查不下载 —— `tke fix --check` 的旧写法，等同于不加 `--fix`
    #[arg(long, hide = true)]
    pub check: bool,

    /// 不询问直接下载（脚本/CI 里用）
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// 分发源地址（默认走官方源；也可用环境变量 TKE_BASE_URL）
    #[arg(long)]
    pub base_url: Option<String>,
}

impl FixArgs {
    /// 这次要不要下载。`tke doctor` 默认只看；`tke fix`（别名）默认下载——
    /// 老命令的语义不能变，用户的脚本和已发布的 install.sh 都还在用它。
    fn wants_download(&self, invoked_as_fix: bool) -> bool {
        if self.check {
            return false; // --check 永远只看（旧写法）
        }
        self.fix || invoked_as_fix
    }
}

/// 一件缺失的依赖
struct Missing {
    /// 分发源上的名字（bin/<platform>/<name>.gz）
    name: &'static str,
    /// 给人看的说明
    what: &'static str,
    /// 大概多大（分发源不一定支持 HEAD，写死一个量级让人有预期）
    size: &'static str,
    /// true=zip 包（Chrome），false=单文件 gz
    is_chrome: bool,
}

pub async fn handle(args: FixArgs) -> Result<()> {
    handle_as(args, false).await
}

/// `invoked_as_fix=true` 表示走的是 `tke fix` 别名——那条命令的旧语义是"默认就下载"
pub async fn handle_as(args: FixArgs, invoked_as_fix: bool) -> Result<()> {
    let download = args.wants_download(invoked_as_fix);
    let base_url = args
        .base_url
        .clone()
        .or_else(|| std::env::var("TKE_BASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let exe_dir = tke_dir()?;
    let platform = platform_tag()?;

    section("DOCTOR");
    println!("  {} {}", dim("平台    "), platform);
    println!("  {} {}", dim("落点    "), exe_dir.display());

    let missing = detect_missing(&exe_dir, &args.profile);
    let wants_ios = args.profile == "ios" || args.profile == "all";
    let ios_blocked = wants_ios && !tke::utils::capability::ios_supported();

    // iOS 门禁要说出来：被挡住时 missing 是空的，光看"依赖已就绪"会以为这台机器什么都能做
    let ios_note = || {
        if ios_blocked {
            println!("  {} {}", dim("iOS     "), dim("不支持 · 需 macOS（设备端 WDA 依赖 Xcode）"));
        } else if wants_ios && !upstream_has_go_ios() {
            println!("  {} {}", dim("iOS     "), dim("不支持 · go-ios 无 32 位 Windows 版"));
        }
    };

    if missing.is_empty() {
        // 依赖状态是**一项检查**，不是结论——结论统一放最后那行（见 print_health 收尾）
        if args.profile == "ios" && (ios_blocked || !upstream_has_go_ios()) {
            println!("  {} {}", dim("依赖    "), dim("无需补齐 · 此机型不支持 iOS"));
        } else {
            println!("  {} {} {}", dim("依赖    "), "已就绪", dim(&format!("· {}", args.profile)));
        }
        ios_note();
        if !download {
            print_health(&exe_dir, 0);
        }
        return Ok(());
    }

    println!("  {} {} {}", dim("依赖    "), format!("缺 {} 项", missing.len()), dim(&format!("· {}", args.profile)));
    for m in &missing {
        println!("    {} {:<16} {}{}", sym_err(), m.name, m.what, dim(&format!("（约 {}）", m.size)));
    }
    ios_note();

    // arm64 Linux：浏览器与安卓那套驱动上游就没有官方包，下了也是白下
    if !upstream_has_drivers() {
        let need_drivers = missing.iter().any(|m| m.is_chrome || m.name != "go-ios");
        if need_drivers {
            println!("⚠️  arm64 Linux 上游没有官方驱动包（Chrome for Testing 与 Google 的");
            println!("    platform-tools 都不出这个架构），分发源自然也没有。请改用发行版自带的：");
            println!("      sudo apt install -y chromium chromium-driver adb");
            println!("    再把 chromium-driver 的 chromedriver 软链到 tke 同目录（tke 只在那儿找）：");
            println!("      ln -sf \"$(command -v chromedriver)\" \"{}/chromedriver\"", exe_dir.display());
            println!("    go-ios 有 arm64 版，`tke fix --profile ios` 照常能补。");
            println!();
        }
    }

    if !download {
        print_health(&exe_dir, missing.len());
        // 只体检不下载：退出码非 0，CI 可以据此判断环境是否就绪
        std::process::exit(1);
    }

    if !args.yes && !confirm("  要现在下载补齐吗？")? {
        println!("  {}", dim("已取消；需要时再跑 tke doctor --fix"));
        return Ok(());
    }

    // 缓存键：先带随机参数取 VERSION（它必须是最新的），再用里面的 build 戳当所有下载的键。
    // 发过新版 → 戳变 → 自然拿到新文件；没发新版 → 戳不变 → 照常命中 CDN。
    let nonce = std::process::id();
    let version = curl_text(&format!("{}/VERSION?t={}", base_url, nonce)).unwrap_or_default();
    let build_key = version
        .lines()
        .find_map(|l| l.strip_prefix("build:"))
        .map(|v| v.trim().to_string());
    let q = match &build_key {
        Some(b) => format!("?b={}", b),
        None => format!("?t={}", nonce), // 老布局没有 build 戳：宁可不走缓存也不装到旧文件
    };

    let tmp = std::env::temp_dir().join(format!("tke-fix-{}", nonce));
    std::fs::create_dir_all(&tmp).map_err(TkeError::IoError)?;

    let mut failed = Vec::new();
    for m in &missing {
        print!("  {} {} ... ", sym_dot(), m.name);
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let r = if m.is_chrome {
            install_chrome(&base_url, &q, &tmp, &platform)
        } else {
            install_bin(&base_url, &q, &tmp, &exe_dir, m.name, &platform)
        };
        match r {
            Ok(()) => println!("{}", sym_ok()),
            Err(e) => {
                println!("{} {}", sym_err(), e);
                failed.push(m.name);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);

    println!();
    // 复验：以"现在还缺不缺"为准，而不是以"下载有没有报错"为准
    let still = detect_missing(&exe_dir, &args.profile);
    if still.is_empty() {
        println!("  {} 补齐了", sym_ok());
        Ok(())
    } else {
        // 失败要可见（INV-9）：不能下完就说好了，得如实说还缺什么
        println!("  {} 还缺 {} 项：", sym_warn(), still.len());
        for m in &still {
            println!("  {} {} —— {}", sym_err(), m.name, m.what);
        }
        if !failed.is_empty() {
            println!();
            println!("   下载失败的多半是分发源上还没有这个平台的文件。");
            println!("   手动装的办法见 skill 的 README，或换 --base-url 指向你自己的源。");
        }
        std::process::exit(1);
    }
}

/// 依赖之外的环境状况——设备连没连、版本跟不跟得上、证据落哪儿、跑有头还是无头。
///
/// 放进 `tke fix --check` 而不是再写一个体检脚本：**一份 Rust 实现三平台通用**。
/// 早先的 `check-env.sh` 是 bash，Windows 用户根本跑不了，而 Windows 恰恰是
/// 「同事跑完 Claude Code 要验一遍」的主力平台。
fn print_health(exe_dir: &Path, missing: usize) {

    // 安卓设备：adb 在才问它，不然徒增一条看不懂的报错
    if deps::present_in(exe_dir, "adb") {
        let adb = exe_dir.join(if cfg!(windows) { "adb.exe" } else { "adb" });
        match Command::new(&adb).arg("devices").output() {
            Ok(o) => {
                let list: Vec<String> = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .skip(1)
                    .filter_map(|l| {
                        let mut it = l.split_whitespace();
                        match (it.next(), it.next()) {
                            (Some(id), Some("device")) => Some(id.to_string()),
                            _ => None,
                        }
                    })
                    .collect();
                if list.is_empty() {
                    println!("  {} {}", dim("设备    "), dim("未连接"));
                } else {
                    println!("  {} {}", dim("设备    "), list.join(" · "));
                }
            }
            Err(e) => println!("  {} {}", dim("设备    "), dim(&format!("adb 不可用：{}", e))),
        }
    }

    // 版本：跟分发源比一下，免得一直用着旧的。取不到就静默跳过（离线/内网照常用）。
    // max_age=0 = 手动跑 doctor 时**强制联网**，不吃缓存——人特地来问，就给他最新的答案。
    // 只报"不一致"，不摆箭头暗示方向——本地可能是刚编出来的、比分发源还新。
    let local = env!("BUILD_VERSION");
    let st = tke::utils::update::check(0);
    match &st {
        None => println!("  {} {} {}", dim("版本    "), local, dim("· 离线，未校验")),
        Some(s) if s.tke_stale => {
            // 只说"有新的"，版本号细节走 dim——人要的是"该不该动手"，不是两串数字对比
            println!(
                "  {} {} {}",
                dim("版本    "),
                "可用更新",
                dim(&format!("· {} → {}", local, s.remote.tke))
            );
        }
        Some(_) => println!("  {} {} {}", dim("版本    "), "已是最新", dim(&format!("· {}", local))),
    }

    // skill 新鲜度：**这条是这次体检最要紧的一行**。tke 二进制版本号只在 bump 时才变，
    // 而 SKILL.md 天天改——只比版本号的话，用户抱着两天前的旧文档，体检照样说"一致"
    // （Q-11 就是这么发生的：改完的四个修复根本没送到用户手上）。
    if let Some(s) = &st {
        match (&s.skill_dir, &s.local_skill_build) {
            (Some(_), Some(_)) if s.skill_stale => {
                println!(
                    "  {} {} {}",
                    dim("skill   "),
                    "可用更新",
                    dim(&format!("· {}", s.remote.build))
                );
            }
            (Some(_), Some(_)) => println!("  {} {}", dim("skill   "), "已是最新"),
            // 老安装器装的没写版本文件——不当成过期（无从判断），但要说清为什么看不出来
            (Some(dir), None) => {
                println!("  {} {}", dim("skill   "), dim("无版本信息 · 由旧安装器安装"));
                println!("    {}", dim(&format!("{}", dir.display())));
            }
            (None, _) => println!("  {} {}", dim("skill   "), dim("未安装")),
        }
    }

    // 日志落点：人找报告时不用回头问 AI
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let logs = PathBuf::from(home).join(".tke").join("logs");
        println!("  {} {}", dim("日志落点"), logs.display());
    }

    // 浏览器**一律默认无头**（有头会抢鼠标和焦点）。这里报的是"这台机器能不能开有头"，
    // 因为需要人在窗口里手动登录时要靠它
    if tke::utils::params::desktop_available() {
        println!("  {} {} {}", dim("浏览器  "), "无头运行", dim("· --headless=off 可开窗口（手动登录时用）"));
    } else {
        println!("  {} {} {}", dim("浏览器  "), "无头运行", dim("· 本机无图形界面，开不了窗口"));
    }

    // ── 结论 ──（对钩在最后：上面每行是一项检查，这行才是"到底行不行"）
    println!();
    if missing == 0 {
        println!("  {} {}", sym_ok(), "全局已就绪");
    } else {
        println!(
            "  {} 环境不完整 · 缺 {} 项{}",
            sym_err(),
            missing,
            dim("　补齐：tke doctor --fix")
        );
    }
    // 更新提示只出现一次：tke 和 skill 谁旧都是同一条命令，分开说两遍是噪音
    if st.as_ref().is_some_and(|s| s.any_stale()) {
        println!("  {} 有可用更新{}", sym_warn(), dim("　更新：tke update"));
    }
}

// ── 检测 ────────────────────────────────────────────────────────────────

fn detect_missing(exe_dir: &Path, profile: &str) -> Vec<Missing> {
    let mut out = Vec::new();
    let want_web = profile == "web" || profile == "all";
    let want_android = profile == "android" || profile == "all";
    let want_ios = profile == "ios" || profile == "all";

    if want_web {
        // chromedriver 必须与 tke 同目录：ToolManager 只搜同目录、不回退 PATH，
        // 版本配对就靠这个约束
        if !deps::present_in(exe_dir, "chromedriver") {
            out.push(Missing {
                name: "chromedriver",
                what: "驱动浏览器所需",
                size: "20MB",
                is_chrome: false,
            });
        }
        if deps::chrome_for_testing_bin().is_none() {
            out.push(Missing {
                name: "chrome",
                what: "Chrome for Testing 浏览器本体",
                size: "600MB",
                is_chrome: true,
            });
        }
    }
    if want_android {
        if !deps::present_in(exe_dir, "adb") {
            out.push(Missing {
                name: "adb",
                what: "连接安卓设备所需",
                size: "10MB",
                is_chrome: false,
            });
        } else if cfg!(windows) && !exe_dir.join("AdbWinApi.dll").is_file() {
            // adb.exe 在、DLL 不在：一样跑不起来，而且报错很难懂
            // （从别处拷 adb.exe 过来最容易出现这种半装状态）
            out.push(Missing {
                name: "AdbWinApi.dll",
                what: "adb.exe 缺了它起不来",
                size: "0.1MB",
                is_chrome: false,
            });
        }
    }
    // 上游没这个平台的包时不报"缺"——报了也补不上，只会让人反复试
    // 宿主机做不了 iOS 就不报"缺 go-ios"——补上也用不了（门禁在 Controller 那层）
    if want_ios && tke::utils::capability::ios_supported() && upstream_has_go_ios()
        && !deps::present_in(exe_dir, "go-ios")
    {
        out.push(Missing {
            name: "go-ios",
            what: "连接 iOS 设备所需",
            size: "23MB",
            is_chrome: false,
        });
    }
    out
}

fn tke_dir() -> Result<PathBuf> {
    deps::tke_dir().ok_or_else(|| TkeError::InvalidArgument("无法获取 tke 所在目录".into()))
}

/// 这个平台的驱动上游有没有官方包。
///
/// **Chrome for Testing 只出 linux64 / mac-arm64 / mac-x64 / win32 / win64**，
/// Google 的 platform-tools 也不出 arm64 Linux 版（实测全 404）。所以 arm64 Linux
/// 上装不了我们分发的驱动——这不是分发源漏传，是上游根本没有。
/// 与其让人对着"下载失败"猜，不如直说该怎么办。
fn upstream_has_drivers() -> bool {
    !(std::env::consts::OS == "linux" && std::env::consts::ARCH == "aarch64")
}

/// go-ios 有没有这个平台的官方包。
/// 它的 Windows 发行包**只有 64 位**（zip 里那个 ios.exe 是 x86-64），
/// 32 位 Windows 上跑不了——所以 windows-386 的分发目录里**有意不放** go-ios。
fn upstream_has_go_ios() -> bool {
    !(std::env::consts::OS == "windows" && std::env::consts::ARCH == "x86")
}

/// 分发源上的平台目录名，与 bin/<platform>/ 一致
fn platform_tag() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86" => "386",
        "x86_64" => "amd64",
        a => return Err(TkeError::InvalidArgument(format!("不支持的架构: {}", a))),
    };
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        o => return Err(TkeError::InvalidArgument(format!("不支持的系统: {}", o))),
    };
    Ok(format!("{}-{}", os, arch))
}

// ── 下载 / 安装 ──────────────────────────────────────────────────────────

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::Write;
    print!("{} [y/N] ", prompt);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return Ok(false); // 非交互环境（管道里跑）当作否——下载要显式同意
    }
    Ok(matches!(line.trim(), "y" | "Y" | "yes"))
}

fn curl_text(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "20", url])
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// 下载到文件并**验文件头**——分发平台对不存在的路径回落 200 + HTML，
/// 只看 curl 退出码会把网页当成二进制装进去（P-19）
fn curl_file(url: &str, out: &Path, magic: &[u8]) -> Result<()> {
    let st = Command::new("curl")
        .args(["-fsSL", "--retry", "2", "--max-time", "900", url, "-o"])
        .arg(out)
        .status()
        .map_err(|e| TkeError::InvalidArgument(format!("curl 起不来（没装？）：{}", e)))?;
    if !st.success() {
        let _ = std::fs::remove_file(out);
        return Err(TkeError::InvalidArgument("下载失败".into()));
    }
    let mut head = vec![0u8; magic.len()];
    let ok = std::fs::File::open(out)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head == magic;
    if !ok {
        let _ = std::fs::remove_file(out);
        return Err(TkeError::InvalidArgument(
            "下到的不是预期文件（多半是这个路径分发源上还没有）".into(),
        ));
    }
    Ok(())
}

fn install_bin(
    base: &str,
    q: &str,
    tmp: &Path,
    exe_dir: &Path,
    name: &str,
    platform: &str,
) -> Result<()> {
    let gz = tmp.join(format!("{}.gz", name));
    curl_file(&format!("{}/bin/{}/{}.gz{}", base, platform, name, q), &gz, &[0x1f, 0x8b])?;

    let raw = std::fs::read(&gz).map_err(TkeError::IoError)?;
    let mut dec = flate2::read::GzDecoder::new(&raw[..]);
    let mut bytes = Vec::new();
    dec.read_to_end(&mut bytes)
        .map_err(|e| TkeError::InvalidArgument(format!("解压失败：{}", e)))?;

    // 分发源上统一叫 `<name>.gz`（不带平台后缀），**落地时 Windows 要补回 `.exe`**——
    // 否则落成一个没有扩展名的文件，Windows 上根本执行不了。
    // （`libc++.so` 这类本身带点的不动它，那也只有 Linux 会用到）
    let dest = if cfg!(windows) && !name.contains('.') {
        exe_dir.join(format!("{}.exe", name))
    } else {
        exe_dir.join(name)
    };
    // 先删后拷：覆盖正在运行的二进制会 ETXTBSY(Linux) / 签名失配被杀(macOS，P-02)
    let _ = std::fs::remove_file(&dest);
    std::fs::write(&dest, bytes).map_err(TkeError::IoError)?;
    set_exec(&dest);
    clear_quarantine(&dest);

    // adb 的**伴生文件按平台不同**，补 adb 时顺手带上（缺了不算失败）：
    //   Linux   —— aapt 单独跑不了（缺 libc++.so），但其 RUNPATH 含 $ORIGIN，同目录即可加载
    //   Windows —— adb.exe **直接依赖 AdbWinApi.dll**，USB 还要 AdbWinUsbApi.dll
    //              （后者由前者在运行时加载，不在导入表里，但没有它连不上真机）。
    //              这两个 DLL 不在的话 adb.exe 根本起不来。
    if name == "adb" {
        let extras: &[&str] = match std::env::consts::OS {
            "linux" => &["aapt", "libc++.so"],
            "windows" => &["aapt", "AdbWinApi.dll", "AdbWinUsbApi.dll"],
            _ => &["aapt"],
        };
        for extra in extras {
            let _ = install_bin(base, q, tmp, exe_dir, extra, platform);
        }
    }
    Ok(())
}

fn install_chrome(base: &str, q: &str, tmp: &Path, _platform: &str) -> Result<()> {
    let pkg = deps::chrome_pkg_name();
    let zip_path = tmp.join(format!("{}.zip", pkg));
    curl_file(&format!("{}/chrome/{}.zip{}", base, pkg, q), &zip_path, b"PK")?;

    let dir = deps::chrome_data_dir()
        .ok_or_else(|| TkeError::InvalidArgument("找不到用户数据目录（HOME 没设？）".into()))?;
    std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;

    let file = std::fs::File::open(&zip_path).map_err(TkeError::IoError)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| TkeError::InvalidArgument(format!("Chrome 包不是有效 zip：{}", e)))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| TkeError::InvalidArgument(format!("读取 zip 条目失败：{}", e)))?;
        // 防 zip-slip：enclosed_name 拒绝绝对路径与 `..`
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(TkeError::IoError)?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(TkeError::IoError)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(TkeError::IoError)?;
        std::fs::write(&out, buf).map_err(TkeError::IoError)?;
        // zip 不保留可执行位，Chrome 的一堆 helper 必须能执行
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if entry.unix_mode().is_some_and(|m| m & 0o111 != 0) {
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755));
            }
        }
    }
    clear_quarantine(&dir.join(pkg));

    // 解压完必须确认那个二进制真的在——半个解压出来的目录也是目录，
    // 只看目录会把"装坏了"当成"装好了"
    if deps::chrome_for_testing_bin().is_none() {
        let _ = std::fs::remove_dir_all(dir.join(pkg)); // 清掉半成品，免得下次误判成已装
        return Err(TkeError::InvalidArgument(
            "解压后找不到 Chrome 可执行文件（包结构不对？）".into(),
        ));
    }
    Ok(())
}

fn set_exec(p: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = p;
}

/// macOS：清隔离属性，否则自动化下会卡在授权弹窗且**没有任何报错**
fn clear_quarantine(p: &Path) {
    if std::env::consts::OS == "macos" {
        let _ = Command::new("xattr").arg("-cr").arg(p).status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_tag_matches_dist_layout() {
        let t = platform_tag().unwrap();
        assert!(
            t.starts_with("linux-") || t.starts_with("darwin-") || t.starts_with("windows-"),
            "平台标签要与分发源 bin/<platform>/ 的命名一致：{}",
            t
        );
    }

    /// profile 决定检查哪些依赖——android 不该因为没装 chromedriver 就说缺
    #[test]
    fn profile_scopes_what_is_checked() {
        let empty = std::env::temp_dir().join(format!("tke-fix-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();

        let names: Vec<&str> = detect_missing(&empty, "android").iter().map(|m| m.name).collect();
        assert!(names.contains(&"adb"), "android 应查 adb：{:?}", names);
        assert!(!names.contains(&"chromedriver"), "android 不该查 chromedriver：{:?}", names);

        // iOS 这条按宿主机能力分：做不了的机器上不该报缺——补上也用不了（门禁在 Controller 层）
        let names: Vec<&str> = detect_missing(&empty, "ios").iter().map(|m| m.name).collect();
        if tke::utils::capability::ios_supported() {
            assert_eq!(names, vec!["go-ios"], "支持 iOS 的机器上该查 go-ios");
        } else {
            assert!(names.is_empty(), "做不了 iOS 的机器不该报缺 go-ios：{:?}", names);
        }

        let _ = std::fs::remove_dir_all(&empty);
    }

    /// 已存在的依赖不该再报缺（幂等的基础）
    #[test]
    fn present_binaries_are_not_reported_missing() {
        let dir = std::env::temp_dir().join(format!("tke-fix-has-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("adb"), b"x").unwrap();

        let names: Vec<&str> = detect_missing(&dir, "android").iter().map(|m| m.name).collect();
        assert!(names.is_empty(), "adb 在场就不该报缺：{:?}", names);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
