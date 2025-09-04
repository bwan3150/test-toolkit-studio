// LocatorFetcher模块 - 负责从XML中提取UI元素

use crate::{Result, TkeError, UIElement, Bounds};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct LocatorFetcher {
    // 有意义的属性集合
    meaningful_attributes: HashSet<String>,
    meaningful_bool_attributes: HashSet<String>,
    // 需要过滤的系统UI元素
    filtered_resource_ids: Vec<String>,
    // 屏幕尺寸
    screen_width: u32,
    screen_height: u32,
}

impl LocatorFetcher {
    pub fn new() -> Self {
        let mut meaningful_attributes = HashSet::new();
        meaningful_attributes.insert("text".to_string());
        meaningful_attributes.insert("content-desc".to_string());
        meaningful_attributes.insert("hint".to_string());
        meaningful_attributes.insert("hintText".to_string());
        meaningful_attributes.insert("title".to_string());
        meaningful_attributes.insert("accessibilityText".to_string());
        
        let mut meaningful_bool_attributes = HashSet::new();
        meaningful_bool_attributes.insert("clickable".to_string());
        meaningful_bool_attributes.insert("checkable".to_string());
        meaningful_bool_attributes.insert("focusable".to_string());
        meaningful_bool_attributes.insert("selectable".to_string());
        
        let filtered_resource_ids = vec![
            "status_bar_container".to_string(),
            "status_bar_launch_animation_container".to_string(),
            "navigationBarBackground".to_string(),
            "navigation_bar".to_string(),
        ];
        
        Self {
            meaningful_attributes,
            meaningful_bool_attributes,
            filtered_resource_ids,
            screen_width: 1080,
            screen_height: 1920,
        }
    }
    
    pub fn set_screen_size(&mut self, width: u32, height: u32) {
        self.screen_width = width;
        self.screen_height = height;
    }
    
    // 从XML文件中提取所有UI元素
    pub fn fetch_elements_from_file(&self, xml_path: &PathBuf) -> Result<Vec<UIElement>> {
        let xml_content = std::fs::read_to_string(xml_path)
            .map_err(|e| TkeError::IoError(e))?;
        
        self.fetch_elements_from_xml(&xml_content)
    }
    
