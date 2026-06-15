// Recognize 命令处理器（② 原子方法）
// 在当前页面定位元素位置（默认先采集最新页面状态再定位）
// 元素库由 --element 指定（缺省按 ./element.json → ./locator/element.json 查找）

use tke::{Result, Recognize, JsonOutput, LocatorStrategy};
use std::path::PathBuf;

/// Recognize 命令参数
#[derive(clap::Args)]
pub struct RecognizeArgs {
    /// 元素名称（元素库 key）；--by-text 时为直接查找的文本
    pub name: String,

    /// 定位策略 (auto/xpath/resource-id/text/content-desc/class-name/ocr/img)
    #[arg(long, short, default_value = "auto")]
    pub strategy: String,

    /// 图像匹配置信度阈值 (0.0-1.0)
    #[arg(long, default_value = "0.60")]
    pub threshold: f32,

    /// 直接按文本在 UI 树中查找（不走元素库）
    #[arg(long)]
    pub by_text: bool,

    /// 使用工作区中已有的页面状态，跳过重新采集
    #[arg(long)]
    pub cached: bool,
}

/// 解析策略字符串
fn parse_strategy(s: &str) -> LocatorStrategy {
    match s.to_lowercase().as_str() {
        "xpath" => LocatorStrategy::XPath,
        "resource-id" | "resourceid" | "id" => LocatorStrategy::ResourceId,
        "text" => LocatorStrategy::Text,
        "content-desc" | "contentdesc" | "desc" => LocatorStrategy::ContentDesc,
        "class-name" | "classname" | "class" => LocatorStrategy::ClassName,
        "ocr" => LocatorStrategy::Ocr,
        "img" | "image" => LocatorStrategy::Img,
        _ => LocatorStrategy::Auto,
    }
}

/// 处理 Recognize 命令（必须指定 -d/--device）
pub async fn handle(
    args: RecognizeArgs,
    device_id: Option<String>,
    element_path: Option<PathBuf>,
) -> Result<()> {
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("recognize 必须指定设备: -d/--device <设备ID>"));

    let mut recognize = Recognize::new(device, element_path.as_deref())
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));
    recognize.set_confidence_threshold(args.threshold);

    let result = if args.by_text {
        recognize.find_text(&args.name, args.cached).await
    } else {
        recognize
            .find(&args.name, parse_strategy(&args.strategy), args.cached)
            .await
    };

    match result {
        Ok((point, bounds)) => {
            JsonOutput::success(serde_json::json!({
                "success": true,
                "element": args.name,
                "x": point.x,
                "y": point.y,
                // 实时边界框：来自当前页面实际匹配到的元素
                "bounds": bounds,
            }));
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
