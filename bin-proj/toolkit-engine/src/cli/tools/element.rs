// Element 命令处理器 - 元素库管理

use tke::{Result, JsonOutput};
use std::path::PathBuf;

/// Element 命令枚举
#[derive(clap::Subcommand)]
pub enum ElementCommands {
    /// 按坐标从当前页面取元素并落库（自动填当前平台通道 + crop 模板图 + ocr 文本）
    Add {
        /// 元素名（元素库 key，脚本中 {元素名} 引用）
        name: String,
        /// 取元素的屏幕坐标（截图像素），格式 x,y
        #[arg(long)]
        at: String,
        /// 元素描述
        #[arg(long)]
        desc: Option<String>,
        /// 使用工作区中已有的页面状态，跳过重新采集
        #[arg(long)]
        cached: bool,
        /// 强制覆盖已有的 img/ocr 通道
        #[arg(long)]
        force: bool,
    },
}

/// 解析 "x,y" 坐标
fn parse_at(s: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (parts[0].parse(), parts[1].parse()) {
            return Ok((x, y));
        }
    }
    Err(tke::TkeError::InvalidArgument(format!(
        "坐标格式错误: {} (应为 x,y 如 363,231)", s
    )))
}

/// 处理 Element 相关命令（必须指定 -d/--device）
/// element_lib 为参数层解析好的元素库路径（写入语义：含默认创建路径）
pub async fn handle(
    action: ElementCommands,
    device_id: Option<String>,
    element_lib: PathBuf,
) -> Result<()> {
    match action {
        ElementCommands::Add { name, at, desc, cached, force } => {
            let device = device_id
                .unwrap_or_else(|| JsonOutput::error("element add 必须指定设备: -d/--device <设备ID>"));
            let (x, y) = parse_at(&at)?;
            let result = tke::tools::element::add_element(
                device, &element_lib, &name, desc, x, y, cached, force,
            )
            .await?;
            JsonOutput::print(result);
        }
    }
    Ok(())
}
