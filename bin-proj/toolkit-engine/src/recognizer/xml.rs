// XML元素查找模块 - 根据 Locator 的 xml 通道在 UI 树中查找元素
// 返回 (中心点, 实时边界框)，边界框来自当前设备上实际匹配到的元素（用于截图标注）

use crate::{Result, TkeError, Point, Bounds, UIElement, Locator, Fetcher};
use crate::models::XmlLocator;
use std::path::Path;
use tracing::debug;

/// 加载 UI 树中的所有元素
fn load_ui_elements(ui_tree_path: &Path) -> Result<Vec<UIElement>> {
    let xml_content = std::fs::read_to_string(ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    let fetcher = Fetcher::new();
    fetcher.fetch_elements_from_xml(&xml_content)
}

/// 获取 locator 的 xml 通道，未定义则报错
fn xml_channel<'a>(locator: &'a Locator) -> Result<&'a XmlLocator> {
    locator.xml.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 xml 通道", locator.name))
    })
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

/// 通过 XPath 查找元素
pub fn find_by_xpath(ui_tree_path: &Path, locator: &Locator) -> Result<(Point, Bounds)> {
    let xpath = xml_channel(locator)?.xpath.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 xpath 字段", locator.name))
    })?;

    find_first(ui_tree_path, "XPath", xpath, |e| e.xpath.as_ref() == Some(xpath))
}

/// 通过 Resource ID 查找元素
pub fn find_by_resource_id(ui_tree_path: &Path, locator: &Locator) -> Result<(Point, Bounds)> {
    let resource_id = xml_channel(locator)?.resource_id.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 resource_id 字段", locator.name))
    })?;

    find_first(ui_tree_path, "ResourceId", resource_id, |e| {
        e.resource_id.as_ref() == Some(resource_id)
    })
}

/// 通过 Text 查找元素（精确匹配）
pub fn find_by_text(ui_tree_path: &Path, locator: &Locator) -> Result<(Point, Bounds)> {
    let text = xml_channel(locator)?.text.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 text 字段", locator.name))
    })?;

    find_first(ui_tree_path, "Text", text, |e| e.text.as_ref() == Some(text))
}

/// 通过 Content Description 查找元素
pub fn find_by_content_desc(ui_tree_path: &Path, locator: &Locator) -> Result<(Point, Bounds)> {
    let content_desc = xml_channel(locator)?.content_desc.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 content_desc 字段", locator.name))
    })?;

    find_first(ui_tree_path, "ContentDesc", content_desc, |e| {
        e.content_desc.as_ref() == Some(content_desc)
    })
}

/// 通过 Class Name 查找元素
pub fn find_by_class_name(ui_tree_path: &Path, locator: &Locator) -> Result<(Point, Bounds)> {
    let class_name = xml_channel(locator)?.class_name.as_ref().ok_or_else(|| {
        TkeError::ElementNotFound(format!("元素 '{}' 未定义 class_name 字段", locator.name))
    })?;

    find_first(ui_tree_path, "ClassName", class_name, |e| &e.class_name == class_name)
}
