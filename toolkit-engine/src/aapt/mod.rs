// AAPT 模块 - 直通命令到内嵌的 AAPT

use crate::utils::AaptManager;
use crate::Result;
use std::process::Command;

/// 执行 AAPT 直通命令
pub fn execute_aapt_command(args: Vec<String>) -> Result<()> {
    let aapt_manager = AaptManager::new()?;
    let aapt_path = aapt_manager.aapt_path();

    let mut command = Command::new(aapt_path);

    // 添加用户提供的参数
    command.args(&args);

    // 执行命令并继承标准输入输出
    let status = command
        .status()
        .map_err(|e| crate::TkeError::AaptError(format!("执行 AAPT 命令失败: {}", e)))?;

    // 使用命令的退出码
    std::process::exit(status.code().unwrap_or(1));
}
