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
    /// 看设备日志 (logcat)：App 崩了、点了没反应，真因往往只在这里
    Log {
        /// 只看这个包的日志（按 PID 过滤，比 grep 包名准——堆栈行里不带包名）
        #[arg(short = 'p', long)]
        package: Option<String>,
        /// 取最后多少行
        #[arg(short = 'n', long, default_value = "200")]
        lines: usize,
        /// 最低级别 V/D/I/W/E。默认 W——拉全量会把上下文冲爆
        #[arg(short = 'l', long, default_value = "W")]
        level: String,
    },
}

/// 处理 App 相关命令
pub async fn handle(action: AppCommands, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
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
        AppCommands::Log { package, lines, level } => {
            let text = app_manager.logcat(package.as_deref(), lines, &level).await?;
            // 日志**直接打原文**，不裹 JSON：这东西是给人和 AI 读的，
            // 裹进 JSON 字符串里满屏 \n 转义，谁也看不下去
            print!("{}", text);
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
