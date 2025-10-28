// Recognizer 命令处理器

use tke::{Result, Recognizer, JsonOutput};
use std::path::PathBuf;

/// Recognizer 命令枚举
#[derive(clap::Subcommand)]
pub enum RecognizerCommands {
    /// 根据XML locator查找元素位置
    FindXml {
        /// Locator名称
        locator_name: String,
    },
    /// 根据图像locator查找元素位置
    FindImage {
        /// Locator名称
        locator_name: String,
        /// 置信度阈值 (0.0-1.0)
        #[arg(long, default_value = "0.50")]
        threshold: f32,
    },
    /// 根据文本查找元素位置
    FindText {
        /// 文本内容
        text: String,
    },
}

/// 处理 Recognizer 相关命令
pub async fn handle(action: RecognizerCommands, project_path: PathBuf) -> Result<()> {
    // 初始化 recognizer，如果失败则输出 JSON 错误并退出
    let recognizer = Recognizer::new(project_path)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match action {
        RecognizerCommands::FindXml { locator_name } => {
            // CLI 调用时不指定策略，使用 locator 定义中的默认行为（全精确匹配）
            let point = recognizer.find_xml_element(&locator_name, None)
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            JsonOutput::success(serde_json::json!({
                "success": true,
                "x": point.x,
                "y": point.y
            }));
        }
        RecognizerCommands::FindImage { locator_name, threshold } => {
            // image 模块内部已经处理了 JSON 输出
            recognizer.find_image_element_json(&locator_name, threshold)
                .unwrap_or_else(|_| std::process::exit(1));
        }
        RecognizerCommands::FindText { text } => {
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
