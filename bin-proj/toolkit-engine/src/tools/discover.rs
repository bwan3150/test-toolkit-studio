// 【设备发现】这台机器现在能测什么——一条命令看全，直接给出 `-d` 该填什么。
//
// 为什么单独一个文件：`tools/device.rs` 回答的是"**这台**设备什么情况"（型号/电池/属性），
// 这里回答的是"**有哪些**设备"。两件事，混在一起那个文件也早该拆了。
//
// ⚠️ **查不了要说出来**（INV-9）：没装 adb 时安卓那栏是空的，而"没装工具"和"没连手机"
// 在结果上长得一模一样。人看到空列表只会去插拔数据线，不会想到是缺依赖。

use std::process::Command;

/// 一个可测目标。四列：`id` / `os` / `model` / `state`——
/// 拆成四列是因为混在一起没法对齐（`CPH2305` 与 `iPhone 17 Pro · iOS 26.2` 长度差太多）
#[derive(Debug, serde::Serialize)]
pub struct Target {
    /// **`-d` 直接填这个**（浏览器有头那行连参数一起给，复制就能用）
    pub id: String,
    /// android / ios / ios-sim / web —— **机器读的**（JSON 输出、脚本判断用）
    pub kind: &'static str,
    /// 系统：`Android 15` / `iOS 26.2` / `—`
    pub os: String,
    /// 机型：`CPH2305` / `iPhone 17 Pro` / `Chrome 无头`
    pub model: String,
    /// 状态：`已连接` / `已启动` / `未启动` / `可用`
    pub state: String,
    /// **能不能马上用**。false = 这一行整行置灰（没启动的模拟器、离线的安卓）
    pub ready: bool,
}

/// 某一类没查成的原因（缺工具 / 平台不支持）——**必须跟结果一起返回**。
///
/// 措辞形如「安卓未检测 · 缺 adb · tke doctor --fix」：**事实 + 下一步，不解释为什么**。
/// 「没查」与「没连」的区别靠"未检测"三个字带出来就够了，不必展开成一句话——
/// CLI 不是教程，多一句解释就是每次都要重读一遍的噪音
#[derive(Debug, serde::Serialize)]
pub struct Skipped {
    pub kind: &'static str,
    pub why: String,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct Discovery {
    pub targets: Vec<Target>,
    pub skipped: Vec<Skipped>,
}

/// 默认只列**在跑的**。装了 Xcode 的 mac 上模拟器动辄二三十台，
/// 全摆出来那份清单就没法看了（实测用户机器：24 台，只有 1 台 Booted）。
pub fn discover() -> Discovery {
    discover_with(false)
}

/// `all=true` 连没启动的模拟器一起列（要挑一台来启动时用）
pub fn discover_with(all: bool) -> Discovery {
    let mut d = Discovery::default();
    // 浏览器：不需要连什么，装了 chromedriver 就能跑。
    // **有头/无头是两种用法**，都列出来——第一列连参数一起给，复制就能用
    d.targets.push(Target {
        id: "web".into(),
        kind: "web",
        os: "—".into(),
        model: "Chrome 无头".into(),
        state: "可用".into(),
        ready: true,
    });
    // 这台机器开不了窗口就别摆有头那行——选了必然失败的选项不该出现（同 iOS 的门禁）
    if crate::utils::params::desktop_available() {
        d.targets.push(Target {
            id: "web --headless=off".into(),
            kind: "web",
            os: "—".into(),
            model: "Chrome 有窗口".into(),
            state: "可用".into(),
            ready: true,
        });
    }
    android(&mut d);
    android_avds(&mut d, all);
    ios_devices(&mut d);
    ios_simulators(&mut d, all);
    d
}

/// 还没启动的 AVD（安卓模拟器）。**选装**：没装 SDK 就一个字都不说——
/// 那不是"缺依赖"，是这条路本来就不必走（真机插上即用）。
///
/// 已经跑着的模拟器由 `android()` 从 `adb devices` 里就列出来了（序列号 `emulator-5554`），
/// 这里只补"能起但没起"的那些，id 给成 `avd:<名字>`——那正是 `boot` 要的参数
fn android_avds(d: &mut Discovery, all: bool) {
    let avds = crate::drivers::avd::list_avds();
    if avds.is_empty() {
        return;
    }
    // 名字对得上的说明已经在跑（上面 android() 列过了），不重复
    let idle: Vec<String> = avds
        .into_iter()
        .filter(|n| crate::drivers::avd::running_serial_of(n).is_none())
        .collect();
    if idle.is_empty() {
        return;
    }
    let running = d.targets.iter().any(|t| t.id.starts_with("emulator-"));
    // 有在跑的就只提一句还有几台闲着（同 iOS 模拟器的处理），别刷屏
    if running && !all {
        d.skipped.push(Skipped {
            kind: "android-avd",
            why: format!("{} 台 AVD 未启动 · --all", idle.len()),
        });
        return;
    }
    for name in idle {
        d.targets.push(Target {
            // **带 avd: 前缀**：序列号是起来之后才有的，起之前只能按名字说
            id: format!("avd:{}", name),
            kind: "android",
            os: "Android".into(),
            model: name,
            state: "未启动".into(),
            ready: false,
        });
    }
}

fn android(d: &mut Discovery) {
    let Ok(adb) = crate::ToolManager::resolve("adb") else {
        d.skipped.push(Skipped {
            kind: "android",
            why: "安卓未检测 · 缺 adb · tke doctor --fix".into(),
        });
        return;
    };
    let Ok(out) = Command::new(&adb).args(["devices", "-l"]).output() else {
        d.skipped.push(Skipped { kind: "android", why: "安卓未检测 · adb devices 失败".into() });
        return;
    };
    for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let (Some(serial), Some(state)) = (it.next(), it.next()) else { continue };
        // `model:Pixel_7` 这种键值对里挑机型出来——序列号对人来说没有意义
        let model = line
            .split_whitespace()
            .find_map(|kv| kv.strip_prefix("model:"))
            .unwrap_or("Android 设备");
        // 系统版本要单独问一次（`adb devices -l` 不给）。离线设备问不到，留空
        let os = if state == "device" {
            Command::new(&adb)
                .args(["-s", serial, "shell", "getprop", "ro.build.version.release"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|v| !v.is_empty())
                .map(|v| format!("Android {}", v))
                .unwrap_or_else(|| "Android".into())
        } else {
            "Android".into()
        };
        d.targets.push(Target {
            id: serial.to_string(),
            kind: "android",
            os,
            model: model.replace('_', " "),
            state: if state == "device" { "已连接".into() } else { state.to_string() },
            ready: state == "device",
        });
    }
}

fn ios_devices(d: &mut Discovery) {
    if !crate::utils::capability::ios_supported() {
        d.skipped.push(Skipped {
            kind: "ios",
            why: "iOS 未检测 · 需 macOS".into(),
        });
        return;
    }
    let Ok(go_ios) = crate::ToolManager::resolve("go-ios") else {
        d.skipped.push(Skipped {
            kind: "ios",
            why: "iOS 真机未检测 · 缺 go-ios · tke doctor --fix --profile ios".into(),
        });
        return;
    };
    let Ok(out) = Command::new(&go_ios).args(["list", "--nojson"]).output() else {
        d.skipped.push(Skipped { kind: "ios", why: "iOS 真机未检测 · go-ios list 失败".into() });
        return;
    };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let udid = line.trim();
        // 只认真机 UDID 的形状，把 go-ios 的提示行滤掉
        if udid.len() == 25 || udid.len() == 40 {
            d.targets.push(Target {
                id: udid.to_string(),
                kind: "ios",
                os: "iOS".into(),
                model: "iPhone / iPad".into(),
                state: "已连接".into(),
                ready: true,
            });
        }
    }

}

