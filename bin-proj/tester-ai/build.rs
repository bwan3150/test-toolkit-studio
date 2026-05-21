// build.rs - 构建脚本，注入版本号

use std::env;

fn main() {
    // 注入版本号到编译时环境变量
    // 如果 BUILD_VERSION 环境变量不存在，使用 "unknown" 暴露配置问题
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = env::var("BUILD_VERSION").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:warning=Tester-AI 版本号: {}", version);
}
