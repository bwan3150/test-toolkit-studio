// 工具直通命令处理器（① 工具调用）
// tke <工具名> <原生指令...> ：透传给 tke 同目录下的任意二进制
// 例: tke k6 run load.js / tke ffmpeg -i in.mp4 out.gif / tke opencv ...

use tke::{Result, ToolManager, JsonOutput};

/// 处理通用工具直通命令
/// args[0] = 工具名，其余为透传参数
pub async fn handle(mut args: Vec<String>, device_id: Option<String>) -> Result<()> {
    if args.is_empty() {
        JsonOutput::error("缺少工具名称");
    }

    let tool_name = args.remove(0);

    // 直通执行（继承 stdio，以工具退出码退出，不返回）
    ToolManager::passthrough(&tool_name, args, device_id)
}
