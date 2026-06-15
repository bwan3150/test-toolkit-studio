// 文本查找模块 - 根据文本内容查找元素（脚本中的纯文本参数）

use crate::{Result, TkeError, Point, Bounds, Fetcher};
use std::path::Path;

/// 根据文本查找元素，返回 (中心点, 实时边界框)
pub fn find_by_text(ui_tree_path: &Path, text: &str) -> Result<(Point, Bounds)> {
    let xml_content = std::fs::read_to_string(ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    // 提取所有UI元素
    let fetcher = Fetcher::new();
    let elements = fetcher.fetch_elements_from_xml(&xml_content)?;

    // 查找匹配文本的元素
    let element = elements.iter()
        .find(|e| e.matches_text(text))
        .ok_or_else(|| TkeError::ElementNotFound(format!("未找到包含文本 '{}' 的元素", text)))?;

    Ok((element.center(), element.bounds.clone()))
}
