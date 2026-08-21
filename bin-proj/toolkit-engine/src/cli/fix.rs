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

use crate::cli::doctor::{dim, section, sym_dot, sym_err, sym_ok, sym_warn, wda_app_dir, wda_app_path, Dep, Health};
use tke::utils::deps;
use tke::{Result, TkeError};

const DEFAULT_BASE_URL: &str = "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

/// Doctor 命令参数（`tke fix` 是它的别名，见 main.rs）
#[derive(clap::Args)]
pub struct FixArgs {
    /// 只看这一类：web（浏览器）/ android / ios / all（默认）。
    /// `android-emu` = 安卓模拟器，**选装**，只有显式点名才装（ADR-0018）
    #[arg(long, default_value = "all", value_parser = ["web", "android", "ios", "android-emu", "all"])]
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


/// `tke doctor --profile android-emu`（不加 --fix）：只说装没装、装了多大、怎么装
fn report_android_emu() -> Result<()> {
    use crate::cli::android_sdk as sdk;
    if sdk::installed() {
        let size = sdk::installed_size_mb().unwrap_or(0);
        println!("  {} {}", dim("状态    "), format!("已装 · {} MB", size));
        println!("  {} {}", dim("落点    "), sdk::sdk_dir().map(|d| d.display().to_string()).unwrap_or_default());
        println!("  {} {}", dim("AVD     "), sdk::AVD_NAME);
        println!();
        println!("  {} {}", sym_ok(), "可用");
        println!("    {}", dim(&format!("起它：tke -d avd:{} control boot", sdk::AVD_NAME)));
    } else {
        println!("  {} {}", dim("状态    "), dim("未安装（选装）"));
        println!();
        println!("  {}", dim("安卓真机插上即用，模拟器不是必经之路——要装再装："));
        println!("    tke doctor --fix --profile android-emu");
        println!("  {}", dim("约 1GB（emulator + 系统镜像），从 Google 官方源下载"));
    }
    Ok(())
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

    // `android-emu` 走**另一条路**：它不是"跑起来所缺的依赖"，是一件选装的大件
    // （ADR-0018）。混进体检表会让人以为不装就不完整——恰恰相反，多数人根本不需要它
    if args.profile == "android-emu" {
        section("ANDROID EMULATOR");
        if !download {
            return report_android_emu();
        }
        return crate::cli::android_sdk::install(args.yes).await;
    }

    let exe_dir = tke_dir()?;
    let platform = platform_tag()?;
    let deps_now = detect_deps(&exe_dir, &args.profile);
    let health = Health::probe(&exe_dir, &platform, &args.profile);

    // 报告先整张打出来（分组 + 对齐，排版规则见 doctor.rs），
    // 下载与否都从**同一张体检表**出发——`--fix` 不该给人另一套面貌
    section("DOCTOR");
    health.print(&deps_now);

    let missing: Vec<&Dep> = deps_now.iter().filter(|d| !d.present).collect();

    // arm64 Linux：浏览器与安卓那套驱动上游就没有官方包，下了也是白下
    if !missing.is_empty() && !upstream_has_drivers() {
        let need_drivers = missing.iter().any(|m| m.is_chrome || m.name != "go-ios");
        if need_drivers {
            println!();
            println!("  {} arm64 Linux 上游没有官方驱动包（Chrome for Testing 与 Google 的", sym_warn());
            println!("    platform-tools 都不出这个架构），分发源自然也没有。请改用发行版自带的：");
            println!("      sudo apt install -y chromium chromium-driver adb");
            println!("    再把 chromium-driver 的 chromedriver 软链到 tke 同目录（tke 只在那儿找）：");
            println!("      ln -sf \"$(command -v chromedriver)\" \"{}/chromedriver\"", exe_dir.display());
            println!("    go-ios 有 arm64 版，`tke fix --profile ios` 照常能补。");
        }
    }

    if !download {
        health.print_verdict(missing.len());
        if !missing.is_empty() {
            // 只体检不下载：退出码非 0，CI 可以据此判断环境是否就绪
            std::process::exit(1);
        }
        return Ok(());
    }

    // ── 以下是 `--fix`：真的要联网下载了 ──
    if missing.is_empty() {
        // 二进制都齐了不代表没事干：模拟器还要 WDA runner
        let nonce = std::process::id();
        let tmp = std::env::temp_dir().join(format!("tke-fix-{}", nonce));
        let _ = std::fs::create_dir_all(&tmp);
        fix_sim_wda(&base_url, &format!("?t={}", nonce), &tmp, &args.profile);
        let _ = std::fs::remove_dir_all(&tmp);
        return Ok(());
    }

    println!();
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

    fix_sim_wda(&base_url, &q, &tmp, &args.profile);

    println!();
    // 复验：以"现在还缺不缺"为准，而不是以"下载有没有报错"为准
    let still: Vec<Dep> =
        detect_deps(&exe_dir, &args.profile).into_iter().filter(|d| !d.present).collect();
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

/// `--fix` 时顺带把**模拟器用的 WebDriverAgent** 装上（只在 macOS、只在 iOS profile）。
///
/// **失败不改退出码**：doctor 的退出码说的是「必需依赖齐不齐」，而这个只影响
/// iOS *模拟器*——安卓、网页、iOS 真机都不靠它。但**必须打出来**（INV-9），
/// 否则用户会以为模拟器可以用了。
fn fix_sim_wda(base_url: &str, q: &str, tmp: &Path, profile: &str) {
    if !cfg!(target_os = "macos") || !matches!(profile, "ios" | "all") || wda_app_path().is_some() {
        return;
    }
    println!();
    println!("  {} {}", dim("iOS模拟器"), "装 WebDriverAgent（模拟器的点击与元素采集靠它）");
    match install_sim_wda(base_url, q, tmp) {
        Ok(()) => println!("  {} 装好了", sym_ok()),
        Err(e) => {
            println!("  {} 没装成：{}", sym_warn(), e);
            println!("  {}", dim("（不影响安卓 / 网页 / iOS 真机——只是模拟器还操作不了）"));
        }
    }
}

/// 下载并解开模拟器版 WDA。
///
/// 为什么是**我们自己分发一个预编译产物**，而不是让用户 brew/pip 装别的东西：
///   - 版本由我们锁 —— 上游哪天变了不会突然把用户的环境搞坏
///   - 模拟器**不需要签名**，一个 `.app` 拷进去就能跑（真机必须签，那是 Apple 的限制）
///   - 实测 `simctl launch` 直接起得来，连 xcodebuild 和 .xctestrun 都不用带
///   - 之后**整套 WDA 协议代码与真机共用**（HTTP+JSON，早就在跑了）
fn install_sim_wda(base: &str, q: &str, tmp: &Path) -> Result<()> {
    let dir = wda_app_dir()
        .ok_or_else(|| TkeError::InvalidArgument("找不到用户目录（HOME 没设？）".into()))?;
    let zip_path = tmp.join("wda-sim.zip");
    curl_file(&format!("{}/wda/WebDriverAgentRunner-Runner-sim.zip{}", base, q), &zip_path, b"PK")?;

    std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;
    let file = std::fs::File::open(&zip_path).map_err(TkeError::IoError)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| TkeError::InvalidArgument(format!("WDA 包不是有效 zip：{}", e)))?;
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
        // zip 不保留可执行位，而 .app 里的主程序必须能执行
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if entry.unix_mode().is_some_and(|m| m & 0o111 != 0) {
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755));
            }
        }
    }
    clear_quarantine(&dir);

    // 解开了不等于装好了：确认那个 .app 真的在（半个解压出来的目录也是目录）
    if wda_app_path().is_none() {
        let _ = std::fs::remove_dir_all(&dir); // 清掉半成品，免得下次误判成已装
        return Err(TkeError::InvalidArgument(
            "解压后找不到 WebDriverAgentRunner-Runner.app（包结构不对？）".into(),
        ));
    }
    Ok(())
}


