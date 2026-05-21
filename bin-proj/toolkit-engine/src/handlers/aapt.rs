// AAPT 直通命令处理器

use tke::Result;

/// 处理 AAPT 直通命令
pub async fn handle(args: Vec<String>) -> Result<()> {
    // 使用 aapt 模块执行命令
    tke::aapt::execute_aapt_command(args)
}
