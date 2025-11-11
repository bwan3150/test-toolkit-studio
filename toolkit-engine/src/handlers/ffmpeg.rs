// FFmpeg 直通命令处理器

use tke::Result;

/// 处理 FFmpeg 直通命令
pub async fn handle(args: Vec<String>) -> Result<()> {
    // 使用 ffmpeg 模块执行命令
    tke::ffmpeg::execute_ffmpeg_command(args)
}
