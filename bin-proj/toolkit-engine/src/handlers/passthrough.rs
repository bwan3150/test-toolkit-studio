// 直通命令处理器（① 直通）
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

    // 路径类参数明确提示（直通只接受工具名）
    if tool_name.contains('/') {
        eprintln!(
            "'{}' 是文件路径而非工具名。直通用法: tke <工具名> <参数> (如 tke adb devices); 运行脚本请用: tke run <path.tks|path.toml>",
            tool_name
        );
        std::process::exit(2);
    }

    // 直通执行（继承 stdio，以工具退出码退出）；缺失时输出干净的错误信息
    if let Err(e) = ToolManager::passthrough(&tool_name, args, device_id) {
        eprintln!("{}", e);
        std::process::exit(127);
    }
    Ok(())
}
