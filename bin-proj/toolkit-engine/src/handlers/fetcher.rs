// Fetcher 命令处理器

use tke::{Result, TkeError, Fetcher, JsonOutput};
use std::path::PathBuf;
use std::io::Read;

/// Fetcher 命令枚举
#[derive(clap::Subcommand)]
pub enum FetcherCommands {
    /// 从XML文件获取所有UI元素
    Extract {
        /// XML文件路径
        xml_path: PathBuf,
    },
    /// 从当前workarea获取UI元素
    Current,
    /// 过滤可交互元素
    Interactive,
    /// 过滤有文本的元素
    Text,
    /// 从XML推断屏幕尺寸
    InferScreenSize {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
    /// 优化UI树结构
    OptimizeUITree {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
    /// 从UI树提取元素列表
    ExtractUIElements {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
        /// 屏幕宽度
        #[arg(long)]
        width: Option<i32>,
        /// 屏幕高度
        #[arg(long)]
        height: Option<i32>,
    },
    /// 生成UI树的字符串描述
    GenerateTreeString {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
}

/// 处理 Fetcher 相关命令
pub async fn handle(action: FetcherCommands, project_path: PathBuf) -> Result<()> {
    let fetcher = Fetcher::new();

    match action {
        FetcherCommands::Extract { xml_path } => {
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            JsonOutput::print(&elements);
        }
        FetcherCommands::Current => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            if !xml_path.exists() {
                return Err(TkeError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "当前UI树文件不存在，请先运行 tke controller capture"
                )));
            }
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            JsonOutput::print(&elements);
        }
        FetcherCommands::Interactive => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let interactive = fetcher.filter_interactive_elements(&elements);
            JsonOutput::print(&interactive);
        }
        FetcherCommands::Text => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let text_elements = fetcher.filter_text_elements(&elements);
            JsonOutput::print(&text_elements);
        }
        FetcherCommands::InferScreenSize { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            if let Some((width, height)) = fetcher.infer_screen_size_from_xml(&xml)? {
                JsonOutput::print(serde_json::json!({"width": width, "height": height}));
            } else {
                JsonOutput::print(serde_json::Value::Null);
            }
        }
        FetcherCommands::OptimizeUITree { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            let optimized = fetcher.optimize_ui_tree(&xml)?;
            JsonOutput::print_raw(&optimized);
        }
        FetcherCommands::ExtractUIElements { xml_content, width, height } => {
            let xml = get_xml_content(xml_content)?;
            let elements = if let (Some(w), Some(h)) = (width, height) {
                fetcher.extract_ui_elements_with_size(&xml, w, h)?
            } else {
                fetcher.extract_ui_elements(&xml)?
            };
            JsonOutput::print(&elements);
        }
        FetcherCommands::GenerateTreeString { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            let tree_string = fetcher.generate_tree_string(&xml)?;
            JsonOutput::print_raw(&tree_string);
        }
    }

    Ok(())
}

/// 从参数或 stdin 获取 XML 内容
fn get_xml_content(xml_content: Option<String>) -> Result<String> {
    match xml_content {
        Some(content) => Ok(content),
        None => {
            // 从stdin读取XML内容
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}
