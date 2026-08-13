// 【运行依赖探测】tke 跑起来要哪些外部件、现在在不在。
//
// 两个调用方共用这里，避免各写一套判断标准然后慢慢分叉：
//   - `tke fix`（cli 层）：决定要补什么
//   - 驱动层：操作失败时判断"是不是因为这个没装"，好把报错说到点子上
//
// 约束（不是随便定的）：外部工具必须与 tke **同目录**，不搜 PATH——chromedriver 与 Chrome
// 的版本必须配对，让 PATH 里随便一个 chromedriver 混进来就会出难查的怪问题。

use std::path::{Path, PathBuf};

/// tke 自己所在目录（外部工具都在这儿找）
pub fn tke_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// 某个外部工具在不在 tke 同目录
pub fn tool_present(name: &str) -> bool {
    tke_dir().is_some_and(|d| present_in(&d, name))
}

/// 指定目录里有没有这个工具（`tke fix` 用它检查目标落点）
pub fn present_in(dir: &Path, name: &str) -> bool {
    dir.join(name).is_file() || dir.join(format!("{}.exe", name)).is_file()
}

/// Chrome for Testing 的用户数据目录（与 web 驱动的查找路径保持一致）
pub fn chrome_data_dir() -> Option<PathBuf> {
    let home = PathBuf::from(
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?,
    );
    Some(match std::env::consts::OS {
        "macos" => home.join("Library/Application Support/tke"),
        "windows" => std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or(home)
            .join("tke"),
        _ => std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("tke"),
    })
}

/// Chrome for Testing 官方 zip 解压出来的目录名（按平台）
pub fn chrome_pkg_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "chrome-mac-arm64",
        ("macos", _) => "chrome-mac-x64",
        ("windows", _) => "chrome-win64",
        _ => "chrome-linux64",
    }
}

/// Chrome for Testing 可执行文件的相对路径（官方 zip 解压后的原样结构）。
/// **驱动层的 find_chrome_binary 与 `tke fix` 的检测必须用这同一份**——各写一套的话，
/// 一边认为装好了、另一边找不到，会出很难查的怪事。
pub const CHROME_REL: &[&str] = &[
    #[cfg(target_os = "macos")]
    "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    #[cfg(target_os = "macos")]
    "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    #[cfg(target_os = "windows")]
    "chrome-win64/chrome.exe",
    #[cfg(target_os = "windows")]
    "chrome-win32/chrome.exe",
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    "chrome-linux64/chrome",
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    "chrome-linux/chrome",
];

/// Chrome for Testing 装没装 —— 返回**可执行文件**路径。
///
/// ⚠️ 判据是那个二进制在不在，**不是目录在不在**：解压到一半失败也会留下目录，
/// 只看目录会把"装坏了"当成"装好了"（我自己就这么错过一次）。
///
/// 注意**没装不等于用不了**：chromedriver 还能去找系统 Chrome，只是版本大概率对不上。
/// 所以这个函数只用来把报错说清楚，不用来提前拦人。
pub fn chrome_for_testing_bin() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(d) = tke_dir() {
        roots.push(d);
    }
    if let Some(d) = chrome_data_dir() {
        roots.push(d);
    }
    roots
        .iter()
        .flat_map(|root| CHROME_REL.iter().map(move |rel| root.join(rel)))
        .find(|p| p.exists())
}

/// 缺依赖时统一的指路话术——**只指路，不自己下载**（ADR-0012）：
/// 普通命令里静默拖几百 MB，在内网/离线/CI/按流量计费的机器上都是灾难。
pub fn fix_hint(profile: &str) -> String {
    format!("补齐依赖：tke fix --profile {}", profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_in_finds_both_plain_and_exe() {
        let d = std::env::temp_dir().join(format!("tke-deps-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert!(!present_in(&d, "adb"), "空目录不该有");
        std::fs::write(d.join("adb"), b"x").unwrap();
        assert!(present_in(&d, "adb"));
        std::fs::write(d.join("go-ios.exe"), b"x").unwrap();
        assert!(present_in(&d, "go-ios"), "Windows 的 .exe 也要认");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn chrome_pkg_name_matches_official_zip_layout() {
        let n = chrome_pkg_name();
        assert!(n.starts_with("chrome-"), "要与官方 zip 解压后的目录名一致：{}", n);
        assert!(
            CHROME_REL.iter().any(|r| r.starts_with(n)),
            "包目录名要与可执行文件路径对得上：{} vs {:?}",
            n,
            CHROME_REL
        );
    }
}
