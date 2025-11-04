// scrcpy-server - Rust 实现的 scrcpy WebSocket 服务器
mod protocol;
mod server;
mod adb;

use std::env;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
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
