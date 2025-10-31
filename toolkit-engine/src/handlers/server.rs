// Server 命令处理器

use tke::{Result, AutoServer, JsonOutput};

/// Server 命令枚举
#[derive(clap::Subcommand)]
pub enum ServerCommands {
    /// 启动 autoserver 视频流
    Start {
        /// 端口号（可选，默认 8765）
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// 停止 autoserver 视频流
    Stop,
    /// 检查 autoserver 状态
    Status,
    /// 启动视频流服务器（H264 转 MJPEG 推流）
    VideoStream {
        /// HTTP 服务器端口（可选，默认 8766）
        #[arg(short = 'p', long)]
        http_port: Option<u16>,
        /// 视频流端口（可选，默认 27183）
        #[arg(short = 'v', long)]
        video_port: Option<u16>,
    },
}

/// 处理 Server 相关命令
pub async fn handle(action: ServerCommands, device_id: Option<String>) -> Result<()> {
    // 将所有操作包装在一个闭包中，统一捕获错误并输出 JSON
    let result = (|| -> Result<serde_json::Value> {
        match action {
            ServerCommands::Start { port } => {
                let server = if let Some(port) = port {
                    AutoServer::new(device_id.clone())?.with_port(port)
                } else {
                    AutoServer::new(device_id.clone())?
                };

                server.start()?;

                Ok(serde_json::json!({
                    "success": true,
                    "status": "started",
                    "port": server.port()
                }))
            }
            ServerCommands::Stop => {
                let server = AutoServer::new(device_id.clone())?;
                server.stop()?;

                Ok(serde_json::json!({
                    "success": true,
                    "status": "stopped"
                }))
            }
            ServerCommands::Status => {
                let server = AutoServer::new(device_id.clone())?;
                let is_running = server.is_running();

                Ok(serde_json::json!({
                    "running": is_running,
                    "port": server.port()
                }))
            }
            ServerCommands::VideoStream { http_port, video_port } => {
                let http_port = http_port.unwrap_or(8766);
                let video_port = video_port.unwrap_or(27183);

                // 启动视频流服务器（阻塞）
                tke::start_video_stream_server(http_port, video_port)?;

                Ok(serde_json::json!({
                    "success": true,
                    "status": "running",
                    "http_port": http_port,
                    "video_port": video_port
                }))
            }
        }
    })();

    match result {
        Ok(json) => {
            JsonOutput::print(json);
        }
        Err(e) => {
            // 将错误也输出为 JSON，然后正常返回（避免额外的错误输出）
            JsonOutput::print(serde_json::json!({
                "success": false,
                "error": format!("{}", e)
            }));
            // 注意：这里返回 Ok 是为了保持纯 JSON 输出
            // 如果需要错误退出码，可以改为 return Err(e)
        }
    }

    Ok(())

}
