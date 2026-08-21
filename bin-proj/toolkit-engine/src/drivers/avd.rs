// 【安卓模拟器（AVD）】起停 + 发现。**选装能力**（用户拍板 2026-08-21）。
//
// 为什么是选装而不是像 chromedriver 那样补齐：
//   - iOS 模拟器是 macOS **自带**的（Xcode 装了就有），我们只补一个 21MB 的 WDA runner
//   - 安卓**真机**开发者模式很好开，插上就能测——模拟器不是必经之路
//   - 而这套东西很重：`emulator` 包 350~490MB + 一个系统镜像 450~860MB，
//     加起来 1GB 上下。让每个人为一条备选路径先下 1GB，不划算（同 ADR-0012 的精神：
//     不静默拖大包；这里更进一步——**根本不进依赖检查**，没装不算"环境不完整"）
//
// 所以这个文件只做两件事：**认出它装没装**，装了就**能起能停**。没装就如实说、
// 给一行安装命令，绝不代下。
//
// 与 iOS 模拟器的对照（`wda/infra.rs`）：那边要自己管端口（多台会撞 8100，见 Q-13），
// 安卓这边**天生不撞**——每台 AVD 占一个从 5554 起、步进 2 的控制台端口，
// 序列号就是 `emulator-<端口>`，adb 直接分得清。

use std::path::PathBuf;
use std::process::Command;

use crate::{Result, TkeError};

/// 一套安卓 SDK：emulator 二进制 + SDK 根 + AVD 目录。**三样必须配套**。
///
/// 为什么不能只找一个 `emulator` 就完事：机器上可能同时有两套——用户自己的
/// `~/Library/Android/sdk` 和 tke 装的 `~/.tke/android-sdk`。拿我们的 emulator 去跑
/// 他的 AVD 会直接失败：他那台 AVD 的 `config.ini` 里 `image.sysdir.1` 指的是
/// **他那套 SDK 里的**系统镜像，而 emulator 按 `ANDROID_SDK_ROOT` 找镜像——
/// 指错了就是 `Broken AVD system path`。
///
/// 所以按**这台 AVD 属于谁**来选整套工具链，而不是全局挑一个 emulator。
#[derive(Debug, Clone)]
pub struct Toolchain {
    pub emulator: PathBuf,
    pub sdk_root: PathBuf,
    pub avd_home: PathBuf,
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "emulator.exe" } else { "emulator" }
}

/// tke 自己装的那套（`doctor --fix --profile android-emu` 的落点）
fn tke_toolchain() -> Option<Toolchain> {
    let sdk = tke_sdk_dir()?;
    let emulator = sdk.join("emulator").join(exe_name());
    emulator.is_file().then(|| Toolchain {
        emulator,
        avd_home: sdk.join("avd"),
        sdk_root: sdk,
    })
}

/// 用户自己装的那套（Android Studio / sdkmanager 装的）。
/// **不走 ToolManager**：那个只在 tke 同目录找，而这是用户的 SDK 的一部分，
/// 报错说"要和 tke 放在同一目录"会把人引到完全错的方向
fn user_toolchain() -> Option<Toolchain> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(v) = std::env::var_os(var) {
            roots.push(PathBuf::from(v));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        roots.push(h.join("Library/Android/sdk")); // macOS 默认
        roots.push(h.join("Android/Sdk")); // Linux 默认
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Android/Sdk")); // Windows 默认
    }
    let sdk = roots.into_iter().find(|r| r.join("emulator").join(exe_name()).is_file())?;
    Some(Toolchain {
        emulator: sdk.join("emulator").join(exe_name()),
        sdk_root: sdk,
        avd_home: user_avd_home(),
    })
}

/// 用户的 AVD 放哪儿：`ANDROID_AVD_HOME` 优先，否则是标准的 `~/.android/avd`
fn user_avd_home() -> PathBuf {
    if let Some(v) = std::env::var_os("ANDROID_AVD_HOME") {
        return PathBuf::from(v);
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".android").join("avd"))
        .unwrap_or_default()
}

/// 这台 AVD 该用哪套工具链——**谁家的 AVD 用谁家的 SDK**
fn toolchain_for(avd: &str) -> Option<Toolchain> {
    [tke_toolchain(), user_toolchain()]
        .into_iter()
        .flatten()
        .find(|tc| tc.avd_home.join(format!("{}.ini", avd)).is_file())
}

/// 装了 emulator 没有（任意一套都算）。给 doctor 判断"这条路通不通"用
pub fn emulator_bin() -> Option<PathBuf> {
    tke_toolchain()
        .or_else(user_toolchain)
        .map(|t| t.emulator)
        .or_else(|| which::which("emulator").ok())
}

