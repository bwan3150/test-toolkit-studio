// XML元素查找模块 - 根据 Locator 定义在 UI 树中查找元素

use crate::{Result, TkeError, Point, UIElement, Locator, Fetcher};
use std::path::PathBuf;
use tracing::debug;

/// 加载当前 UI 树中的所有元素
fn load_ui_elements(project_path: &PathBuf) -> Result<Vec<UIElement>> {
    let ui_tree_path = project_path.join("workarea").join("current_ui_tree.xml");
    let xml_content = std::fs::read_to_string(&ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    let fetcher = Fetcher::new();
    fetcher.fetch_elements_from_xml(&xml_content)
}

/// 通过 XPath 查找元素
pub fn find_by_xpath(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let xpath = locator.xpath.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 xpath 字段", locator.name)
        ))?;

    let elements = load_ui_elements(project_path)?;

    debug!("XPath 查找: {}", xpath);

    let element = elements.iter()
        .find(|e| e.xpath.as_ref() == Some(xpath))
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("XPath 未找到匹配元素: {}", xpath)
        ))?;

    Ok(element.center())
}

/// 通过 Resource ID 查找元素
pub fn find_by_resource_id(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let resource_id = locator.resource_id.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 resource_id 字段", locator.name)
        ))?;

    let elements = load_ui_elements(project_path)?;

    debug!("ResourceId 查找: {}", resource_id);

    let matches: Vec<&UIElement> = elements.iter()
        .filter(|e| e.resource_id.as_ref() == Some(resource_id))
        .collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            format!("ResourceId 未找到匹配元素: {}", resource_id)
        ));
    }

    if matches.len() > 1 {
        debug!("⚠️ ResourceId '{}' 找到 {} 个匹配，使用第一个", resource_id, matches.len());
    }

    Ok(matches[0].center())
}

/// 通过 Text 查找元素（精确匹配）
pub fn find_by_text(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let text = locator.text.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 text 字段", locator.name)
        ))?;

    let elements = load_ui_elements(project_path)?;

    debug!("Text 查找: {}", text);

    let matches: Vec<&UIElement> = elements.iter()
        .filter(|e| e.text.as_ref() == Some(text))
        .collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            format!("Text 未找到匹配元素: {}", text)
        ));
    }

    if matches.len() > 1 {
        debug!("⚠️ Text '{}' 找到 {} 个匹配，使用第一个", text, matches.len());
    }

    Ok(matches[0].center())
}

/// 通过 Content Description 查找元素
pub fn find_by_content_desc(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let content_desc = locator.content_desc.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 content_desc 字段", locator.name)
        ))?;

    let elements = load_ui_elements(project_path)?;

    debug!("ContentDesc 查找: {}", content_desc);

    let matches: Vec<&UIElement> = elements.iter()
        .filter(|e| e.content_desc.as_ref() == Some(content_desc))
        .collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            format!("ContentDesc 未找到匹配元素: {}", content_desc)
        ));
    }

    if matches.len() > 1 {
        debug!("⚠️ ContentDesc '{}' 找到 {} 个匹配，使用第一个", content_desc, matches.len());
    }

    Ok(matches[0].center())
}

/// 通过 Class Name 查找元素
pub fn find_by_class_name(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let class_name = locator.class_name.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 class_name 字段", locator.name)
        ))?;

    let elements = load_ui_elements(project_path)?;

    debug!("ClassName 查找: {}", class_name);

    let matches: Vec<&UIElement> = elements.iter()
        .filter(|e| &e.class_name == class_name)
        .collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            format!("ClassName 未找到匹配元素: {}", class_name)
        ));
    }

    if matches.len() > 1 {
        debug!("⚠️ ClassName '{}' 找到 {} 个匹配，使用第一个", class_name, matches.len());
    }

    Ok(matches[0].center())
}

/// 全精确匹配 - 所有定义的字段都必须匹配
pub fn find_by_exact_match(project_path: &PathBuf, locator: &Locator) -> Result<Point> {
    let elements = load_ui_elements(project_path)?;

    debug!("全精确匹配查找元素: {}", locator.name);

    let matches: Vec<&UIElement> = elements.iter().filter(|e| {
        // 所有非空字段都必须精确匹配
        (locator.text.is_none() || e.text.as_ref() == locator.text.as_ref()) &&
        (locator.resource_id.is_none() || e.resource_id.as_ref() == locator.resource_id.as_ref()) &&
        (locator.class_name.is_none() || Some(&e.class_name) == locator.class_name.as_ref()) &&
        (locator.xpath.is_none() || e.xpath.as_ref() == locator.xpath.as_ref()) &&
        (locator.content_desc.is_none() || e.content_desc.as_ref() == locator.content_desc.as_ref()) &&
        (locator.clickable.is_none() || Some(e.clickable) == locator.clickable) &&
        (locator.enabled.is_none() || Some(e.enabled) == locator.enabled)
    }).collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            format!("全精确匹配未找到元素 '{}'", locator.name)
        ));
    }

    if matches.len() > 1 {
        debug!("⚠️ 全精确匹配找到 {} 个匹配，使用第一个", matches.len());
    }

    Ok(matches[0].center())
}
