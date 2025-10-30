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
}

/// 处理 Server 相关命令
pub async fn handle(action: ServerCommands, device_id: Option<String>) -> Result<()> {
    match action {
        ServerCommands::Start { port } => {
            let server = if let Some(port) = port {
                AutoServer::new(device_id)?.with_port(port)
            } else {
                AutoServer::new(device_id)?
            };

            server.start()?;

            JsonOutput::print(serde_json::json!({
                "success": true,
                "status": "started",
                "port": server.port()
            }));
        }
        ServerCommands::Stop => {
            let server = AutoServer::new(device_id)?;
            server.stop()?;

            JsonOutput::print(serde_json::json!({
                "success": true,
                "status": "stopped"
            }));
        }
        ServerCommands::Status => {
            let server = AutoServer::new(device_id)?;
            let is_running = server.is_running();

            JsonOutput::print(serde_json::json!({
                "running": is_running,
                "port": server.port()
            }));
        }
    }

    Ok(())
}
