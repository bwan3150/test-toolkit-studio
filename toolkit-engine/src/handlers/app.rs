// App 命令处理器

use tke::{Result, JsonOutput, AppManager};

/// App 命令枚举
#[derive(clap::Subcommand)]
pub enum AppCommands {
    /// 列出设备上所有第三方应用及版本信息
    List,
    /// 强制卸载应用
    Uninstall {
        /// 应用包名 (如: com.example.app)
        package: String,
    },
}

/// 处理 App 相关命令
pub async fn handle(action: AppCommands, device_id: Option<String>) -> Result<()> {
    let app_manager = AppManager::new(device_id)?;

    match action {
        AppCommands::List => {
            let apps = app_manager.list_third_party_apps().await?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "count": apps.len(),
                "apps": apps
            }));
        }
        AppCommands::Uninstall { package } => {
            let (success, message) = app_manager.uninstall_app(&package).await?;
            JsonOutput::print(serde_json::json!({
                "success": success,
                "message": message,
                "package": package
            }));
        }
    }

    Ok(())
}
