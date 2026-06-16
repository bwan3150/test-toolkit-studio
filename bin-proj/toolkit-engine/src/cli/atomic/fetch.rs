// Fetch 命令处理器（② 原子方法）
// 提取当前页面的元素列表（含 xpath），直接输出 JSON 数组
// 默认先刷新页面状态；--cached 用工作区已有状态

use tke::{Result, Fetch, JsonOutput};

/// Fetch 命令参数
#[derive(clap::Args)]
pub struct FetchArgs {
    /// 使用工作区中已有的页面状态，跳过重新采集（先 tke refresh 后使用）
    #[arg(long)]
    pub cached: bool,

    /// 只输出可交互元素
    #[arg(long)]
    pub interactive: bool,
}

/// 处理 Fetch 命令（必须指定 -d/--device）
pub async fn handle(args: FetchArgs, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("fetch 必须指定设备: -d/--device <设备ID>"));

    let fetch = Fetch::new(device)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match fetch.elements(args.cached).await {
        Ok(elements) => {
            let output = if args.interactive {
                elements.into_iter().filter(|e| e.clickable).collect::<Vec<_>>()
            } else {
                elements
            };
            // 直接输出元素数组
            JsonOutput::print(&output);
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