// ── 检测 ────────────────────────────────────────────────────────────────


/// 这个 profile 该有哪些依赖，以及**每一项现在在不在**。
///
/// 返回全量而不是只返回缺的：体检要说得出「7 项已就绪」，光有缺失列表数不出分母。
fn detect_deps(exe_dir: &Path, profile: &str) -> Vec<Dep> {
    let mut out = Vec::new();
    let want_web = profile == "web" || profile == "all";
    let want_android = profile == "android" || profile == "all";
    let want_ios = profile == "ios" || profile == "all";

    if want_web {
        // chromedriver 必须与 tke 同目录：ToolManager 只搜同目录、不回退 PATH，
        // 版本配对就靠这个约束
        out.push(Dep {
            name: "chromedriver",
            what: "驱动浏览器所需",
            size: "20MB",
            is_chrome: false,
            present: deps::present_in(exe_dir, "chromedriver"),
        });
        out.push(Dep {
            name: "chrome",
            what: "Chrome for Testing 浏览器本体",
            size: "600MB",
            is_chrome: true,
            present: deps::chrome_for_testing_bin().is_some(),
        });
    }
    if want_android {
        let adb = deps::present_in(exe_dir, "adb");
        out.push(Dep {
            name: "adb",
            what: "连接安卓设备所需",
            size: "10MB",
            is_chrome: false,
            present: adb,
        });
        // adb.exe 在、DLL 不在：一样跑不起来，而且报错很难懂
        // （从别处拷 adb.exe 过来最容易出现这种半装状态）
        if cfg!(windows) && adb {
            out.push(Dep {
                name: "AdbWinApi.dll",
                what: "adb.exe 缺了它起不来",
                size: "0.1MB",
                is_chrome: false,
                present: exe_dir.join("AdbWinApi.dll").is_file(),
            });
        }
    }
    // 上游没这个平台的包时不列——报了也补不上，只会让人反复试。
    // 宿主机做不了 iOS 也不列（门禁在 Controller 那层，补上也用不了）
    if want_ios && tke::utils::capability::ios_supported() && upstream_has_go_ios() {
        out.push(Dep {
            name: "go-ios",
            what: "连接 iOS 设备所需",
            size: "23MB",
            is_chrome: false,
            present: deps::present_in(exe_dir, "go-ios"),
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

pub(crate) fn confirm(prompt: &str) -> Result<bool> {
    use std::io::Write;
    print!("{} [y/N] ", prompt);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    // 等输入期间 Ctrl+C 立即退出，不然它要等到用户敲回车才生效
    let _g = tke::utils::interrupt::prompting();
    if std::io::stdin().read_line(&mut line).is_err() {
        return Ok(false); // 非交互环境（管道里跑）当作否——下载要显式同意
    }
    Ok(matches!(line.trim(), "y" | "Y" | "yes"))
}

pub(crate) fn curl_text(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "20", url])
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// 下载到文件并**验文件头**——分发平台对不存在的路径回落 200 + HTML，
/// 只看 curl 退出码会把网页当成二进制装进去（P-19）
pub(crate) fn curl_file(url: &str, out: &Path, magic: &[u8]) -> Result<()> {
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

        let names: Vec<&str> = detect_deps(&empty, "android").iter().filter(|d| !d.present).map(|d| d.name).collect();
        assert!(names.contains(&"adb"), "android 应查 adb：{:?}", names);
        assert!(!names.contains(&"chromedriver"), "android 不该查 chromedriver：{:?}", names);

        // iOS 这条按宿主机能力分：做不了的机器上不该报缺——补上也用不了（门禁在 Controller 层）
        let names: Vec<&str> = detect_deps(&empty, "ios").iter().filter(|d| !d.present).map(|d| d.name).collect();
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

        let names: Vec<&str> = detect_deps(&dir, "android").iter().filter(|d| !d.present).map(|d| d.name).collect();
        assert!(names.is_empty(), "adb 在场就不该报缺：{:?}", names);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

