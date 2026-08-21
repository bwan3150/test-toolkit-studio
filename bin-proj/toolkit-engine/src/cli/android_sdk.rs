// 【安卓模拟器的安装】从 **Google 官方源**下载 emulator + 系统镜像，建一台 AVD。
//
// ⚠️ **为什么不放进我们自己的分发源**（用户问过，查了条款）：
// Android SDK 许可 3.4 明文禁止 redistribute「the SDK or any part of the SDK」，
// 3.1 又是 non-sublicensable——我们没有权利把这些字节转授给下载我们镜像的人。
// 这跟 WebDriverAgent 不同：那个是 BSD 开源的，自己编译产物再分发完全合法（ADR-0017）。
// 所以这里的模式是：**我们做编排，Google 做分发**——字节来自 dl.google.com，
// 许可关系是用户 ↔ Google，我们只是替他敲了那几行命令（同 ADR-0014 的 `tke update`
// 就是去跑官方 install.sh）。下载前把这句话说给人听。
//
// 顺带一个好处：**不需要 JDK**。官方的 `sdkmanager` / `avdmanager` 是 Java 写的，
// 装它们得先装 JDK；而包的直链就在 Google 的仓库 XML 里，`curl` + 解压就完事。
// AVD 也不用 `avdmanager` 建——它本质就是两个 ini 文件，自己写更直接（也少一个依赖）。

use std::path::{Path, PathBuf};

use crate::cli::doctor::{dim, sym_dot, sym_ok};
use tke::{Result, TkeError};

const REPO_BASE: &str = "https://dl.google.com/android/repository";

/// 默认装哪个 API。34 是体积与兼容性的平衡点（`aosp_atd` x86_64 615MB / arm64 599MB）；
/// 更高的 API 镜像明显更大（36 是 746MB），更低的又开始缺现代 App 要的能力
const DEFAULT_API: u32 = 34;

/// 我们建的那台 AVD 叫什么。**固定名字**：这是"tke 装的那一台"，
/// 人自己用 Android Studio 建的那些照常并存、互不干扰
pub const AVD_NAME: &str = "tke";

/// 我们装的 SDK 落哪儿。**不碰用户已有的 `~/Android/Sdk`**——
/// 那是他自己的东西，我们往里塞会让卸载变成一件说不清的事
pub fn sdk_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".tke").join("android-sdk"))
}

/// AVD 放我们自己的目录，靠 `ANDROID_AVD_HOME` 让 emulator 找到它。
/// **不往 `~/.android/avd` 里写**：那是用户的地盘，卸载时不好界定该删哪些
pub fn avd_dir() -> Option<PathBuf> {
    sdk_dir().map(|d| d.join("avd"))
}

/// 这台机器的宿主标签（Google 仓库 XML 里的 host-os / host-arch）。
/// 官方**只出这四个**——linux-arm64 与 windows-arm64 Google 至今不发布（ADR-0018）
fn host_tags() -> Option<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some(("linux", "x64")),
        ("macos", "aarch64") => Some(("macosx", "aarch64")),
        ("macos", "x86_64") => Some(("macosx", "x64")),
        ("windows", "x86_64") => Some(("windows", "x64")),
        _ => None,
    }
}

/// 这台机器该用哪个 abi 的系统镜像
fn image_abi() -> &'static str {
    if std::env::consts::ARCH == "aarch64" { "arm64-v8a" } else { "x86_64" }
}

/// 装好了没有——**两样都在才算**：emulator 二进制，以及那台 AVD 的配置
pub fn installed() -> bool {
    emulator_path().is_some_and(|p| p.is_file())
        && avd_dir().is_some_and(|d| d.join(format!("{}.ini", AVD_NAME)).is_file())
}

pub fn emulator_path() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "emulator.exe" } else { "emulator" };
    sdk_dir().map(|d| d.join("emulator").join(exe))
}

