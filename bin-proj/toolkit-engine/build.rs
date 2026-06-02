// build.rs - 构建脚本：仅处理版本号注入
// ADB/AAPT 不再嵌入，运行时从 tke 同目录查找

use std::env;

fn main() {
    inject_version();
}

/// 注入版本号到编译时环境变量
fn inject_version() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = env::var("BUILD_VERSION").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:warning=TKE 版本号: {}", version);
}
