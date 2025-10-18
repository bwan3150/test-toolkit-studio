// build.rs - 构建脚本，自动判断平台并将对应的 ADB 嵌入到可执行文件中
// 同时处理版本号注入

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 1. 处理版本号注入
    inject_version();

    // 2. 处理 ADB 嵌入
    println!("cargo:rerun-if-changed=resources/");
    
    // 获取目标平台信息
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    // 根据目标平台确定 ADB 文件名和平台目录
    let (adb_name, platform_dir) = match target_os.as_str() {
        "windows" => ("adb.exe", "win32"),
        "macos" => ("adb", "darwin"), 
        "linux" => ("adb", "linux"),
        _ => {
            println!("cargo:warning=不支持的目标平台: {}", target_os);
            println!("cargo:rustc-env=TKE_HAS_BUNDLED_ADB=false");
            create_empty_adb_binary();
            return;
        }
    };
    
    let adb_source_path = format!("resources/{}/{}", platform_dir, adb_name);
    
    // 检查对应平台的 ADB 文件是否存在
    if Path::new(&adb_source_path).exists() {
        // 设置环境变量，告诉运行时有内置 ADB
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_ADB=true");
        println!("cargo:rustc-env=TKE_ADB_BINARY_NAME={}", adb_name);
        
        // 读取 ADB 二进制文件
        let adb_bytes = fs::read(&adb_source_path)
            .unwrap_or_else(|e| panic!("无法读取 ADB 二进制文件 {}: {}", adb_source_path, e));
        
        // 生成包含 ADB 二进制数据的 Rust 代码
        let out_dir = env::var("OUT_DIR").unwrap();
        let adb_rs_path = Path::new(&out_dir).join("embedded_adb.rs");
        
        let adb_const = format!(
            "// 自动生成的嵌入式 ADB 二进制数据\n\
             pub const EMBEDDED_ADB_BINARY: &[u8] = &{:?};\n\
             pub const ADB_BINARY_NAME: &str = \"{}\";\n\
             pub const HAS_BUNDLED_ADB: bool = true;",
            adb_bytes,
            adb_name
        );
        
        fs::write(&adb_rs_path, adb_const)
            .unwrap_or_else(|e| panic!("无法写入 ADB 常量文件: {}", e));
        
        println!("✓ ADB 已嵌入: {} -> {} ({} KB)", 
                 adb_source_path, 
                 adb_name,
                 adb_bytes.len() / 1024);
    } else {
        // 没有找到对应平台的 ADB，将回退到系统 ADB
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_ADB=false");
        println!("cargo:warning=平台 {} 的 ADB 未找到: {}，将使用系统 ADB", 
                 platform_dir, adb_source_path);
        
        create_empty_adb_binary();
    }
}

fn create_empty_adb_binary() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let adb_rs_path = Path::new(&out_dir).join("embedded_adb.rs");

    let empty_adb_const =
        "// 空的 ADB 二进制数据 - 将使用系统 ADB\n\
         pub const EMBEDDED_ADB_BINARY: &[u8] = &[];\n\
         pub const ADB_BINARY_NAME: &str = \"adb\";\n\
         pub const HAS_BUNDLED_ADB: bool = false;";

    fs::write(&adb_rs_path, empty_adb_const)
        .expect("无法写入空 ADB 常量文件");
}

/// 注入版本号到编译时环境变量
/// 如果 BUILD_VERSION 环境变量不存在，使用 "unknown" 暴露配置问题
fn inject_version() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = env::var("BUILD_VERSION").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:warning=TKE 版本号: {}", version);
}