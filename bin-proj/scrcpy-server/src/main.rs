// scrcpy-server - Rust 实现的 scrcpy WebSocket 服务器
mod protocol;
mod server;
mod adb;

use std::env;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 获取编译时注入的版本号
    let version = env!("BUILD_VERSION");

    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("tke-scrcpy {}", version);
                return;
            }
            "--help" | "-h" => {
                println!("tke-scrcpy {}", version);
                println!();
                println!("Rust 实现的 scrcpy WebSocket 服务器");
                println!();
                println!("用法:");
                println!("  tke-scrcpy              启动服务器");
                println!("  tke-scrcpy --version    显示版本号");
                println!("  tke-scrcpy --help       显示帮助信息");
                println!();
                println!("环境变量:");
                println!("  PORT        服务器端口 (默认: 8000)");
                println!("  ADB_PATH    ADB 可执行文件路径 (默认: 使用系统 ADB)");
                return;
            }
            _ => {
                eprintln!("未知参数: {}", args[1]);
                eprintln!("使用 --help 查看帮助");
                std::process::exit(1);
            }
        }
    }

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tke_scrcpy=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 从环境变量读取配置
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .expect("PORT 必须是有效的端口号");

    let adb_path = env::var("ADB_PATH").ok();

    info!("scrcpy-server 启动中...");
    info!("版本: {}", version);
    info!("监听端口: {}", port);
    if let Some(ref path) = adb_path {
        info!("使用 ADB 路径: {}", path);
    } else {
        info!("使用系统默认 ADB");
    }

    // 启动 WebSocket 服务器
    if let Err(e) = server::start_server(port, adb_path).await {
        error!("服务器启动失败: {}", e);
        std::process::exit(1);
    }
}
