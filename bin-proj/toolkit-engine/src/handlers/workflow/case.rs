// Case 命令处理器（③ 工作流）
// tke case <用例.md|"用例文字"> --script <导出.tks路径>
// AI 根据文字用例在真机上探索测试并生成 .tks 脚本（由 tester-ai 实现，tke 作为入口）

use tke::{Result, ToolManager, JsonOutput};
use std::path::PathBuf;

/// Case 命令参数
#[derive(clap::Args)]
pub struct CaseArgs {
    /// 测试用例: .md 文件路径或一段文字描述
    pub case: String,

    /// 生成的 .tks 脚本导出路径
    #[arg(long)]
    pub script: PathBuf,

    /// 透传给 tester-ai 的额外参数
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub extra: Vec<String>,
}

/// 处理 Case 命令：组装参数透传给 tester-ai
pub async fn handle(
    args: CaseArgs,
    device_id: Option<String>,
    element: Option<PathBuf>,
) -> Result<()> {
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("case 必须指定设备: -d/--device <设备ID>"));

    let mut forward: Vec<String> = vec![
        "--case".into(), args.case,
        "--script".into(), args.script.to_string_lossy().to_string(),
        "--device".into(), device,
    ];
    if let Some(element) = element {
        forward.push("--element".into());
        forward.push(element.to_string_lossy().to_string());
    }
    forward.extend(args.extra);

    // 透传给 tester-ai（与 tke 同目录），AI 探索工作流由其实现
    ToolManager::passthrough("tester-ai", forward, None)
}