/// tke 自己装的那套 SDK 在哪（与 `cli/android_sdk.rs` 同一口径）
pub fn tke_sdk_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".tke").join("android-sdk"))
        .filter(|d| d.is_dir())
}

/// 把 `-d avd:<名字>` 翻成能直接用的 adb 序列号。**这是唯一的解析口**。
///
/// 起来了 → `emulator-5554`；没起来 → 原样返回（只有 `boot` 用得了它）。
/// 为什么要单独一个函数：第一版只在 `AdbDriver::new` 里解析，于是 `tke device info`
/// 走 `DeviceManager` 那条路时 `avd:tke` 被原样塞进 `adb -s`，报出
/// `adb: unknown host service`——同一个前缀，两条路各解析各的，必然漏
pub fn resolve_device_id(device_id: Option<String>) -> Option<String> {
    match device_id {
        Some(d) if d.starts_with("avd:") => {
            let name = d.trim_start_matches("avd:");
            running_serial_of(name).or(Some(d))
        }
        other => other,
    }
}

/// 把 SDK 目录补成 emulator 认得的样子。
///
/// **emulator 靠 `platform-tools/` 这个子目录判断"这是不是一个 SDK root"**——
/// 没有它就一路往上猜，猜不到就 `FATAL | Broken AVD system path` 直接退出
/// （实测：我们只放了 emulator/ 与 system-images/，日志里连着五行 "please install it"，
/// 而报到人眼前的却是"起了三分钟还没就绪"，完全指不到这儿）。
///
/// adb 我们本来就分发，软链过去即可——**不重复下一份**。
/// 幂等：每次 boot 前都调，老的安装也能自己补上
pub fn ensure_sdk_layout() -> Result<()> {
    let Some(sdk) = tke_sdk_dir() else { return Ok(()) };
    let pt = sdk.join("platform-tools");
    std::fs::create_dir_all(&pt).map_err(TkeError::IoError)?;
    let name = if cfg!(windows) { "adb.exe" } else { "adb" };
    let dst = pt.join(name);
    if dst.exists() {
        return Ok(());
    }
    if let Ok(adb) = crate::ToolManager::resolve("adb") {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&adb, &dst);
        // Windows 上建符号链接要管理员权限，复制更省事（adb.exe 几 MB）
        #[cfg(not(unix))]
        let _ = std::fs::copy(&adb, &dst);
    }
    Ok(())
}

/// 给 emulator 子进程配好这套工具链的环境。
///
/// **三样要一致**：`ANDROID_SDK_ROOT` 决定去哪找系统镜像（AVD 的 `config.ini` 里
/// `image.sysdir.1` 是相对 SDK 根的路径），`ANDROID_AVD_HOME` 决定去哪找 AVD。
/// 早先这里无条件指向 tke 自己那套——于是**一旦装了我们的 SDK，用户自己的 AVD
/// 就跑不了了**（AVD 找不到，就算找到镜像路径也对不上）
fn with_env(cmd: &mut Command, tc: &Toolchain) {
    cmd.env("ANDROID_SDK_ROOT", &tc.sdk_root);
    cmd.env("ANDROID_HOME", &tc.sdk_root);
    cmd.env("ANDROID_AVD_HOME", &tc.avd_home);
}

