// Recognizer 命令处理器

use tke::{Result, Recognizer, JsonOutput, LocatorStrategy};
use std::path::PathBuf;

/// Recognizer 命令枚举
#[derive(clap::Subcommand)]
pub enum RecognizerCommands {
    /// 统一元素查找（支持所有策略）
    Find {
        /// Locator名称
        locator_name: String,
        /// 定位策略 (auto/xpath/resource-id/text/content-desc/class-name/ocr/img)
        #[arg(long, short, default_value = "auto")]
        strategy: String,
        /// 置信度阈值 (用于图像匹配, 0.0-1.0)
        #[arg(long, default_value = "0.60")]
        threshold: f32,
    },
    /// 根据文本查找元素位置（直接文本匹配，非 locator）
    FindText {
        /// 文本内容
        text: String,
    },
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

/// 处理 Recognizer 相关命令
pub async fn handle(action: RecognizerCommands, project_path: PathBuf) -> Result<()> {
    // 初始化 recognizer
    let mut recognizer = Recognizer::new(project_path)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match action {
        RecognizerCommands::Find { locator_name, strategy, threshold } => {
            let strategy = parse_strategy(&strategy);

            // 设置图像匹配阈值
            recognizer.set_confidence_threshold(threshold);

            // 使用统一的 find_element 方法
            let point = recognizer.find_element(&locator_name, strategy).await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            JsonOutput::success(serde_json::json!({
                "success": true,
                "x": point.x,
                "y": point.y
            }));
        }
        RecognizerCommands::FindText { text } => {
            // 直接文本查找（在 UI 树中搜索）
            let point = recognizer.find_element_by_text(&text)
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            JsonOutput::success(serde_json::json!({
                "success": true,
                "x": point.x,
                "y": point.y
            }));
        }
    }

    Ok(())
}
