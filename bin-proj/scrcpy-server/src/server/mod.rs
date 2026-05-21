// WebSocket 服务器模块
mod websocket;
mod proxy;

use anyhow::{Result, Context};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_hdr_async;
use tungstenite::handshake::server::{Request, Response};
use tracing::{info, error, debug};
use url::Url;

use crate::adb::AdbUtils;

/// 启动 WebSocket 服务器
pub async fn start_server(port: u16, adb_path: Option<String>) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(&addr)
        .await
        .context("无法绑定端口")?;

    info!("✅ Listening on: http://{}", addr);

    // 创建 ADB 工具实例
    let adb_utils = std::sync::Arc::new(AdbUtils::new(adb_path));

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("新连接: {}", addr);
                let adb = adb_utils.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, addr, adb).await {
                        error!("处理连接失败 {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("接受连接失败: {}", e);
            }
        }
    }
}

/// 处理单个连接
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    adb_utils: std::sync::Arc<AdbUtils>,
) -> Result<()> {
    // 用于捕获 WebSocket 握手请求的 URL
    let request_url = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let url_capture = request_url.clone();

    // WebSocket 握手回调
    let callback = move |req: &Request, response: Response| {
        // 保存请求 URL
        if let Ok(mut url) = url_capture.lock() {
            *url = Some(req.uri().to_string());
        }
        debug!("WebSocket 握手: {} {}", req.method(), req.uri());
        Ok(response)
    };

    // 执行 WebSocket 握手
    let ws_stream = accept_hdr_async(stream, callback)
        .await
        .context("WebSocket 握手失败")?;

    info!("✅ WebSocket 连接建立: {}", addr);

    // 解析请求参数
    let url_str = request_url.lock().unwrap().clone()
        .ok_or_else(|| anyhow::anyhow!("无法获取请求 URL"))?;

    // 处理相对 URL（添加假的 host）
    let full_url = if url_str.starts_with('/') {
        format!("http://localhost{}", url_str)
    } else {
        url_str
    };

    let url = Url::parse(&full_url)
        .context("解析 URL 失败")?;

    // 获取查询参数
    let params: std::collections::HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let action = params.get("action");

    // 只支持 proxy-adb 模式
    if action != Some(&"proxy-adb".to_string()) {
        error!("不支持的 action: {:?}", action);
        return Err(anyhow::anyhow!("只支持 action=proxy-adb"));
    }

    // 获取必需参数
    let udid = params.get("udid")
        .ok_or_else(|| anyhow::anyhow!("缺少 udid 参数"))?;
    let remote = params.get("remote")
        .ok_or_else(|| anyhow::anyhow!("缺少 remote 参数"))?;

    info!("proxy-adb 请求: udid={}, remote={}", udid, remote);

    // 处理 proxy-adb 连接
    proxy::handle_proxy_adb(ws_stream, udid, remote, adb_utils).await
}
