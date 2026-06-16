// Device 命令处理器

use tke::{Result, JsonOutput, DeviceManager};

/// Device 命令枚举
#[derive(clap::Subcommand)]
pub enum DeviceCommands {
    /// 获取完整的设备信息（包括硬件、电池、网络等所有信息）
    Info,
    /// 获取设备的单个 prop 属性值
    Prop {
        /// 属性名称 (如 ro.build.version.release)
        name: String,
    },
}

/// 处理 Device 相关命令
pub fn handle(action: DeviceCommands, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
    // 非 Android 设备（iOS/Web）：prop 是 adb 专属；info 走驱动给基础信息（型号/屏幕尺寸）
    if tke::Platform::from_device(device_id.as_deref()) != tke::Platform::Android {
        match action {
            DeviceCommands::Info => {
                let controller = tke::Controller::new(device_id)?;
                let info = controller.get_device_info()?;
                JsonOutput::print(serde_json::to_value(info).unwrap());
                return Ok(());
            }
            DeviceCommands::Prop { .. } => {
                return Err(tke::TkeError::InvalidArgument(
                    "device prop 仅支持 Android 设备 (adb getprop)".to_string(),
                ));
            }
        }
    }

    let device_manager = DeviceManager::new(device_id)?;

    match action {
        DeviceCommands::Info => {
            let device_info = device_manager.get_full_device_info()?;
            JsonOutput::print(serde_json::to_value(device_info).unwrap());
        }
        DeviceCommands::Prop { name } => {
            let value = device_manager.get_device_prop(&name)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "property": name,
                "value": value
            }));
        }
    }

    Ok(())
}
