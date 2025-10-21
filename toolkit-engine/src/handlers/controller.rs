// Controller 命令处理器

use tke::{Result, Controller, JsonOutput};

/// Controller 命令枚举
#[derive(clap::Subcommand)]
pub enum ControllerCommands {
    /// 获取连接的设备列表
    Devices,
    /// 获取设备截图和XML并保存到项目workarea
    Capture,
    /// 点击坐标
    Tap {
        /// X坐标
        x: i32,
        /// Y坐标
        y: i32,
    },
    /// 滑动
    Swipe {
        /// 起点X坐标
        x1: i32,
        /// 起点Y坐标
        y1: i32,
        /// 终点X坐标
        x2: i32,
        /// 终点Y坐标
        y2: i32,
        /// 持续时间(毫秒)
        #[arg(short, long, default_value = "300")]
        duration: u32,
    },
    /// 启动应用
    Launch {
        /// 应用包名
        package: String,
        /// Activity名
        activity: String,
    },
    /// 停止应用
    Stop {
        /// 应用包名
        package: String,
    },
    /// 输入文本
    Input {
        /// 要输入的文本
        text: String,
    },
    /// 清空输入框 (通过全选+删除)
    ClearInput,
    /// 返回键
    Back,
    /// 主页键
    Home,
}

/// 处理 Controller 相关命令
pub async fn handle(action: ControllerCommands, device_id: Option<String>, project_path: std::path::PathBuf) -> Result<()> {
    let controller = Controller::new(device_id)?;

    match action {
        ControllerCommands::Devices => {
            let devices = controller.get_devices()?;
            JsonOutput::print(serde_json::json!({
                "devices": devices
            }));
        }
        ControllerCommands::Capture => {
            // 使用传入的 project_path 参数而不是 current_dir()
            controller.capture_ui_state(&project_path).await?;

            let screenshot_path = project_path.join("workarea").join("current_screenshot.png");
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");

            JsonOutput::print(serde_json::json!({
                "success": true,
                "screenshot": screenshot_path.to_string_lossy(),
                "xml": xml_path.to_string_lossy()
            }));
        }
        ControllerCommands::Tap { x, y } => {
            controller.tap(x, y)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "x": x,
                "y": y
            }));
        }
        ControllerCommands::Swipe { x1, y1, x2, y2, duration } => {
            controller.swipe(x1, y1, x2, y2, duration)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "from": {"x": x1, "y": y1},
                "to": {"x": x2, "y": y2},
                "duration": duration
            }));
        }
        ControllerCommands::Launch { package, activity } => {
            controller.launch_app(&package, &activity)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "package": package,
                "activity": activity
            }));
        }
        ControllerCommands::Stop { package } => {
            controller.stop_app(&package)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "package": package
            }));
        }
        ControllerCommands::Input { text } => {
            controller.input_text(&text)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "text": text
            }));
        }
        ControllerCommands::ClearInput => {
            controller.clear_input()?;
            JsonOutput::print(serde_json::json!({
                "success": true
            }));
        }
        ControllerCommands::Back => {
            controller.back()?;
            JsonOutput::print(serde_json::json!({
                "success": true
            }));
        }
        ControllerCommands::Home => {
            controller.home()?;
            JsonOutput::print(serde_json::json!({
                "success": true
            }));
        }
    }

    Ok(())
}