/// 模拟器：走 `simctl`，**跟真机完全是两条路**——没有 USB、不需要 go-ios 隧道
fn ios_simulators(d: &mut Discovery, all: bool) {
    if !cfg!(target_os = "macos") {
        return; // 上面 ios_devices 已经说过"只有 macOS"，这儿不重复刷屏
    }
    let out = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available", "--json"])
        .output();
    let Ok(out) = out else {
        d.skipped.push(Skipped { kind: "ios-sim", why: "模拟器未检测 · simctl 失败".into() });
        return;
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        d.skipped.push(Skipped { kind: "ios-sim", why: "模拟器未检测 · simctl 输出解析失败".into() });
        return;
    };
    // 列得出来 ≠ 操作得了：模拟器的点击/采集要靠 WebDriverAgent，没有就先说清楚
    let has_wda = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".tke/wda/WebDriverAgentRunner-Runner.app"))
        .is_some_and(|p| p.exists());
    if !has_wda {
        d.skipped.push(Skipped {
            kind: "ios-sim",
            why: "模拟器操作不了 · 缺 WebDriverAgent · tke doctor --fix --profile ios".into(),
        });
    }
    let Some(runtimes) = v["devices"].as_object() else { return };
    // 没启动的先攒着：**一台在跑的都没有**时才摆出来——否则用户看到空列表
    // 会以为"这台机器不支持模拟器"，而真相只是"都关着"
    let mut idle: Vec<Target> = Vec::new();
    for (runtime, list) in runtimes {
        // "com.apple.CoreSimulator.SimRuntime.iOS-26-0" → "iOS 26.0"
        let ver = runtime
            .rsplit('.')
            .next()
            .map(|s| s.replacen('-', " ", 1).replace('-', "."))
            .unwrap_or_else(|| runtime.clone());
        for dev in list.as_array().into_iter().flatten() {
            let (Some(udid), Some(name)) = (dev["udid"].as_str(), dev["name"].as_str()) else {
                continue;
            };
            let booted = dev["state"].as_str() == Some("Booted");
            let t = Target {
                // **必须带 sim: 前缀**：模拟器 UDID 是标准 UUID（36 位），
                // 而 tke 认 iOS 靠的是真机 UDID 的形状（25 位），不加前缀会被当成安卓序列号
                id: format!("sim:{}", udid),
                kind: "ios-sim",
                os: ver.trim().to_string(),
                model: name.to_string(),
                state: if booted { "已启动".into() } else { "未启动".into() },
                ready: booted,
            };
            if all || t.ready {
                d.targets.push(t);
            } else {
                idle.push(t);
            }
        }
    }

    // 一台在跑的都没有 → 把关着的也列出来（不然等于告诉人"没有模拟器"）；
    // 有在跑的 → 只提一句还有多少台关着,别刷屏
    if !idle.is_empty() {
        let booted = d.targets.iter().any(|t| t.kind == "ios-sim");
        if booted {
            d.skipped.push(Skipped {
                kind: "ios-sim",
                why: format!("{} 台模拟器未启动 · --all", idle.len()),
            });
        } else {
            d.targets.extend(idle);
        }
    }
}
