// Proxy-ADB 处理模块
use anyhow::{Result, Context};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug};

use crate::adb::{AdbUtils, ScrcpyServer};

/// 处理 proxy-adb 连接
/// 在 WebSocket 和 TCP 之间双向转发数据
pub async fn handle_proxy_adb(
    ws_stream: WebSocketStream<TcpStream>,
    udid: &str,
    remote: &str,
    adb_utils: std::sync::Arc<AdbUtils>,
) -> Result<()> {
    info!("开始处理 proxy-adb 连接: {} -> {}", udid, remote);

    // 0. 获取 ADB 路径并创建 ScrcpyServer 实例
    let adb_path = adb_utils.get_adb_path();
    let scrcpy_server = ScrcpyServer::new(adb_path.to_string());

    // 1. 确保 scrcpy-server 在手机上运行
    info!("确保 scrcpy-server 在设备上运行...");
    scrcpy_server
        .ensure_server_running(udid)
        .await
        .context("启动 scrcpy-server 失败")?;

    // 2. 建立 ADB forward
    let local_port = adb_utils
        .forward(udid, remote)
        .await
        .context("建立 ADB forward 失败")?;

    info!("ADB forward 建立成功，本地端口: {}", local_port);

    // 3. 连接到本地转发端口
    let tcp_addr = format!("127.0.0.1:{}", local_port);
    let tcp_stream = match TcpStream::connect(&tcp_addr).await {
        Ok(stream) => {
            info!("成功连接到本地端口: {}", local_port);
            stream
        }
        Err(e) => {
            error!("连接到本地端口 {} 失败: {}", local_port, e);
            // 清理 forward
            let _ = adb_utils.remove_forward(udid, local_port).await;
            return Err(e.into());
        }
    };

    // 4. 双向转发数据
    let result = bidirectional_forward(ws_stream, tcp_stream).await;

    // 5. 清理：不移除 forward（因为可能有其他连接在使用）
    // 在实际应用中，ws-scrcpy 也不会立即清理 forward
    debug!("proxy-adb 连接结束");

    result
}

/// 在 WebSocket 和 TCP 之间双向转发数据
async fn bidirectional_forward(
    ws_stream: WebSocketStream<TcpStream>,
    tcp_stream: TcpStream,
) -> Result<()> {
    // 分离 WebSocket 的读写
    let (ws_write, mut ws_read) = ws_stream.split();
    let ws_write = std::sync::Arc::new(tokio::sync::Mutex::new(ws_write));

    // 分离 TCP 的读写
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    // 创建两个任务：
    // 1. WebSocket -> TCP (控制消息)
    // 2. TCP -> WebSocket (视频流)

    let ws_write_clone = ws_write.clone();
    let ws_to_tcp = async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    debug!("WebSocket -> TCP: {} 字节", data.len());
                    if let Err(e) = tcp_write.write_all(&data).await {
                        error!("写入 TCP 失败: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket 关闭");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    debug!("收到 Ping");
                    let mut writer = ws_write_clone.lock().await;
                    if let Err(e) = writer.send(Message::Pong(data)).await {
                        error!("发送 Pong 失败: {}", e);
                        break;
                    }
                }
                Ok(_) => {
                    // 忽略其他类型消息（Text, Pong 等）
                }
                Err(e) => {
                    error!("WebSocket 读取错误: {}", e);
                    break;
                }
            }
        }
        info!("WebSocket -> TCP 转发结束");
    };

    let tcp_to_ws = async move {
        let mut buffer = vec![0u8; 65536]; // 64KB 缓冲区
        loop {
            match tcp_read.read(&mut buffer).await {
                Ok(0) => {
                    debug!("TCP 连接关闭");
                    break;
                }
                Ok(n) => {
                    debug!("TCP -> WebSocket: {} 字节", n);
                    let data = &buffer[..n];
                    let mut writer = ws_write.lock().await;
                    if let Err(e) = writer.send(Message::Binary(data.to_vec())).await {
                        error!("发送 WebSocket 消息失败: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("TCP 读取错误: {}", e);
                    break;
                }
            }
        }
        info!("TCP -> WebSocket 转发结束");
    };

    // 并发执行两个转发任务，任一任务结束则全部结束
    tokio::select! {
        _ = ws_to_tcp => {
            debug!("WebSocket -> TCP 任务完成");
        }
        _ = tcp_to_ws => {
            debug!("TCP -> WebSocket 任务完成");
        }
    }

    Ok(())
}
