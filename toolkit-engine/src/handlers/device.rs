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
pub fn handle(action: DeviceCommands, device_id: Option<String>) -> Result<()> {
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
