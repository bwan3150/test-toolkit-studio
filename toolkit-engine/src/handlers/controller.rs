// Controller 命令处理器

use tke::{Result, Controller, JsonOutput};

/// Capture 子命令
#[derive(clap::Subcommand)]
pub enum CaptureTarget {
    /// 仅获取截图
    Screenshot,
    /// 仅获取 UI 树 XML
    Xml,
}

/// Controller 命令枚举
#[derive(clap::Subcommand)]
pub enum ControllerCommands {
    /// 获取连接的设备列表
    Devices,
    /// 获取设备截图和XML并保存到项目workarea
    Capture {
        #[command(subcommand)]
        target: Option<CaptureTarget>,
    },
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
    // 将所有操作包装在闭包中，统一捕获错误并输出 JSON
    let result: Result<()> = (|| async {
        let controller = Controller::new(device_id)?;

        match action {
        ControllerCommands::Devices => {
            let devices = controller.get_devices()?;
            JsonOutput::print(serde_json::json!({
                "devices": devices
            }));
        }
        ControllerCommands::Capture { target } => {
            let workarea = project_path.join("workarea");

            // 确保 workarea 目录存在
            std::fs::create_dir_all(&workarea)?;

            match target {
                None => {
                    // 默认：获取截图和 XML
                    controller.capture_ui_state(&project_path).await?;

                    let screenshot_path = workarea.join("current_screenshot.png");
                    let xml_path = workarea.join("current_ui_tree.xml");

                    JsonOutput::print(serde_json::json!({
                        "success": true,
                        "screenshot": screenshot_path.to_string_lossy(),
                        "xml": xml_path.to_string_lossy()
                    }));
                }
                Some(CaptureTarget::Screenshot) => {
                    // 仅获取截图
                    let screenshot_path = workarea.join("current_screenshot.png");
                    controller.capture_screenshot_only(&screenshot_path).await?;

                    JsonOutput::print(serde_json::json!({
                        "success": true,
                        "screenshot": screenshot_path.to_string_lossy()
                    }));
                }
                Some(CaptureTarget::Xml) => {
                    // 仅获取 XML
                    let xml_path = workarea.join("current_ui_tree.xml");
                    controller.capture_xml_only(&xml_path).await?;

                    JsonOutput::print(serde_json::json!({
                        "success": true,
                        "xml": xml_path.to_string_lossy()
                    }));
                }
            }
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
    })().await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // 将错误也输出为 JSON，然后正常返回（避免额外的错误输出）
            JsonOutput::print(serde_json::json!({
                "success": false,
                "error": format!("{}", e)
            }));
            Ok(())
        }
    }
}
