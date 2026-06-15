// 结构标识查找模块 - 在归一化结构文件中按标识匹配元素
// Android: uiautomator dump 的 XML；Web: JS 提取归一化的同构 XML
// 返回 (中心点, 实时边界框)，边界框来自当前页面实际匹配到的元素（用于截图标注）

use crate::{Result, TkeError, Point, Bounds, UIElement, Fetcher};
use crate::models::AndroidLocator;
use std::path::Path;
use tracing::debug;

/// 加载结构文件中的所有元素
fn load_ui_elements(ui_tree_path: &Path) -> Result<Vec<UIElement>> {
    let xml_content = std::fs::read_to_string(ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    let fetcher = Fetcher::new();
    fetcher.fetch_elements_from_xml(&xml_content)
}

/// 按指定条件过滤元素，返回第一个匹配的 (中心点, 边界框)
fn find_first<F>(
    ui_tree_path: &Path,
    desc: &str,
    value: &str,
    pred: F,
) -> Result<(Point, Bounds)>
where
    F: Fn(&UIElement) -> bool,
{
    let elements = load_ui_elements(ui_tree_path)?;

    debug!("{} 查找: {}", desc, value);

    let matches: Vec<&UIElement> = elements.iter().filter(|e| pred(e)).collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(format!(
            "{} 未找到匹配元素: {}", desc, value
        )));
    }

    if matches.len() > 1 {
        debug!("⚠️ {} '{}' 找到 {} 个匹配，使用第一个", desc, value, matches.len());
    }

    Ok((matches[0].center(), matches[0].bounds.clone()))
}

/// 取结构标识字段，未定义则报错
fn field<'a>(name: &str, v: &'a Option<String>, field_name: &str) -> Result<&'a String> {
    v.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 {} 字段", name, field_name))
    })
}

/// 通过 XPath 查找元素
pub fn find_by_xpath(ui_tree_path: &Path, name: &str, st: &AndroidLocator) -> Result<(Point, Bounds)> {
    let xpath = field(name, &st.xpath, "xpath")?;
    find_first(ui_tree_path, "XPath", xpath, |e| e.xpath.as_ref() == Some(xpath))
}

/// 通过 Resource ID / DOM id 查找元素
pub fn find_by_resource_id(ui_tree_path: &Path, name: &str, st: &AndroidLocator) -> Result<(Point, Bounds)> {
    let resource_id = field(name, &st.resource_id, "resource_id/id")?;
    find_first(ui_tree_path, "ResourceId", resource_id, |e| {
        e.resource_id.as_ref() == Some(resource_id)
    })
}

/// 通过 Text 查找元素（精确匹配）
pub fn find_by_text(ui_tree_path: &Path, name: &str, st: &AndroidLocator) -> Result<(Point, Bounds)> {
    let text = field(name, &st.text, "text")?;
    find_first(ui_tree_path, "Text", text, |e| e.text.as_ref() == Some(text))
}

/// 通过 Content Description / aria-label 查找元素
pub fn find_by_content_desc(ui_tree_path: &Path, name: &str, st: &AndroidLocator) -> Result<(Point, Bounds)> {
    let content_desc = field(name, &st.content_desc, "content_desc/aria")?;
    find_first(ui_tree_path, "ContentDesc", content_desc, |e| {
        e.content_desc.as_ref() == Some(content_desc)
    })
}

/// 通过 Class Name / 标签名 查找元素
pub fn find_by_class_name(ui_tree_path: &Path, name: &str, st: &AndroidLocator) -> Result<(Point, Bounds)> {
    let class_name = field(name, &st.class_name, "class_name/tag")?;
    find_first(ui_tree_path, "ClassName", class_name, |e| &e.class_name == class_name)
}
