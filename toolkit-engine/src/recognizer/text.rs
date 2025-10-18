// 文本查找模块 - 根据文本内容查找元素

use crate::{Result, TkeError, Point, Fetcher};
use std::path::PathBuf;

/// 根据文本查找元素
pub fn find_by_text(project_path: &PathBuf, text: &str) -> Result<Point> {
    // 获取当前UI树
    let ui_tree_path = project_path.join("workarea").join("current_ui_tree.xml");
    let xml_content = std::fs::read_to_string(&ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    // 提取所有UI元素
    let fetcher = Fetcher::new();
    let elements = fetcher.fetch_elements_from_xml(&xml_content)?;

    // 查找匹配文本的元素
    let element = elements.iter()
        .find(|e| e.matches_text(text))
        .ok_or_else(|| TkeError::ElementNotFound(format!("未找到包含文本 '{}' 的元素", text)))?;

    Ok(element.center())
}
