// build.rs - 构建脚本：仅处理版本号注入
// ADB/AAPT 不再嵌入，运行时从 tke 同目录查找

use std::env;

fn main() {
    inject_version();
}

/// 注入版本号到编译时环境变量。
///
/// **同时注入 git 短 commit**：版本号(`0.7.4-beta`)整个 beta 期都不变，
/// 于是"这台节点到底装没装上某个修复"谁也说不清 —— 实测撞到过：
/// 用户更新完节点，我这头无法判断他拿到的是哪一版，只能靠猜。
/// 版本号回答"哪个大版本"，commit 回答"具体是哪一次构建"，两个都要。
fn inject_version() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = env::var("BUILD_VERSION").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:warning=TKE 版本号: {}", version);

    // git 短 hash。**拿不到就写 unknown**，不让构建失败 ——
    // 打包环境里没有 .git 是常事（下载 tarball 构建），而版本号本身已经够用
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_COMMIT={}", commit);
    // 改动会换 HEAD，让 cargo 知道要重跑这个脚本
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
