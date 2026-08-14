// 【宿主机能力门禁】这台机器能驱动哪些平台的设备。
//
// 存在理由：让人在**注定跑不通的组合**上少花时间。最典型的是 Windows/Linux 上测 iOS——
// 撞过去只会得到一串 go-ios 的底层报错，看半天也不知道是自己配错了还是本来就不行。
//
// ⚠️ 这条界线是**产品决策，不是技术极限**，得说清楚：
//   go-ios 本身是跨平台的，运行期也不需要 Xcode（它经 testmanagerd 拉起设备上的 WDA）。
//   真正卡住的是**一次性前置**：设备上那个 WebDriverAgent App 必须先用 Xcode 装一次，
//   而 Xcode 只有 macOS 有。所以「没有 mac 就搞不定 iOS」在实践上成立，
//   但「WDA 已经装好的设备接到 Linux CI 上」这种场景技术上是通的。
//   为此留了 `TKE_ALLOW_IOS=1` 逃生口——默认干净，真有需要的人不至于被堵死。

use crate::{Platform, Result, TkeError};

/// 放行 iOS 的逃生口（见上）
const ALLOW_IOS_ENV: &str = "TKE_ALLOW_IOS";

/// 这台宿主机能不能驱动 `platform` 的设备；不能就返回一条**说清为什么、以及能做什么**的错误。
pub fn check(platform: Platform) -> Result<()> {
    match platform {
        Platform::Ios if !ios_supported() => {
            // 用数组拼而不是 `\n\` 续行：续行符后面的缩进空白会原样进字符串，
            // 用全角空格对齐还会引来编译器警告，写法本身就有歧义
            let lines = [
                format!("iOS 检查只能在 macOS 上做（当前是 {}）。", os_label()),
                "  原因：设备上的 WebDriverAgent 必须先用 Xcode 装一次，而 Xcode 只有 macOS 有。".into(),
                "  （运行期本身不需要 Xcode——go-ios 经 testmanagerd 拉起 WDA。所以如果这台设备".into(),
                format!("    已经在别处装好了 WDA，可以设 {}=1 放行，风险自负。）", ALLOW_IOS_ENV),
                "  这台机器可以做：网页（-d web）、安卓（-d <序列号>）".into(),
            ];
            Err(TkeError::InvalidArgument(lines.join("\n")))
        }
        _ => Ok(()),
    }
}

/// 本机是否放行 iOS
pub fn ios_supported() -> bool {
    cfg!(target_os = "macos") || std::env::var_os(ALLOW_IOS_ENV).is_some()
}

/// 本机能测的平台（给向导/`list_devices`/帮助文案用，别把做不到的选项摆出来让人选）
pub fn supported_platforms() -> Vec<Platform> {
    let mut v = vec![Platform::Web, Platform::Android];
    if ios_supported() {
        v.push(Platform::Ios);
    }
    v
}

fn os_label() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// web/android 在哪儿都放行——它们缺依赖时由驱动层报，不该在这一层拦
    #[test]
    fn web_and_android_always_allowed() {
        assert!(check(Platform::Web).is_ok());
        assert!(check(Platform::Android).is_ok());
    }

    /// iOS 按宿主机分：mac 放行，其余拒绝且报错要说清原因与替代做法
    #[test]
    fn ios_gated_by_host_os() {
        let r = check(Platform::Ios);
        if cfg!(target_os = "macos") {
            assert!(r.is_ok(), "macOS 上应放行 iOS");
        } else {
            let e = r.unwrap_err().to_string();
            assert!(e.contains("macOS"), "要说清只能在 macOS 上做：{}", e);
            assert!(e.contains("Xcode"), "要说清卡在哪：{}", e);
            assert!(e.contains("web"), "要告诉他这台机器能做什么：{}", e);
            assert!(e.contains(ALLOW_IOS_ENV), "要给出逃生口：{}", e);
        }
    }

    /// 逃生口生效（技术上可行的场景不该被堵死）
    #[test]
    fn escape_hatch_allows_ios() {
        // 注：环境变量是进程级的，这个测试与 ios_gated_by_host_os 可能并行跑，
        // 所以只验函数对环境变量的反应，不改全局状态去验 check()
        if cfg!(not(target_os = "macos")) {
            assert!(!ios_supported() || std::env::var_os(ALLOW_IOS_ENV).is_some());
        }
    }

    /// 非 mac 上不该把 iOS 摆进可选平台
    #[test]
    fn supported_platforms_reflects_host() {
        let v = supported_platforms();
        assert!(v.contains(&Platform::Web) && v.contains(&Platform::Android));
        assert_eq!(v.contains(&Platform::Ios), ios_supported());
    }
}
