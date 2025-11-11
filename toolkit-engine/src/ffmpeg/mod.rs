// FFmpeg 模块 - 直通命令到内嵌的 FFmpeg

use crate::utils::FfmpegManager;
use crate::Result;
use std::process::Command;

/// 执行 FFmpeg 直通命令
pub fn execute_ffmpeg_command(args: Vec<String>) -> Result<()> {
    let ffmpeg_manager = FfmpegManager::new()?;
    let ffmpeg_path = ffmpeg_manager.ffmpeg_path();

    let mut command = Command::new(ffmpeg_path);

    // 添加用户提供的参数
    command.args(&args);

    // 执行命令并继承标准输入输出
    let status = command
        .status()
        .map_err(|e| crate::TkeError::AdbError(format!("执行 FFmpeg 命令失败: {}", e)))?;

    // 使用命令的退出码
    std::process::exit(status.code().unwrap_or(1));
}
