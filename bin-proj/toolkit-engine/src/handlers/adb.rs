// ADB 直通命令处理器

use tke::Result;

/// 处理 ADB 直通命令
pub async fn handle(args: Vec<String>, device_id: Option<String>) -> Result<()> {
    // 使用 adb 模块执行命令
    tke::adb::execute_adb_command(args, device_id)
}
