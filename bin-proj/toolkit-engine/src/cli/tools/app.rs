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
    /// 获取当前聚焦的应用信息 (包名和Activity)
    Focus,
    /// 启动应用
    Launch {
        /// 应用包名 (如: com.example.app)
        package: String,
        /// Activity名称 (如: .MainActivity 或 com.example.app.MainActivity)
        activity: String,
    },
    /// 关闭应用
    Stop {
        /// 应用包名 (如: com.example.app)
        package: String,
    },
}

/// 处理 App 相关命令
pub async fn handle(action: AppCommands, device_id: Option<String>) -> Result<()> {
    // app 工具是 adb 专属（list/uninstall/focus 依赖包管理器）；
    // iOS/Web 的 启动/关闭 用原子指令 control launch/close
    if tke::Platform::from_device(device_id.as_deref()) != tke::Platform::Android {
        return Err(tke::TkeError::InvalidArgument(
            "tke app 仅支持 Android 设备；iOS/Web 请使用 control launch/close".to_string(),
        ));
    }

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
        AppCommands::Focus => {
            let focus_info = app_manager.get_current_focus().await?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "package_name": focus_info.package_name,
                "activity_name": focus_info.activity_name,
                "window_info": focus_info.window_info
            }));
        }
        AppCommands::Launch { package, activity } => {
            let (success, message) = app_manager.launch_app(&package, &activity).await?;
            JsonOutput::print(serde_json::json!({
                "success": success,
                "message": message,
                "package": package,
                "activity": activity
            }));
        }
        AppCommands::Stop { package } => {
            let (success, message) = app_manager.stop_app(&package).await?;
            JsonOutput::print(serde_json::json!({
                "success": success,
                "message": message,
                "package": package
            }));
        }
    }

    Ok(())
}