/// 这台机器上建好的 AVD 名字——**两套 SDK 的都算**（tke 装的 + 用户自己的）。
///
/// 直接扫 `<avd_home>/*.ini` 而不是跑 `emulator -list-avds`：那条命令只看**一个**
/// AVD 目录（取决于环境变量），而这里恰恰要把两边合起来；扫目录也不依赖二进制跑得起来
pub fn list_avds() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tc in [tke_toolchain(), user_toolchain()].into_iter().flatten() {
        for n in avd_names_in(&tc.avd_home) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

/// 一个 AVD 目录里有哪些 AVD（`<名字>.ini` 与同名 `.avd` 目录都在才算数）
fn avd_names_in(dir: &std::path::Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_stem()?.to_str()?.to_string();
            (p.extension()? == "ini" && dir.join(format!("{}.avd", name)).is_dir()).then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// 没装时给人看的一行——**只说怎么装，不代劳**（1GB 级的东西不该由 tke 悄悄下）
pub fn install_hint() -> String {
    "没装安卓模拟器（选装）。装法：Android Studio 里建一台，或命令行 \
     `sdkmanager --install \"emulator\" \"system-images;android-34;aosp_atd;x86_64\"` \
     再 `avdmanager create avd -n tke -k \"system-images;android-34;aosp_atd;x86_64\"`\
     （arm64 机器把 x86_64 换成 arm64-v8a；aosp_atd 是给自动化用的精简镜像，最小）"
        .to_string()
}

/// 起一台 AVD，**等到它真的能用**，返回 adb 序列号（`emulator-5554`）。
///
/// `headed=None` 时按有没有桌面自动定（同 web 的 `--headless=auto`）：
/// 无头服务器上开窗口必然失败，而有桌面时看得见画面对人排查更有用。
pub fn boot(name: &str, headed: Option<bool>) -> Result<String> {
    // **谁家的 AVD 用谁家的 SDK**：拿我们的 emulator 去跑用户的 AVD，
    // 镜像路径必然对不上（他的 config.ini 指的是他那套 SDK 里的 system-images）
    let Some(tc) = toolchain_for(name) else {
        let avds = list_avds();
        return Err(TkeError::InvalidArgument(if avds.is_empty() {
            install_hint()
        } else {
            format!("没有叫 `{}` 的 AVD。现有的：{}", name, avds.join(" / "))
        }));
    };
    // 已经起着就直接用（幂等）——boot 写在脚本第一行，每次跑都会执行
    if let Some(serial) = running_serial_of(name) {
        return Ok(serial);
    }

    // 每次都补一遍：便宜、幂等，而且能让"装的时候还没有 adb"那种安装自己长好
    ensure_sdk_layout()?;

    let before = emulator_serials();
    let headed = headed.unwrap_or_else(crate::utils::params::desktop_available);
    let mut cmd = Command::new(&tc.emulator);
    with_env(&mut cmd, &tc);
    cmd.args(["-avd", name, "-no-audio", "-no-boot-anim", "-no-snapshot-save"]);
    if !headed {
        cmd.arg("-no-window");
        // 无头下用软件渲染：CI / 无桌面机器上没有可用的 GL，`auto` 会起不来。
        //
        // ⚠️ **必须是 `swiftshader` 而不是 `swiftshader_indirect`**（实测，Linux amd64）：
        // indirect 那个能起、能采元素、能点中，**唯独截图是一张纯色图**——
        // emulator 自己的 `emu screenrecord screenshot` 拿到的也一样，
        // 说明合成器只出了背景层、App 的内容层根本没合上去（63KB 的纯色 PNG）。
        // 换成 `swiftshader` 之后同一屏是 1.7MB、壁纸图标状态栏全在。
        //
        // 这个坑特别值得防：**每一步都报成功**，元素采得到、点也点得中，
        // 只有留给人看的那张证据是空的（P-35 那一族）。
        // `TKE_AVD_GPU` 留个口子，换后端不必改代码
        let gpu = std::env::var("TKE_AVD_GPU").unwrap_or_else(|_| "swiftshader".into());
        cmd.args(["-gpu", &gpu]);
    }
    let log = log_path(name);
    let out_file = std::fs::File::create(&log).map_err(TkeError::IoError)?;
    let err_file = out_file.try_clone().map_err(TkeError::IoError)?;
    cmd.stdin(std::process::Stdio::null()).stdout(out_file).stderr(err_file);
    cmd.spawn()
        .map_err(|e| TkeError::DeviceError(format!("启动模拟器失败: {}（日志 {}）", e, log.display())))?;

    // 冷启动很慢（首次能到一两分钟），而**只等 adb 认出设备是不够的**：
    // 那时系统还在起，装 App / 采集都会失败。要等 sys.boot_completed
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    let mut serial = None;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        if serial.is_none() {
            serial = emulator_serials().into_iter().find(|s| !before.contains(s));
        }
        if let Some(s) = &serial {
            if boot_completed(s) {
                return Ok(s.clone());
            }
        }
    }
    // Linux 上起不来，头号嫌疑是 KVM。
    // ⚠️ **判据要跟 emulator 一致**：它不是去 open /dev/kvm，而是**读 `/etc/group` 看
    // 你在不在 kvm 组**（日志原话："The KVM line in /etc/group is: [kvm:x:993:]"）。
    // 所以 `setfacl -m u:$USER:rw /dev/kvm` 那条路它根本不认——而且实测那个 ACL
    // 还会被 systemd-logind 的 uaccess 规则重置掉。要治就得 usermod 进组
    if cfg!(target_os = "linux") && !in_kvm_group() {
        return Err(TkeError::DeviceError(format!(
            "模拟器起不来，多半是没有 KVM 加速：/dev/kvm 打不开（在不在 kvm 组？）。\n\
             \u{3000}修：sudo usermod -aG kvm $USER，然后重新登录。日志：{}",
            log.display()
        )));
    }
    Err(TkeError::DeviceError(format!(
        "模拟器 {} 起了三分钟还没就绪{}。日志：{}",
        name,
        match &serial {
            Some(s) => format!("（adb 已看到 {}，但 sys.boot_completed 一直不是 1）", s),
            None => "（adb 始终没看到它）".to_string(),
        },
        log.display()
    )))
}

/// 关掉一台正在跑的模拟器。`emu kill` 是模拟器自己的控制台命令，比杀进程干净
pub fn shutdown(serial: &str) -> Result<()> {
    if !serial.starts_with("emulator-") {
        return Err(TkeError::InvalidArgument(format!(
            "{} 不是模拟器（真机没法从这儿关机）",
            serial
        )));
    }
    let adb = crate::ToolManager::resolve("adb")?;
    let out = Command::new(adb)
        .args(["-s", serial, "emu", "kill"])
        .output()
        .map_err(|e| TkeError::AdbError(format!("关闭模拟器失败: {}", e)))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(TkeError::DeviceError(format!(
            "关闭 {} 失败：{}",
            serial,
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// 这个 AVD 现在是不是已经跑着了——**按名字找**，因为序列号每次可能不同。
/// `adb -s emulator-NNNN emu avd name` 回的是它的 AVD 名
pub fn running_serial_of(name: &str) -> Option<String> {
    let adb = crate::ToolManager::resolve("adb").ok()?;
    emulator_serials().into_iter().find(|s| {
        Command::new(&adb)
            .args(["-s", s, "emu", "avd", "name"])
            .output()
            .ok()
            .is_some_and(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == name))
    })
}

/// 现在 adb 看得到的模拟器序列号
fn emulator_serials() -> Vec<String> {
    let Ok(adb) = crate::ToolManager::resolve("adb") else { return Vec::new() };
    let Ok(out) = Command::new(adb).arg("devices").output() else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            match (it.next(), it.next()) {
                (Some(id), Some("device")) if id.starts_with("emulator-") => Some(id.to_string()),
                _ => None,
            }
        })
        .collect()
}

/// 当前用户在不在 `kvm` 组——**按 emulator 的判据来**，不是看能不能 open /dev/kvm。
/// 这两件事会分叉：`setfacl` 给了 ACL 时 open 得到、emulator 却仍然拒绝
fn in_kvm_group() -> bool {
    let Ok(groups) = std::fs::read_to_string("/etc/group") else { return true };
    let Some(user) = std::env::var("USER").ok().or_else(|| std::env::var("LOGNAME").ok()) else {
        return true; // 判断不了就别拦路，让 emulator 自己去报
    };
    groups
        .lines()
        .find(|l| l.starts_with("kvm:"))
        .is_some_and(|l| l.rsplit(':').next().is_some_and(|m| m.split(',').any(|g| g == user)))
}

fn boot_completed(serial: &str) -> bool {
    let Ok(adb) = crate::ToolManager::resolve("adb") else { return false };
    Command::new(adb)
        .args(["-s", serial, "shell", "getprop", "sys.boot_completed"])
        .output()
        .ok()
        .is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
}

fn log_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("tke").join("avd");
    let _ = std::fs::create_dir_all(&dir);
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    dir.join(format!("{}.log", safe))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 没装 SDK 时**不能报错、更不能 panic**——它是选装的，"没有"是正常状态
    #[test]
    fn missing_sdk_is_not_an_error() {
        // 这台机器上装没装都行：list_avds 都必须安静地返回一个列表
        let _ = list_avds();
    }

    /// AVD 目录里要 `<名字>.ini` **和**同名 `.avd` 目录都在才算数——
    /// 只有一个 ini 的多半是删剩下的残骸，列出来会让人去启动一台根本起不来的
    #[test]
    fn avd_needs_both_ini_and_dir() {
        let base = std::env::temp_dir().join(format!("tke-avdtest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("good.ini"), "x").unwrap();
        std::fs::create_dir_all(base.join("good.avd")).unwrap();
        std::fs::write(base.join("orphan.ini"), "x").unwrap(); // 只有 ini
        std::fs::create_dir_all(base.join("nodir.avd")).unwrap(); // 只有目录

        assert_eq!(avd_names_in(&base), vec!["good".to_string()]);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 关真机要说清楚，别让人以为命令没生效
    #[test]
    fn shutdown_rejects_real_device() {
        let e = shutdown("R5CT30XXXXX").unwrap_err();
        assert!(e.to_string().contains("不是模拟器"), "{}", e);
    }

    /// 安装提示里必须有那条能直接粘的命令，否则等于只说了"你没装"
    #[test]
    fn install_hint_carries_a_command() {
        let h = install_hint();
        assert!(h.contains("sdkmanager"), "{}", h);
        assert!(h.contains("aosp_atd"), "{}", h);
    }
}