/// 装：emulator → 系统镜像 → 建 AVD。每一步单独报，失败就停在那一步
pub async fn install(yes: bool) -> Result<()> {
    let Some((host_os, host_arch)) = host_tags() else {
        return Err(TkeError::InvalidArgument(format!(
            "{}-{} 上没有官方 Android 模拟器（Google 不发布 linux-arm64 / windows-arm64）。\n\
             \u{3000}那一档只能用 redroid 之类的容器方案，见 docs/adr/0018-android-emulator-optional.md",
            std::env::consts::OS,
            std::env::consts::ARCH
        )));
    };
    let sdk = sdk_dir().ok_or_else(|| TkeError::InvalidArgument("找不到用户目录".into()))?;

    // 先把清单取下来，**把要下什么、多大、从哪来说清楚再问**——
    // 这不是我们的分发源，人有权先知道字节从哪儿来（许可关系是他 ↔ Google）
    println!("  {} 取包清单…", sym_dot());
    let emu = pick_emulator(host_os, host_arch)?;
    let img = pick_image(DEFAULT_API, image_abi())?;

    println!();
    println!("  将从 {} 下载：", dim("Google 官方源 dl.google.com"));
    println!("    emulator      {:>6} MB", emu.size / 1_000_000);
    println!("    系统镜像       {:>6} MB   {}", img.size / 1_000_000, dim(&img.pkg));
    println!("    落点          {}", sdk.display());
    // 下载多少 ≠ 占盘多少。系统镜像是稀疏的，展开后逻辑 8GB 上下、
    // 实占约 2GB——两个数字都说，人才好判断这台机器塞不塞得下
    println!("    {}", dim("装完约占 2GB（系统镜像是稀疏的，逻辑大小会显示 8GB 上下）"));
    println!();
    println!("  {}", dim("这是 Google 的 Android SDK，下载即表示你接受 Android SDK 许可"));
    println!("  {}", dim("（https://developer.android.com/studio/terms）"));
    println!("  {}", dim("tke 不转发这些文件——我们只替你把它们取下来放好"));

    if !yes && !crate::cli::fix::confirm("\n  开始下载吗？")? {
        println!("  {}", dim("已取消"));
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("tke-avd-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(TkeError::IoError)?;

    println!();
    print!("  {} emulator … ", sym_dot());
    flush();
    let zip = tmp.join("emulator.zip");
    crate::cli::fix::curl_file(&format!("{}/{}", REPO_BASE, emu.url), &zip, b"PK")?;
    // zip 里第一层就是 `emulator/`，所以解到 SDK 根
    unzip_into(&zip, &sdk)?;
    println!("{}", sym_ok());

    print!("  {} 系统镜像 … ", sym_dot());
    flush();
    let zip = tmp.join("sysimg.zip");
    crate::cli::fix::curl_file(&format!("{}/sys-img/aosp_atd/{}", REPO_BASE, img.url), &zip, b"PK")?;
    // 镜像 zip 的第一层是 abi 名（`x86_64/`），要落在
    // `system-images/android-<API>/aosp_atd/` 下面——emulator 按这个路径找
    let dest = sdk.join("system-images").join(format!("android-{}", DEFAULT_API)).join("aosp_atd");
    std::fs::create_dir_all(&dest).map_err(TkeError::IoError)?;
    unzip_into(&zip, &dest)?;
    println!("{}", sym_ok());
    let _ = std::fs::remove_dir_all(&tmp);

    print!("  {} 建 AVD `{}` … ", sym_dot(), AVD_NAME);
    flush();
    create_avd(&sdk, DEFAULT_API, image_abi())?;
    println!("{}", sym_ok());

    println!();
    println!("  {} 装好了", sym_ok());
    println!("    {}", dim(&format!("起它：tke -d avd:{} control boot", AVD_NAME)));
    println!("    {}", dim("看设备：tke device list"));
    if cfg!(target_os = "linux") && !kvm_ok() {
        // Linux 上没有 KVM 权限，模拟器会以纯软件模拟跑——慢到没法用。
        // **装完就说**，别等他起模拟器等十分钟才发现
        println!();
        println!("  {} 这台机器还不能用 KVM 加速，模拟器会慢到没法用", crate::cli::doctor::sym_warn());
        println!("    {}", dim("修：sudo usermod -aG kvm $USER  然后重新登录（或重启）"));
    }
    Ok(())
}

/// 删掉我们装的那一套。**只删我们自己放的**——用户已有的 `~/Android/Sdk` 一个字节都不碰
pub fn remove() -> Result<()> {
    if let Some(d) = sdk_dir() {
        if d.exists() {
            std::fs::remove_dir_all(&d).map_err(TkeError::IoError)?;
        }
    }
    Ok(())
}

/// 装了多大——卸载前要摆出来给人看（1GB 级的东西，删之前该知道删的是多少）
pub fn installed_size_mb() -> Option<u64> {
    let d = sdk_dir()?;
    if !d.exists() {
        return None;
    }
    // 算**实际占盘**而不是逻辑大小：`system.img` 是稀疏的——逻辑 8.1GB、
    // 实占 1.1GB。报逻辑大小会让卸载预览说"9603 MB"，而人量出来只有 2GB
    fn size_of(m: &std::fs::Metadata) -> u64 {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            m.blocks() * 512
        }
        #[cfg(not(unix))]
        {
            m.len()
        }
    }
    fn walk(p: &Path) -> u64 {
        let Ok(rd) = std::fs::read_dir(p) else { return 0 };
        rd.flatten()
            .map(|e| match e.file_type() {
                Ok(t) if t.is_dir() => walk(&e.path()),
                Ok(_) => e.metadata().map(|m| size_of(&m)).unwrap_or(0),
                _ => 0,
            })
            .sum()
    }
    Some(walk(&d) / 1_000_000)
}

fn kvm_ok() -> bool {
    // 存在还不够，**要能读写**：`/dev/kvm` 在但不在 kvm 组里是最常见的情形
    std::fs::OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok()
}

fn flush() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

// ── 仓库清单 ─────────────────────────────────────────────────────────────

pub struct Pkg {
    pub pkg: String,
    pub url: String,
    pub size: u64,
}

/// 从 `repository2-3.xml` 里挑这台机器要的 emulator。
///
/// **按 revision 取最新**：同一个 path 在清单里会出现好几次（不同版本），
/// 顺序不代表新旧——第一版写成"取最后一个"，实测装到的是 37.1.11 而不是最新那个
fn pick_emulator(host_os: &str, host_arch: &str) -> Result<Pkg> {
    let xml = fetch(&format!("{}/repository2-3.xml", REPO_BASE))?;
    let mut best: Option<(Vec<u32>, Pkg)> = None;
    for a in archives(&xml) {
        if a.path == "emulator" && a.os == host_os && a.arch == host_arch {
            let newer = best.as_ref().is_none_or(|(r, _)| a.rev > *r);
            if newer {
                best = Some((a.rev.clone(), Pkg { pkg: a.path, url: a.url, size: a.size }));
            }
        }
    }
    best.map(|(_, p)| p).ok_or_else(|| {
        TkeError::DeviceError(format!("Google 的清单里没有 {}-{} 的 emulator", host_os, host_arch))
    })
}

/// 从 `sys-img/aosp_atd/sys-img2-3.xml` 里挑指定 API + abi 的系统镜像。
/// 用 `aosp_atd` 是有意的：Google 给自动化测试用的精简镜像，比 `google_apis` 省约 40%，
/// 代价是**没有 Google Play 服务**——要测依赖 GMS 的 App 得换 `google_atd`
fn pick_image(api: u32, abi: &str) -> Result<Pkg> {
    let xml = fetch(&format!("{}/sys-img/aosp_atd/sys-img2-3.xml", REPO_BASE))?;
    let want = format!("system-images;android-{};aosp_atd;{}", api, abi);
    let mut best: Option<(Vec<u32>, Pkg)> = None;
    for a in archives(&xml) {
        if a.path == want {
            let newer = best.as_ref().is_none_or(|(r, _)| a.rev > *r);
            if newer {
                best = Some((a.rev.clone(), Pkg { pkg: a.path, url: a.url, size: a.size }));
            }
        }
    }
    best.map(|(_, p)| p).ok_or_else(|| TkeError::DeviceError(format!("Google 的清单里没有 {}", want)))
}

/// 清单里的一条 `<archive>`
struct Entry {
    path: String,
    os: String,
    arch: String,
    url: String,
    size: u64,
    /// `<revision>` 的 major.minor.micro——**挑最新版靠它**，不是靠出现顺序
    rev: Vec<u32>,
}

/// 把清单里的 `<archive>` 摊平。
///
/// 手写事件循环而不是找个 XML 映射库：这份清单有好几个 schema 版本混在一起
///（老包没有 `host-arch`、`<complete>` 与直接放 `<url>` 两种写法都有），
/// 用结构体去套反而要处处 Option，还得跟着 Google 改 schema
fn archives(xml: &str) -> Vec<Entry> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_str(xml);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    let (mut path, mut os, mut arch, mut url, mut size) =
        (String::new(), String::new(), String::new(), String::new(), 0u64);
    let mut rev: Vec<u32> = Vec::new();
    let mut in_rev = false;
    let mut tag = String::new();
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match tag.as_str() {
                    "revision" => {
                        in_rev = true;
                        rev.clear();
                    }
                    "remotePackage" => {
                        rev.clear();
                        path = e
                            .attributes()
                            .flatten()
                            .find(|a| a.key.as_ref() == b"path")
                            .map(|a| String::from_utf8_lossy(&a.value).to_string())
                            .unwrap_or_default();
                    }
                    // 一个 archive 开始：把上一条的宿主信息清掉，
                    // 否则**没有 host-os 的通用包会继承上一条的**（那就全错位了）
                    "archive" => {
                        os.clear();
                        arch.clear();
                        url.clear();
                        size = 0;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                let v = t.unescape().unwrap_or_default().trim().to_string();
                if v.is_empty() {
                    continue;
                }
                if in_rev && matches!(tag.as_str(), "major" | "minor" | "micro") {
                    rev.push(v.parse().unwrap_or(0));
                    continue;
                }
                match tag.as_str() {
                    "host-os" => os = v,
                    "host-arch" => arch = v,
                    "url" => url = v,
                    "size" => size = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == "revision" {
                    in_rev = false;
                }
                if name == "archive" && !url.is_empty() {
                    out.push(Entry {
                        path: path.clone(),
                        os: os.clone(),
                        arch: arch.clone(),
                        url: url.clone(),
                        size,
                        rev: rev.clone(),
                    });
                }
                tag.clear();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

fn fetch(url: &str) -> Result<String> {
    crate::cli::fix::curl_text(url)
        .ok_or_else(|| TkeError::DeviceError(format!("取不到 {}（网络不通？）", url)))
}

// ── 解压 / 建 AVD ────────────────────────────────────────────────────────

/// 解压时**把全零块留成空洞**，而不是老老实实写一遍零。
///
/// 为什么非做不可：`system.img` 是稀疏镜像——zip 里 600MB，`io::copy` 展开后
/// **在盘上实占 8.1GB**（实测）。对一个卖点是"轻量"的选装件来说，
/// 这个数字直接决定了人愿不愿意装。改成遇到全零块就 seek 过去，
/// ext4 / APFS 上空洞不占盘（NTFS 不会自动稀疏，但也不会比原来更差）。
fn write_sparse<R: std::io::Read>(src: &mut R, dst: &mut std::fs::File) -> Result<()> {
    use std::io::{Seek, SeekFrom, Write};
    let mut buf = vec![0u8; 256 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = src.read(&mut buf).map_err(TkeError::IoError)?;
        if n == 0 {
            break;
        }
        if buf[..n].iter().all(|&b| b == 0) {
            dst.seek(SeekFrom::Current(n as i64)).map_err(TkeError::IoError)?;
        } else {
            dst.write_all(&buf[..n]).map_err(TkeError::IoError)?;
        }
        total += n as u64;
    }
    // seek 出来的空洞要靠 set_len 定住文件真实长度，否则末尾那段空洞会丢
    dst.set_len(total).map_err(TkeError::IoError)?;
    Ok(())
}

fn unzip_into(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path).map_err(TkeError::IoError)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| TkeError::InvalidArgument(format!("不是有效 zip：{}", e)))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| TkeError::InvalidArgument(format!("读取 zip 条目失败：{}", e)))?;
        // 防 zip-slip：enclosed_name 拒绝绝对路径与 `..`
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(TkeError::IoError)?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(TkeError::IoError)?;
        }
        let mut f = std::fs::File::create(&out).map_err(TkeError::IoError)?;
        write_sparse(&mut entry, &mut f)?;
        // zip 不保留可执行位，而 emulator/qemu 那些必须能执行
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if entry.unix_mode().is_some_and(|m| m & 0o111 != 0) {
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755));
            }
        }
    }
    Ok(())
}

