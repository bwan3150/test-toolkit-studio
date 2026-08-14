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

const DEFAULT_BASE_URL: &str = "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

/// Fix 命令参数
#[derive(clap::Args)]
pub struct FixArgs {
    /// 只补这一类：web（浏览器）/ android / ios / all（默认，按平台补齐所有缺的）
    #[arg(long, default_value = "all", value_parser = ["web", "android", "ios", "all"])]
    pub profile: String,

    /// 只检查不下载——报告缺什么就退出（CI 里适合用这个 + 退出码判断）
    #[arg(long)]
    pub check: bool,

    /// 不询问直接下载（脚本/CI 里用）
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// 分发源地址（默认走官方源；也可用环境变量 TKE_BASE_URL）
    #[arg(long)]
    pub base_url: Option<String>,
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
    let base_url = args
        .base_url
        .clone()
        .or_else(|| std::env::var("TKE_BASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let exe_dir = tke_dir()?;
    let platform = platform_tag()?;

    println!("== tke fix ==");
    println!("   平台     {}", platform);
    println!("   落点     {}", exe_dir.display());
    println!("   分发源   {}", base_url);
    println!();

    let missing = detect_missing(&exe_dir, &args.profile);

    if missing.is_empty() {
        println!("✅ {} 需要的依赖都在，不用补。", args.profile);
        return Ok(());
    }

    println!("缺少 {} 项：", missing.len());
    for m in &missing {
        println!("   ❌ {:<14} {}（约 {}）", m.name, m.what, m.size);
    }
    println!();

    if args.check {
        // --check 只报告不下载：退出码非 0，CI 可以据此判断环境是否就绪
        println!("（--check 模式，未下载。去掉 --check 即可补齐）");
        std::process::exit(1);
    }

    if !args.yes && !confirm("要现在下载补齐吗？")? {
        println!("已取消。需要时再跑 `tke fix`。");
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
        print!("-- {} ... ", m.name);
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let r = if m.is_chrome {
            install_chrome(&base_url, &q, &tmp, &platform)
        } else {
            install_bin(&base_url, &q, &tmp, &exe_dir, m.name, &platform)
        };
        match r {
            Ok(()) => println!("✅"),
            Err(e) => {
                println!("❌ {}", e);
                failed.push(m.name);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);

    println!();
    // 复验：以"现在还缺不缺"为准，而不是以"下载有没有报错"为准
    let still = detect_missing(&exe_dir, &args.profile);
    if still.is_empty() {
        println!("✅ 补齐了。");
        Ok(())
    } else {
        // 失败要可见（INV-9）：不能下完就说好了，得如实说还缺什么
        println!("⚠️  还缺 {} 项：", still.len());
        for m in &still {
            println!("   ❌ {} —— {}", m.name, m.what);
        }
        if !failed.is_empty() {
            println!();
            println!("   下载失败的多半是分发源上还没有这个平台的文件。");
            println!("   手动装的办法见 skill 的 README，或换 --base-url 指向你自己的源。");
        }
        std::process::exit(1);
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
    if want_ios && !deps::present_in(exe_dir, "go-ios") {
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

/// 分发源上的平台目录名，与 bin/<platform>/ 一致
fn platform_tag() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
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

        let names: Vec<&str> = detect_missing(&empty, "ios").iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["go-ios"], "ios 只该查 go-ios");

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
