// XML元素查找模块 - 根据Locator定义查找UI元素

use crate::{Result, TkeError, Point, UIElement, Locator, Fetcher};
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::debug;

/// 根据XML locator查找元素
///
/// # 参数
/// - `strategy_override`: 脚本中指定的策略（如 {元素名}&resourceId），优先于 locator 定义中的 matchStrategy
pub fn find_by_locator(
    project_path: &PathBuf,
    locators: &HashMap<String, Locator>,
    locator_name: &str,
    strategy_override: Option<&str>  // 脚本语法指定的策略
) -> Result<Point> {
    // 获取当前UI树
    let ui_tree_path = project_path.join("workarea").join("current_ui_tree.xml");
    let xml_content = std::fs::read_to_string(&ui_tree_path)
        .map_err(|e| TkeError::IoError(e))?;

    // 提取所有UI元素
    let fetcher = Fetcher::new();
    let elements = fetcher.fetch_elements_from_xml(&xml_content)?;

    // 获取locator定义
    let locator = locators.get(locator_name)
        .ok_or_else(|| TkeError::ElementNotFound(format!("Locator '{}' 未定义", locator_name)))?;

    // 查找匹配的元素
    let element = find_element_by_locator(&elements, locator, strategy_override)?;

    Ok(element.center())
}

/// 根据locator定义查找元素
///
/// # 新的查找逻辑 (2024重构)
/// 1. 如果脚本指定了策略（如 {元素名}&resourceId），则**只使用该策略，严格匹配**
/// 2. 如果脚本没有指定策略（仅 {元素名}），则使用**全精确匹配**（所有 locator 字段都必须匹配）
/// 3. 移除了原有的瀑布式匹配逻辑，避免找错元素
fn find_element_by_locator(
    elements: &[UIElement],
    locator: &Locator,
    strategy_override: Option<&str>
) -> Result<UIElement> {
    debug!("🔍 开始查找元素");
    debug!("  - Locator 定义: {:?}", locator);
    debug!("  - 脚本指定策略: {:?}", strategy_override);
    debug!("  - 当前有 {} 个UI元素可供匹配", elements.len());

    // 🔥 情况1: 脚本指定了策略（如 {登录按钮}#resourceId），只使用该策略
    if let Some(strategy) = strategy_override {
        debug!("✅ 使用脚本指定的策略: {}", strategy);
        return find_by_single_strategy(elements, locator, strategy);
    }

    // 🔥 情况2: 脚本没有指定策略（仅 {登录按钮}），使用全精确匹配
    debug!("✅ 使用全精确匹配模式（所有字段必须匹配）");
    find_by_exact_match_strict(elements, locator)
}

/// 🔥 使用单一策略查找（严格匹配，不使用 contains）
fn find_by_single_strategy(
    elements: &[UIElement],
    locator: &Locator,
    strategy: &str
) -> Result<UIElement> {
    debug!("  - 使用单一策略: {}", strategy);

    let result = match strategy {
        "resourceId" => {
            if let Some(ref resource_id) = locator.resource_id {
                find_by_resource_id_strict(elements, resource_id)
            } else {
                return Err(TkeError::ElementNotFound(
                    format!("策略 'resourceId' 要求 locator 定义中必须有 resourceId 字段")
                ));
            }
        }
        "text" => {
            if let Some(ref text) = locator.text {
                find_by_text_strict(elements, text)
            } else {
                return Err(TkeError::ElementNotFound(
                    format!("策略 'text' 要求 locator 定义中必须有 text 字段")
                ));
            }
        }
        "className" => {
            if let Some(ref class_name) = locator.class_name {
                find_by_class_name_strict(elements, class_name)
            } else {
                return Err(TkeError::ElementNotFound(
                    format!("策略 'className' 要求 locator 定义中必须有 className 字段")
                ));
            }
        }
        "xpath" => {
            if let Some(ref xpath) = locator.xpath {
                find_by_xpath_strict(elements, xpath)
            } else {
                return Err(TkeError::ElementNotFound(
                    format!("策略 'xpath' 要求 locator 定义中必须有 xpath 字段")
                ));
            }
        }
        _ => {
            return Err(TkeError::ElementNotFound(
                format!("未知的查找策略: {}", strategy)
            ));
        }
    };

    result.ok_or_else(|| {
        TkeError::ElementNotFound(format!("使用 {} 策略未找到匹配元素", strategy))
    })
}