/// 建 AVD。**不用 `avdmanager`**（它要 JDK）——AVD 本质就是两个 ini：
/// 一个指路的 `<名字>.ini`，一个描述硬件的 `<名字>.avd/config.ini`
fn create_avd(sdk: &Path, api: u32, abi: &str) -> Result<()> {
    let dir = avd_dir().ok_or_else(|| TkeError::InvalidArgument("找不到用户目录".into()))?;
    let avd = dir.join(format!("{}.avd", AVD_NAME));
    std::fs::create_dir_all(&avd).map_err(TkeError::IoError)?;

    let cpu_arch = if abi == "arm64-v8a" { "arm64" } else { "x86_64" };
    // 屏幕按 1080×2340 / 440dpi——常见中端机的样子。**别用超大屏**：
    // 截图更大、采集更慢，而测的是功能不是分辨率
    let config = format!(
        "avd.ini.encoding=UTF-8\n\
         AvdId={name}\n\
         avd.ini.displayname={name}\n\
         abi.type={abi}\n\
         hw.cpu.arch={cpu}\n\
         image.sysdir.1=system-images/android-{api}/aosp_atd/{abi}/\n\
         tag.id=aosp_atd\n\
         tag.display=AOSP ATD\n\
         image.androidVersion.api={api}\n\
         hw.lcd.width=1080\n\
         hw.lcd.height=2340\n\
         hw.lcd.density=440\n\
         hw.ramSize=2048\n\
         vm.heapSize=256\n\
         disk.dataPartition.size=6G\n\
         hw.keyboard=yes\n\
         hw.gpu.enabled=yes\n\
         hw.gpu.mode=auto\n\
         hw.audioInput=no\n\
         hw.audioOutput=no\n\
         showDeviceFrame=no\n",
        name = AVD_NAME,
        abi = abi,
        cpu = cpu_arch,
        api = api
    );
    std::fs::write(avd.join("config.ini"), config).map_err(TkeError::IoError)?;

    let ini = format!(
        "avd.ini.encoding=UTF-8\npath={}\npath.rel=avd/{}.avd\ntarget=android-{}\n",
        avd.display(),
        AVD_NAME,
        api
    );
    std::fs::write(dir.join(format!("{}.ini", AVD_NAME)), ini).map_err(TkeError::IoError)?;
    let _ = sdk; // 路径已写成相对 SDK 根，emulator 靠 ANDROID_SDK_ROOT 找过去
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 清单解析：每个 archive 的宿主信息**不能串到下一条**（没有 host-os 的通用包会继承）
    #[test]
    fn archives_do_not_leak_host_between_entries() {
        let xml = r#"<r>
          <remotePackage path="emulator">
            <archives>
              <archive><complete><size>10</size><url>a.zip</url></complete>
                <host-os>linux</host-os><host-arch>x64</host-arch></archive>
              <archive><complete><size>20</size><url>b.zip</url></complete></archive>
            </archives>
          </remotePackage></r>"#;
        let got = archives(xml);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].os, "linux");
        assert_eq!(got[1].os, "", "第二条没有 host-os，不该继承第一条的");
    }

    /// 认得出 sys-img 那种带分号的包名
    #[test]
    fn parses_system_image_entries() {
        let xml = r#"<r><remotePackage path="system-images;android-34;aosp_atd;x86_64">
            <archives><archive><complete><size>615</size><url>x86_64-34_r02.zip</url></complete></archive></archives>
          </remotePackage></r>"#;
        let got = archives(xml);
        assert_eq!(got[0].path, "system-images;android-34;aosp_atd;x86_64");
        assert_eq!(got[0].url, "x86_64-34_r02.zip");
    }

    /// 挑包**按 revision，不按出现顺序**——第一版写成"取最后一个"，
    /// 实测装到的是 37.1.11 而不是清单里最新的那个
    #[test]
    fn picks_newest_revision_not_last_entry() {
        let xml = r#"<r><remotePackage path="emulator">
            <revision><major>40</major><minor>1</minor><micro>0</micro></revision>
            <archives><archive><complete><size>1</size><url>new.zip</url></complete>
              <host-os>linux</host-os><host-arch>x64</host-arch></archive></archives>
          </remotePackage>
          <remotePackage path="emulator">
            <revision><major>37</major><minor>1</minor><micro>11</micro></revision>
            <archives><archive><complete><size>2</size><url>old.zip</url></complete>
              <host-os>linux</host-os><host-arch>x64</host-arch></archive></archives>
          </remotePackage></r>"#;
        let all = archives(xml);
        let newest = all.iter().filter(|a| a.os == "linux").max_by_key(|a| a.rev.clone()).unwrap();
        assert_eq!(newest.url, "new.zip", "取的该是 revision 最大的那个,不是最后出现的");
    }

    /// 四个官方平台之外要**明说没有**，而不是给个空结果让人以为是网络问题
    #[test]
    fn unsupported_hosts_are_named() {
        // 只能验当前这台的映射；关键是这张表不为空且 abi 跟着架构走
        if let Some((os, arch)) = host_tags() {
            assert!(!os.is_empty() && !arch.is_empty());
        }
        assert_eq!(image_abi(), if cfg!(target_arch = "aarch64") { "arm64-v8a" } else { "x86_64" });
    }
}
