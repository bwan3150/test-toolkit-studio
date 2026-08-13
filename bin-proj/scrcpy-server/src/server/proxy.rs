// Proxy-ADB 处理模块
use anyhow::{Result, Context, anyhow};
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, connect_async};
use tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
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

    // 1. 先停止旧的 scrcpy-server（如果存在）
    // 因为 scrcpy-server 每次只能处理一个连接，旧连接可能已死但进程还在
    info!("停止旧的 scrcpy-server（如果存在）...");
    let _ = scrcpy_server.stop_server(udid).await; // 忽略错误

    // 2. 重新启动 scrcpy-server
    info!("重新启动 scrcpy-server...");
    if let Err(e) = scrcpy_server.ensure_server_running(udid).await {
        error!("❌ 启动 scrcpy-server 失败: {}", e);
        return Err(e).context("启动 scrcpy-server 失败");
    }
    info!("✅ scrcpy-server 已成功启动并就绪");

    // 2. 建立 ADB forward
    let local_port = adb_utils
        .forward(udid, remote)
        .await
        .context("建立 ADB forward 失败")?;

    info!("ADB forward 建立成功，本地端口: {}", local_port);

    // 3. 连接到本地转发端口（WebSocket 客户端）
    // 注意：scrcpy-server 内置了 WebSocket 服务器
    // 需要重试，因为 scrcpy-server 启动需要时间
    let ws_url = format!("ws://127.0.0.1:{}", local_port);
    info!("连接到 scrcpy-server WebSocket: {}", ws_url);

    // 先等待一段时间，让 scrcpy-server 完全启动
    info!("等待 scrcpy-server WebSocket 就绪...");
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let mut remote_ws = None;
    let max_retries = 10;
    let retry_interval = tokio::time::Duration::from_millis(1000);

    for attempt in 1..=max_retries {
        info!("尝试连接 scrcpy-server WebSocket（第 {} 次）...", attempt);

        match connect_async(&ws_url).await {
            Ok(result) => {
                info!("✅ 成功连接到 scrcpy-server WebSocket");
                remote_ws = Some(result.0);
                break;
            }
            Err(e) => {
                if attempt < max_retries {
                    info!("连接失败: {}，{}ms 后重试...", e, retry_interval.as_millis());
                    tokio::time::sleep(retry_interval).await;
                } else {
                    error!("连接到 scrcpy-server WebSocket 失败（已重试 {} 次）: {}", max_retries, e);
                    // 清理 forward
                    let _ = adb_utils.remove_forward(udid, local_port).await;
                    return Err(e.into());
                }
            }
        }
    }

    let remote_ws = remote_ws.ok_or_else(|| anyhow!("无法连接到 scrcpy-server WebSocket"))?;

    // 4. 双向转发数据（WebSocket 到 WebSocket）
    let result = bidirectional_forward_ws(ws_stream, remote_ws).await;

    // 5. 清理：不移除 forward（因为可能有其他连接在使用）
    // 在实际应用中，ws-scrcpy 也不会立即清理 forward
    debug!("proxy-adb 连接结束");

    result
}

/// 在两个 WebSocket 之间双向转发数据
async fn bidirectional_forward_ws(
    client_ws: WebSocketStream<TcpStream>,
    remote_ws: WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
) -> Result<()> {
    // 分离客户端 WebSocket
    let (client_write, mut client_read) = client_ws.split();
    let client_write = std::sync::Arc::new(tokio::sync::Mutex::new(client_write));

    // 分离远程 WebSocket (scrcpy-server)
    let (remote_write, mut remote_read) = remote_ws.split();
    let remote_write = std::sync::Arc::new(tokio::sync::Mutex::new(remote_write));

    // 任务 1: 客户端 -> scrcpy-server (控制消息)
    let remote_write_clone = remote_write.clone();
    let client_to_remote = async move {
        let mut msg_count = 0;
        while let Some(msg) = client_read.next().await {
            match msg {
                Ok(msg) => {
                    msg_count += 1;
                    if msg_count <= 3 {
                        info!("客户端 -> scrcpy-server [消息 {}]: {:?}", msg_count, msg);
                    }
                    let mut writer = remote_write_clone.lock().await;
                    if let Err(e) = writer.send(msg).await {
                        error!("发送到 scrcpy-server 失败: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("客户端 WebSocket 读取错误: {}", e);
                    break;
                }
            }
        }
        info!("客户端 -> scrcpy-server 转发结束（总计 {} 条消息）", msg_count);
    };

    // 任务 2: scrcpy-server -> 客户端 (视频流)
    let client_write_clone = client_write.clone();
    let remote_to_client = async move {
        let mut msg_count = 0;
        while let Some(msg) = remote_read.next().await {
            match msg {
                Ok(msg) => {
                    msg_count += 1;
                    if msg_count <= 3 {
                        info!("scrcpy-server -> 客户端 [消息 {}]: {:?}", msg_count, msg);
                    }
                    // 注释掉频繁的日志输出
                    // else if msg_count % 100 == 0 {
                    //     info!("scrcpy-server -> 客户端: 已转发 {} 条消息", msg_count);
                    // }
                    let mut writer = client_write_clone.lock().await;
                    if let Err(e) = writer.send(msg).await {
                        error!("发送到客户端失败: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("scrcpy-server WebSocket 读取错误: {}", e);
                    break;
                }
            }
        }
        info!("scrcpy-server -> 客户端 转发结束（总计 {} 条消息）", msg_count);
    };

    // 并发执行两个转发任务
    tokio::select! {
        _ = client_to_remote => {
            debug!("客户端 -> scrcpy-server 任务完成");
        }
        _ = remote_to_client => {
            debug!("scrcpy-server -> 客户端 任务完成");
        }
    }

    Ok(())
}
