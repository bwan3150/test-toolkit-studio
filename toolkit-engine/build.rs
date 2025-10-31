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
    let (adb_name, aapt_name, ffmpeg_name, platform_dir) = match target_os.as_str() {
        "windows" => ("adb.exe", "aapt.exe", "ffmpeg.exe", "win32"),
        "macos" => ("adb", "aapt", "ffmpeg", "darwin"),
        "linux" => ("adb", "aapt", "ffmpeg", "linux"),
        _ => {
            println!("cargo:warning=不支持的目标平台: {}", target_os);
            println!("cargo:rustc-env=TKE_HAS_BUNDLED_ADB=false");
            println!("cargo:rustc-env=TKE_HAS_BUNDLED_AAPT=false");
            println!("cargo:rustc-env=TKE_HAS_BUNDLED_FFMPEG=false");
            create_empty_adb_binary();
            create_empty_aapt_binary();
            create_empty_ffmpeg_binary();
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

    // 3. 处理 AAPT 嵌入
    let aapt_source_path = format!("resources/{}/{}", platform_dir, aapt_name);

    // 检查对应平台的 AAPT 文件是否存在
    if Path::new(&aapt_source_path).exists() {
        // 设置环境变量，告诉运行时有内置 AAPT
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_AAPT=true");
        println!("cargo:rustc-env=TKE_AAPT_BINARY_NAME={}", aapt_name);

        // 读取 AAPT 二进制文件
        let aapt_bytes = fs::read(&aapt_source_path)
            .unwrap_or_else(|e| panic!("无法读取 AAPT 二进制文件 {}: {}", aapt_source_path, e));

        // 生成包含 AAPT 二进制数据的 Rust 代码
        let out_dir = env::var("OUT_DIR").unwrap();
        let aapt_rs_path = Path::new(&out_dir).join("embedded_aapt.rs");

        let aapt_const = format!(
            "// 自动生成的嵌入式 AAPT 二进制数据\n\
             pub const EMBEDDED_AAPT_BINARY: &[u8] = &{:?};\n\
             pub const AAPT_BINARY_NAME: &str = \"{}\";\n\
             pub const HAS_BUNDLED_AAPT: bool = true;",
            aapt_bytes,
            aapt_name
        );

        fs::write(&aapt_rs_path, aapt_const)
            .unwrap_or_else(|e| panic!("无法写入 AAPT 常量文件: {}", e));

        println!("✓ AAPT 已嵌入: {} -> {} ({} KB)",
                 aapt_source_path,
                 aapt_name,
                 aapt_bytes.len() / 1024);
    } else {
        // 没有找到对应平台的 AAPT，将回退到系统 AAPT
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_AAPT=false");
        println!("cargo:warning=平台 {} 的 AAPT 未找到: {}，将使用系统 AAPT",
                 platform_dir, aapt_source_path);

        create_empty_aapt_binary();
    }

    // 4. 处理 ffmpeg 嵌入
    let ffmpeg_source_path = format!("resources/{}/{}", platform_dir, ffmpeg_name);

    // 检查对应平台的 ffmpeg 文件是否存在
    if Path::new(&ffmpeg_source_path).exists() {
        // 设置环境变量，告诉运行时有内置 ffmpeg
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_FFMPEG=true");
        println!("cargo:rustc-env=TKE_FFMPEG_BINARY_NAME={}", ffmpeg_name);

        // 读取 ffmpeg 二进制文件
        let ffmpeg_bytes = fs::read(&ffmpeg_source_path)
            .unwrap_or_else(|e| panic!("无法读取 ffmpeg 二进制文件 {}: {}", ffmpeg_source_path, e));

        // 生成包含 ffmpeg 二进制数据的 Rust 代码
        let out_dir = env::var("OUT_DIR").unwrap();
        let ffmpeg_rs_path = Path::new(&out_dir).join("embedded_ffmpeg.rs");

        let ffmpeg_const = format!(
            "// 自动生成的嵌入式 ffmpeg 二进制数据\n\
             pub const EMBEDDED_FFMPEG_BINARY: &[u8] = &{:?};\n\
             pub const FFMPEG_BINARY_NAME: &str = \"{}\";\n\
             pub const HAS_BUNDLED_FFMPEG: bool = true;",
            ffmpeg_bytes,
            ffmpeg_name
        );

        fs::write(&ffmpeg_rs_path, ffmpeg_const)
            .unwrap_or_else(|e| panic!("无法写入 ffmpeg 常量文件: {}", e));

        println!("✓ ffmpeg 已嵌入: {} -> {} ({} KB)",
                 ffmpeg_source_path,
                 ffmpeg_name,
                 ffmpeg_bytes.len() / 1024);
    } else {
        // 没有找到对应平台的 ffmpeg，将回退到系统 ffmpeg
        println!("cargo:rustc-env=TKE_HAS_BUNDLED_FFMPEG=false");
        println!("cargo:warning=平台 {} 的 ffmpeg 未找到: {}，将使用系统 ffmpeg",
                 platform_dir, ffmpeg_source_path);

        create_empty_ffmpeg_binary();
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

fn create_empty_aapt_binary() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let aapt_rs_path = Path::new(&out_dir).join("embedded_aapt.rs");

    let empty_aapt_const =
        "// 空的 AAPT 二进制数据 - 将使用系统 AAPT\n\
         pub const EMBEDDED_AAPT_BINARY: &[u8] = &[];\n\
         pub const AAPT_BINARY_NAME: &str = \"aapt\";\n\
         pub const HAS_BUNDLED_AAPT: bool = false;";

    fs::write(&aapt_rs_path, empty_aapt_const)
        .expect("无法写入空 AAPT 常量文件");
}

fn create_empty_ffmpeg_binary() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let ffmpeg_rs_path = Path::new(&out_dir).join("embedded_ffmpeg.rs");

    let empty_ffmpeg_const =
        "// 空的 ffmpeg 二进制数据 - 将使用系统 ffmpeg\n\
         pub const EMBEDDED_FFMPEG_BINARY: &[u8] = &[];\n\
         pub const FFMPEG_BINARY_NAME: &str = \"ffmpeg\";\n\
         pub const HAS_BUNDLED_FFMPEG: bool = false;";

    fs::write(&ffmpeg_rs_path, empty_ffmpeg_const)
        .expect("无法写入空 ffmpeg 常量文件");
}

/// 注入版本号到编译时环境变量
/// 如果 BUILD_VERSION 环境变量不存在，使用 "unknown" 暴露配置问题
fn inject_version() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");

    let version = env::var("BUILD_VERSION").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_VERSION={}", version);
    println!("cargo:warning=TKE 版本号: {}", version);
}