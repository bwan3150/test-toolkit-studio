// 【设备发现】这台机器现在能测什么——一条命令看全，直接给出 `-d` 该填什么。
//
// 为什么单独一个文件：`tools/device.rs` 回答的是"**这台**设备什么情况"（型号/电池/属性），
// 这里回答的是"**有哪些**设备"。两件事，混在一起那个文件也早该拆了。
//
// ⚠️ **查不了要说出来**（INV-9）：没装 adb 时安卓那栏是空的，而"没装工具"和"没连手机"
// 在结果上长得一模一样。人看到空列表只会去插拔数据线，不会想到是缺依赖。

use std::process::Command;

/// 一个可测目标
#[derive(Debug, serde::Serialize)]
pub struct Target {
    /// `-d` 直接填这个
    pub id: String,
    /// android / ios / ios-sim / web
    pub kind: &'static str,
    /// 给人看的名字（机型/浏览器）
    pub name: String,
    /// 状态：安卓的 device/offline、模拟器的 Booted/Shutdown
    pub state: String,
}

/// 某一类没查成的原因（缺工具 / 平台不支持）——**必须跟结果一起返回**
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

pub fn discover() -> Discovery {
    let mut d = Discovery::default();
    // 浏览器：不需要连什么，装了 chromedriver 就能跑
    d.targets.push(Target {
        id: "web".into(),
        kind: "web",
        name: "浏览器（无头）".into(),
        state: "ready".into(),
    });
    android(&mut d);
    ios_devices(&mut d);
    ios_simulators(&mut d);
    d
}

fn android(d: &mut Discovery) {
    let Ok(adb) = crate::ToolManager::resolve("adb") else {
        d.skipped.push(Skipped {
            kind: "android",
            why: "没装 adb —— 安卓设备是「没查」，不是「没连」。补齐：tke doctor --fix".into(),
        });
        return;
    };
    let Ok(out) = Command::new(&adb).args(["devices", "-l"]).output() else {
        d.skipped.push(Skipped { kind: "android", why: "adb devices 执行失败".into() });
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
        d.targets.push(Target {
            id: serial.to_string(),
            kind: "android",
            name: model.replace('_', " "),
            state: state.to_string(),
        });
    }
}

fn ios_devices(d: &mut Discovery) {
    if !crate::utils::capability::ios_supported() {
        d.skipped.push(Skipped {
            kind: "ios",
            why: "iOS 只能在 macOS 上测（设备端 WDA 依赖 Xcode）".into(),
        });
        return;
    }
    let Ok(go_ios) = crate::ToolManager::resolve("go-ios") else {
        d.skipped.push(Skipped {
            kind: "ios",
            why: "没装 go-ios —— iOS 真机是「没查」，不是「没连」。补齐：tke doctor --fix --profile ios".into(),
        });
        return;
    };
    let Ok(out) = Command::new(&go_ios).args(["list", "--nojson"]).output() else {
        d.skipped.push(Skipped { kind: "ios", why: "go-ios list 执行失败".into() });
        return;
    };
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let udid = line.trim();
        // 只认真机 UDID 的形状，把 go-ios 的提示行滤掉
        if udid.len() == 25 || udid.len() == 40 {
            d.targets.push(Target {
                id: udid.to_string(),
                kind: "ios",
                name: "iOS 真机".into(),
                state: "connected".into(),
            });
        }
    }
}

/// 模拟器：走 `simctl`，**跟真机完全是两条路**——没有 USB、不需要 go-ios 隧道
fn ios_simulators(d: &mut Discovery) {
    if !cfg!(target_os = "macos") {
        return; // 上面 ios_devices 已经说过"只有 macOS"，这儿不重复刷屏
    }
    let out = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available", "--json"])
        .output();
    let Ok(out) = out else {
        d.skipped.push(Skipped { kind: "ios-sim", why: "xcrun simctl 执行失败（装了 Xcode 吗）".into() });
        return;
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        d.skipped.push(Skipped { kind: "ios-sim", why: "simctl 输出解析不了".into() });
        return;
    };
    // 列得出来 ≠ 操作得了：模拟器的点击/采集要靠 idb，没装就先说清楚
    if crate::ToolManager::resolve("idb").is_err() && which::which("idb").is_err() {
        d.skipped.push(Skipped {
            kind: "ios-sim",
            why: "模拟器列得出来但操作不了：没装 idb。\n\u{3000}brew tap facebook/fb && brew trust facebook/fb\n\u{3000}brew install idb-companion && pip3 install fb-idb"
                .into(),
        });
    }
    let Some(runtimes) = v["devices"].as_object() else { return };
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
            d.targets.push(Target {
                // **必须带 sim: 前缀**：模拟器 UDID 是标准 UUID（36 位），
                // 而 tke 认 iOS 靠的是真机 UDID 的形状（25 位），不加前缀会被当成安卓序列号
                id: format!("sim:{}", udid),
                kind: "ios-sim",
                name: format!("{}（{}）", name, ver),
                state: dev["state"].as_str().unwrap_or("?").to_string(),
            });
        }
    }
}
