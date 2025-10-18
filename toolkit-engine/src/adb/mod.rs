// ADB 模块 - 直通命令到内嵌的 ADB

use crate::utils::AdbManager;
use crate::Result;
use std::process::Command;

/// 执行 ADB 直通命令
pub fn execute_adb_command(args: Vec<String>, device_id: Option<String>) -> Result<()> {
    let adb_manager = AdbManager::new()?;
    let adb_path = adb_manager.adb_path();

    let mut command = Command::new(adb_path);

    // 如果指定了设备ID，添加 -s 参数
    if let Some(device) = device_id {
        command.arg("-s").arg(device);
    }

    // 添加用户提供的参数
    command.args(&args);

    // 执行命令并继承标准输入输出
    let status = command
        .status()
        .map_err(|e| crate::TkeError::AdbError(format!("执行 ADB 命令失败: {}", e)))?;

    // 使用命令的退出码
    std::process::exit(status.code().unwrap_or(1));
}
