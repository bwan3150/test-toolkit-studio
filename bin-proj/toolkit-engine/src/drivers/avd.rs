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

/// 找 Android SDK 里的 `emulator` 二进制。**不走 ToolManager**：
/// 那个只在 tke 同目录找，而这东西是用户自己装的 SDK 的一部分，
/// 报错文案也不该说"要和 tke 放在同一目录"（会把人引到错的方向）
pub fn emulator_bin() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "emulator.exe" } else { "emulator" };
    let mut roots: Vec<PathBuf> = Vec::new();
    // **我们自己装的那套优先**（`tke doctor --fix --profile android-emu` 的落点）：
    // 版本是我们挑的、AVD 是我们建的，比去猜用户 SDK 里有什么确定得多
    if let Some(d) = tke_sdk_dir() {
        roots.push(d);
    }
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
    for r in roots {
        let p = r.join("emulator").join(exe);
        if p.is_file() {
            return Some(p);
        }
    }
    which::which("emulator").ok()
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

/// 给 emulator 子进程配好环境。
///
/// **两个都要**：`ANDROID_SDK_ROOT` 让它找到系统镜像（config.ini 里写的是相对 SDK 根的
/// 路径），`ANDROID_AVD_HOME` 让它找到我们建的 AVD——我们**有意不往 `~/.android/avd`
/// 写**（那是用户的地盘，卸载时不好界定该删哪些）
fn with_env(cmd: &mut Command) {
    if let Some(sdk) = tke_sdk_dir() {
        cmd.env("ANDROID_SDK_ROOT", &sdk);
        cmd.env("ANDROID_HOME", &sdk);
        cmd.env("ANDROID_AVD_HOME", sdk.join("avd"));
    }
}

/// 这台机器上建好的 AVD 名字。没装 SDK / 一个都没建 → 空
pub fn list_avds() -> Vec<String> {
    let Some(bin) = emulator_bin() else { return Vec::new() };
    let mut cmd = Command::new(&bin);
    with_env(&mut cmd);
    let Ok(out) = cmd.arg("-list-avds").output() else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        // 这条命令偶尔会往 stdout 混一行提示（比如 SDK 路径警告），只收像名字的
        .filter(|l| !l.is_empty() && !l.contains(' ') && !l.starts_with('#'))
        .collect()
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
    let bin = emulator_bin().ok_or_else(|| TkeError::InvalidArgument(install_hint()))?;
    let avds = list_avds();
    if !avds.iter().any(|a| a == name) {
        return Err(TkeError::InvalidArgument(format!(
            "没有叫 `{}` 的 AVD。现有的：{}",
            name,
            if avds.is_empty() { "一个都没有".into() } else { avds.join(" / ") }
        )));
    }
    // 已经起着就直接用（幂等）——boot 写在脚本第一行，每次跑都会执行
    if let Some(serial) = running_serial_of(name) {
        return Ok(serial);
    }

    // 每次都补一遍：便宜、幂等，而且能让"装的时候还没有 adb"那种安装自己长好
    ensure_sdk_layout()?;

    let before = emulator_serials();
    let headed = headed.unwrap_or_else(crate::utils::params::desktop_available);
    let mut cmd = Command::new(&bin);
    with_env(&mut cmd);
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