    // 从XML字符串中提取所有UI元素
    pub fn fetch_elements_from_xml(&self, xml_content: &str) -> Result<Vec<UIElement>> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);
        
        let mut elements = Vec::new();
        let mut buf = Vec::new();
        let mut element_index = 0;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.name().as_ref() == b"node" {
                        if let Some(element) = self.parse_node(&e, element_index) {
                            if self.should_include_element(&element) {
                                elements.push(element);
                                element_index += 1;
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(TkeError::XmlError(format!("解析XML失败: {}", e))),
                _ => {}
            }
            
            buf.clear();
        }
        
        info!("从XML中提取了 {} 个UI元素", elements.len());
        Ok(elements)
    }
    
    // 解析单个节点
    fn parse_node(&self, e: &quick_xml::events::BytesStart, index: usize) -> Option<UIElement> {
        let mut class_name = String::new();
        let mut bounds = Bounds::new(0, 0, 0, 0);
        let mut text = None;
        let mut content_desc = None;
        let mut resource_id = None;
        let mut hint = None;
        let mut clickable = false;
        let mut checkable = false;
        let mut checked = false;
        let mut focusable = false;
        let mut focused = false;
        let mut scrollable = false;
        let mut selected = false;
        let mut enabled = true;
        
        // 解析所有属性
        for attr_result in e.attributes() {
            if let Ok(attr) = attr_result {
                let key = String::from_utf8_lossy(attr.key.as_ref());
                let value = attr.unescape_value()
                    .unwrap_or_default()
                    .to_string();
                
                match key.as_ref() {
                    "class" => class_name = value,
                    "bounds" => bounds = self.parse_bounds(&value),
                    "text" => if !value.is_empty() { text = Some(value) },
                    "content-desc" => if !value.is_empty() { content_desc = Some(value) },
                    "resource-id" => if !value.is_empty() { resource_id = Some(value) },
                    "hint" | "hintText" => if !value.is_empty() { hint = Some(value) },
                    "clickable" => clickable = value == "true",
                    "checkable" => checkable = value == "true",
                    "checked" => checked = value == "true",
                    "focusable" => focusable = value == "true",
                    "focused" => focused = value == "true",
                    "scrollable" => scrollable = value == "true",
                    "selected" => selected = value == "true",
                    "enabled" => enabled = value != "false",
                    _ => {}
                }
            }
        }
        
        Some(UIElement {
            index,
            class_name,
            bounds,
            text,
            content_desc,
            resource_id,
            hint,
            clickable,
            checkable,
            checked,
            focusable,
            focused,
            scrollable,
            selected,
            enabled,
            xpath: Some(format!("//node[{}]", index)),
        })
    }
    
    // 解析bounds字符串
    fn parse_bounds(&self, bounds_str: &str) -> Bounds {
        // 格式: "[0,0][1080,1920]"
        let clean = bounds_str
            .replace("][", ",")
            .replace("[", "")
            .replace("]", "");
        
        let coords: Vec<i32> = clean
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        
        if coords.len() == 4 {
            Bounds::new(coords[0], coords[1], coords[2], coords[3])
        } else {
            Bounds::new(0, 0, 0, 0)
        }
    }
    
    // 判断是否应该包含该元素
    fn should_include_element(&self, element: &UIElement) -> bool {
        // 检查是否是需要过滤的系统UI
        if let Some(ref resource_id) = element.resource_id {
            for filtered_id in &self.filtered_resource_ids {
                if resource_id.contains(filtered_id) {
                    return false;
                }
            }
        }
        
        // 检查是否可见
        if !element.is_visible() {
            return false;
        }
        
        // 检查是否在屏幕范围内
        if element.bounds.x1 >= self.screen_width as i32 || 
           element.bounds.y1 >= self.screen_height as i32 {
            return false;
        }
        
        if element.bounds.x2 <= 0 || element.bounds.y2 <= 0 {
            return false;
        }
        
        true
    }
    
    // 根据条件过滤元素
    pub fn filter_elements(&self, elements: &[UIElement], filter_fn: impl Fn(&UIElement) -> bool) -> Vec<UIElement> {
        elements
            .iter()
            .filter(|e| filter_fn(e))
            .cloned()
            .collect()
    }
    
    // 过滤可交互元素
    pub fn filter_interactive_elements(&self, elements: &[UIElement]) -> Vec<UIElement> {
        self.filter_elements(elements, |e| {
            e.clickable || e.focusable || e.checkable || e.scrollable
        })
    }
    
    // 过滤有文本的元素
    pub fn filter_text_elements(&self, elements: &[UIElement]) -> Vec<UIElement> {
        self.filter_elements(elements, |e| {
            e.text.is_some() || e.content_desc.is_some() || e.hint.is_some()
        })
    }
    
    // 过滤指定类名的元素
    pub fn filter_by_class_name(&self, elements: &[UIElement], class_name: &str) -> Vec<UIElement> {
        self.filter_elements(elements, |e| {
            e.class_name.contains(class_name)
        })
    }
    
    // 推断屏幕尺寸从XML内容
    pub fn infer_screen_size_from_xml(&self, xml_content: &str) -> Option<(u32, u32)> {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);
        
        let mut max_x = 0i32;
        let mut max_y = 0i32;
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.name().as_ref() == b"node" {
                        for attr_result in e.attributes() {
                            if let Ok(attr) = attr_result {
                                if attr.key.as_ref() == b"bounds" {
                                    if let Ok(value) = attr.unescape_value() {
                                        let bounds = self.parse_bounds(&value);
                                        max_x = max_x.max(bounds.x2);
                                        max_y = max_y.max(bounds.y2);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            
            buf.clear();
        }
        
        if max_x > 0 && max_y > 0 {
            debug!("从XML推断的屏幕尺寸: {}x{}", max_x, max_y);
            Some((max_x as u32, max_y as u32))
        } else {
            None
        }
    }
}