// XML元素查找模块 - 根据Locator定义查找UI元素

use crate::{Result, TkeError, Point, UIElement, Locator, Bounds, Fetcher};
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::debug;

/// 根据XML locator查找元素
pub fn find_by_locator(
    project_path: &PathBuf,
    locators: &HashMap<String, Locator>,
    locator_name: &str
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
    let element = find_element_by_locator(&elements, locator)?;

    Ok(element.center())
}

/// 根据locator定义查找元素
fn find_element_by_locator(elements: &[UIElement], locator: &Locator) -> Result<UIElement> {
    debug!("开始查找元素，locator: {:?}", locator);
    debug!("当前有{}个UI元素可供匹配", elements.len());

    // 策略1: 根据matchStrategy优先匹配
    if let Some(ref strategy) = locator.match_strategy {
        debug!("使用匹配策略: {}", strategy);
        match strategy.as_str() {
            "resourceId" => {
                if let Some(ref resource_id) = locator.resource_id {
                    if let Some(element) = find_by_resource_id(elements, resource_id) {
                        debug!("通过resource_id匹配找到元素");
                        return Ok(element);
                    }
                }
            }
            "className" => {
                if let Some(ref class_name) = locator.class_name {
                    if let Some(element) = find_by_class_name(elements, class_name) {
                        debug!("通过class_name匹配找到元素");
                        return Ok(element);
                    }
                }
            }
            "text" => {
                if let Some(ref text) = locator.text {
                    if let Some(element) = find_by_text(elements, text) {
                        debug!("通过text匹配找到元素");
                        return Ok(element);
                    }
                }
            }
            "xpath" => {
                if let Some(ref xpath) = locator.xpath {
                    if let Some(element) = find_by_xpath(elements, xpath) {
                        debug!("通过xpath匹配找到元素");
                        return Ok(element);
                    }
                }
            }
            _ => debug!("未知的匹配策略: {}", strategy),
        }
    }

    // 策略2: 精确匹配
    if let Some(element) = find_by_exact_match(elements, locator) {
        debug!("通过精确匹配找到元素");
        return Ok(element);
    }

    // 策略3: resource_id匹配
    if let Some(ref resource_id) = locator.resource_id {
        if let Some(element) = find_by_resource_id(elements, resource_id) {
            debug!("通过resource_id匹配找到元素");
            return Ok(element);
        }
    }

    // 策略4: text匹配
    if let Some(ref text) = locator.text {
        if let Some(element) = find_by_text(elements, text) {
            debug!("通过text匹配找到元素");
            return Ok(element);
        }
    }

    // 策略5: 类名和位置的模糊匹配
    if let (Some(ref class_name), Some(ref bounds)) = (&locator.class_name, &locator.bounds) {
        if let Some(element) = find_by_class_and_position(elements, class_name, bounds) {
            debug!("通过类名和位置模糊匹配找到元素");
            return Ok(element);
        }
    }

    // 策略6: 仅基于类名的匹配
    if let Some(ref class_name) = locator.class_name {
        if let Some(element) = find_by_class_name(elements, class_name) {
            debug!("通过类名匹配找到元素");
            return Ok(element);
        }
    }

    // 策略7: xpath匹配
    if let Some(ref xpath) = locator.xpath {
        if let Some(element) = find_by_xpath(elements, xpath) {
            debug!("通过xpath匹配找到元素");
            return Ok(element);
        }
    }

    debug!("所有匹配策略都未找到元素");
    Err(TkeError::ElementNotFound("所有匹配策略都未找到元素".to_string()))
}

// 精确匹配
fn find_by_exact_match(elements: &[UIElement], locator: &Locator) -> Option<UIElement> {
    elements.iter().find(|e| {
        (locator.text.is_none() || e.text == locator.text) &&
        (locator.resource_id.is_none() || e.resource_id == locator.resource_id) &&
        (locator.class_name.is_none() || Some(&e.class_name) == locator.class_name.as_ref()) &&
        (locator.xpath.is_none() || e.xpath == locator.xpath)
    }).cloned()
}

// resource_id匹配
fn find_by_resource_id(elements: &[UIElement], resource_id: &str) -> Option<UIElement> {
    elements.iter().find(|e| {
        if let Some(ref id) = e.resource_id {
            id == resource_id || id.contains(resource_id) || resource_id.contains(id)
        } else {
            false
        }
    }).cloned()
}

// xpath匹配
fn find_by_xpath(elements: &[UIElement], xpath: &str) -> Option<UIElement> {
    elements.iter().find(|e| {
        if let Some(ref e_xpath) = e.xpath {
            e_xpath == xpath
        } else {
            false
        }
    }).cloned()
}

// text匹配
fn find_by_text(elements: &[UIElement], text: &str) -> Option<UIElement> {
    elements.iter().find(|e| {
        if let Some(ref t) = e.text {
            t == text || t.contains(text) || text.contains(t)
        } else {
            false
        }
    }).cloned()
}

// 类名匹配
fn find_by_class_name(elements: &[UIElement], class_name: &str) -> Option<UIElement> {
    // 先尝试完全匹配
    if let Some(element) = elements.iter().find(|e| e.class_name == class_name) {
        return Some(element.clone());
    }

    // 再尝试包含匹配
    elements.iter().find(|e| {
        e.class_name.contains(class_name) || class_name.contains(&e.class_name)
    }).cloned()
}

// 类名和位置的模糊匹配
fn find_by_class_and_position(elements: &[UIElement], class_name: &str, original_bounds: &Bounds) -> Option<UIElement> {
    let original_center = original_bounds.center();
    let tolerance = 100; // 像素容差

    // 查找同类型的元素
    let same_class_elements: Vec<&UIElement> = elements.iter()
        .filter(|e| e.class_name == class_name ||
                   e.class_name.contains(class_name) ||
                   class_name.contains(&e.class_name))
        .collect();

    if same_class_elements.is_empty() {
        return None;
    }

    // 如果只有一个同类型元素，直接返回
    if same_class_elements.len() == 1 {
        return Some(same_class_elements[0].clone());
    }

    // 多个同类型元素时，选择位置最接近的
    let mut best_match: Option<UIElement> = None;
    let mut min_distance = f64::MAX;

    for element in same_class_elements {
        let center = element.center();
        let distance = ((center.x - original_center.x).pow(2) +
                      (center.y - original_center.y).pow(2)) as f64;
        let distance = distance.sqrt();

        if distance < min_distance && distance <= tolerance as f64 {
            min_distance = distance;
            best_match = Some(element.clone());
        }
    }

    best_match
}