/// 🔥 全精确匹配（所有 locator 字段都必须完全匹配）
fn find_by_exact_match_strict(elements: &[UIElement], locator: &Locator) -> Result<UIElement> {
    let matches: Vec<&UIElement> = elements.iter().filter(|e| {
        // 所有非空字段都必须精确匹配
        (locator.text.is_none() || e.text.as_ref() == locator.text.as_ref()) &&
        (locator.resource_id.is_none() || e.resource_id.as_ref() == locator.resource_id.as_ref()) &&
        (locator.class_name.is_none() || Some(&e.class_name) == locator.class_name.as_ref()) &&
        (locator.xpath.is_none() || e.xpath.as_ref() == locator.xpath.as_ref()) &&
        (locator.clickable.is_none() || Some(e.clickable) == locator.clickable) &&
        (locator.focusable.is_none() || Some(e.focusable) == locator.focusable) &&
        (locator.scrollable.is_none() || Some(e.scrollable) == locator.scrollable) &&
        (locator.enabled.is_none() || Some(e.enabled) == locator.enabled)
    }).collect();

    if matches.is_empty() {
        return Err(TkeError::ElementNotFound(
            "全精确匹配未找到元素（locator 定义的所有字段都必须完全匹配）".to_string()
        ));
    }

    // 🔥 如果找到多个匹配，警告并返回第一个
    if matches.len() > 1 {
        debug!("⚠️ 警告: 找到 {} 个匹配元素，将使用第一个", matches.len());
        for (idx, element) in matches.iter().enumerate() {
            debug!("  [{}/{}] text={:?}, resource_id={:?}, class={}, bounds={:?}",
                   idx + 1, matches.len(),
                   element.text, element.resource_id, element.class_name, element.bounds);
        }
    } else {
        debug!("✅ 找到唯一匹配元素: text={:?}, resource_id={:?}",
               matches[0].text, matches[0].resource_id);
    }

    Ok(matches[0].clone())
}

// ========== 严格匹配函数（不使用 contains 模糊匹配）==========

fn find_by_resource_id_strict(elements: &[UIElement], resource_id: &str) -> Option<UIElement> {
    let matches: Vec<&UIElement> = elements.iter().filter(|e| {
        e.resource_id.as_ref() == Some(&resource_id.to_string())
    }).collect();

    if matches.is_empty() {
        return None;
    }

    if matches.len() > 1 {
        debug!("⚠️ resourceId='{}' 找到 {} 个匹配，使用第一个", resource_id, matches.len());
    }

    Some(matches[0].clone())
}

fn find_by_xpath_strict(elements: &[UIElement], xpath: &str) -> Option<UIElement> {
    elements.iter().find(|e| {
        e.xpath.as_ref() == Some(&xpath.to_string())
    }).cloned()
}

fn find_by_text_strict(elements: &[UIElement], text: &str) -> Option<UIElement> {
    let matches: Vec<&UIElement> = elements.iter().filter(|e| {
        e.text.as_ref() == Some(&text.to_string())
    }).collect();

    if matches.is_empty() {
        return None;
    }

    if matches.len() > 1 {
        debug!("⚠️ text='{}' 找到 {} 个匹配，使用第一个", text, matches.len());
    }

    Some(matches[0].clone())
}

fn find_by_class_name_strict(elements: &[UIElement], class_name: &str) -> Option<UIElement> {
    let matches: Vec<&UIElement> = elements.iter().filter(|e| {
        &e.class_name == class_name
    }).collect();

    if matches.is_empty() {
        return None;
    }

    if matches.len() > 1 {
        debug!("⚠️ className='{}' 找到 {} 个匹配，使用第一个", class_name, matches.len());
    }

    Some(matches[0].clone())
}

