// Recognizer模块 - 负责元素识别(XML匹配和图像匹配)

use crate::{Result, TkeError, UIElement, Locator, LocatorType, Point, Bounds};
use image::{DynamicImage, GenericImageView, Rgb};
use imageproc::template_matching::{find_extremes, match_template, MatchTemplateMethod};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

pub struct Recognizer {
    project_path: PathBuf,
    locators: HashMap<String, Locator>,
    confidence_threshold: f32,
}

impl Recognizer {
    pub fn new(project_path: PathBuf) -> Result<Self> {
        // 加载locator定义
        let locators = Self::load_locators(&project_path)?;
        
        Ok(Self {
            project_path,
            locators,
            confidence_threshold: 0.8,
        })
    }
    
    // 加载项目的locator定义
    fn load_locators(project_path: &PathBuf) -> Result<HashMap<String, Locator>> {
        let locator_file = project_path.join("locator").join("element.json");
        
        if !locator_file.exists() {
            debug!("Locator文件不存在: {:?}", locator_file);
            return Ok(HashMap::new());
        }
        
        let content = std::fs::read_to_string(&locator_file)
            .map_err(|e| TkeError::IoError(e))?;
        
        let locators: HashMap<String, Locator> = serde_json::from_str(&content)
            .map_err(|e| TkeError::JsonError(e))?;
        
        info!("加载了 {} 个locator定义", locators.len());
        Ok(locators)
    }
    
    // 重新加载locator定义
    pub fn reload_locators(&mut self) -> Result<()> {
        self.locators = Self::load_locators(&self.project_path)?;
        Ok(())
    }
    
    // 设置置信度阈值
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }
    
    // 指令1: 根据XML locator查找元素
    pub fn find_xml_element(&self, locator_name: &str) -> Result<Point> {
        // 获取当前UI树
        let ui_tree_path = self.project_path.join("workarea").join("current_ui_tree.xml");
        let xml_content = std::fs::read_to_string(&ui_tree_path)
            .map_err(|e| TkeError::IoError(e))?;
        
        // 提取所有UI元素
        let fetcher = crate::LocatorFetcher::new();
        let elements = fetcher.fetch_elements_from_xml(&xml_content)?;
        
        // 获取locator定义
        let locator = self.locators.get(locator_name)
            .ok_or_else(|| TkeError::ElementNotFound(format!("Locator '{}' 未定义", locator_name)))?;
        
        // 查找匹配的元素
        let element = self.find_element_by_locator(&elements, locator)?;
        
        Ok(element.center())
    }
    
    // 根据locator定义查找元素
    fn find_element_by_locator(&self, elements: &[UIElement], locator: &Locator) -> Result<UIElement> {
        debug!("开始查找元素，locator: {:?}", locator);
        debug!("当前有{}个UI元素可供匹配", elements.len());
        
        // 策略1: 根据matchStrategy优先匹配
        if let Some(ref strategy) = locator.match_strategy {
            debug!("使用匹配策略: {}", strategy);
            match strategy.as_str() {
                "resourceId" => {
                    if let Some(ref resource_id) = locator.resource_id {
                        if let Some(element) = self.find_by_resource_id(elements, resource_id) {
                            debug!("通过resource_id匹配找到元素");
                            return Ok(element);
                        }
                    }
                }
                "className" => {
                    if let Some(ref class_name) = locator.class_name {
                        if let Some(element) = self.find_by_class_name(elements, class_name) {
                            debug!("通过class_name匹配找到元素");
                            return Ok(element);
                        }
                    }
                }
                "text" => {
                    if let Some(ref text) = locator.text {
                        if let Some(element) = self.find_by_text(elements, text) {
                            debug!("通过text匹配找到元素");
                            return Ok(element);
                        }
                    }
                }
                "xpath" => {
                    if let Some(ref xpath) = locator.xpath {
                        if let Some(element) = self.find_by_xpath(elements, xpath) {
                            debug!("通过xpath匹配找到元素");
                            return Ok(element);
                        }
                    }
                }
                _ => debug!("未知的匹配策略: {}", strategy),
            }
        }
        
        // 策略2: 精确匹配
        if let Some(element) = self.find_by_exact_match(elements, locator) {
            debug!("通过精确匹配找到元素");
            return Ok(element);
        }
        
        // 策略3: resource_id匹配
        if let Some(ref resource_id) = locator.resource_id {
            if let Some(element) = self.find_by_resource_id(elements, resource_id) {
                debug!("通过resource_id匹配找到元素");
                return Ok(element);
            }
        }
        
        // 策略4: text匹配
        if let Some(ref text) = locator.text {
            if let Some(element) = self.find_by_text(elements, text) {
                debug!("通过text匹配找到元素");
                return Ok(element);
            }
        }
        
        // 策略5: 类名和位置的模糊匹配
        if let (Some(ref class_name), Some(ref bounds_vec)) = (&locator.class_name, &locator.bounds) {
            if bounds_vec.len() == 4 {
                let bounds = Bounds::new(bounds_vec[0], bounds_vec[1], bounds_vec[2], bounds_vec[3]);
                if let Some(element) = self.find_by_class_and_position(elements, class_name, &bounds) {
                    debug!("通过类名和位置模糊匹配找到元素");
                    return Ok(element);
                }
            }
        }
        
        // 策略6: 仅基于类名的匹配
        if let Some(ref class_name) = locator.class_name {
            if let Some(element) = self.find_by_class_name(elements, class_name) {
                debug!("通过类名匹配找到元素");
                return Ok(element);
            }
        }
        
        // 策略7: xpath匹配
        if let Some(ref xpath) = locator.xpath {
            if let Some(element) = self.find_by_xpath(elements, xpath) {
                debug!("通过xpath匹配找到元素");
                return Ok(element);
            }
        }
        
        debug!("所有匹配策略都未找到元素");
        Err(TkeError::ElementNotFound("所有匹配策略都未找到元素".to_string()))
    }
    
    // 精确匹配
    fn find_by_exact_match(&self, elements: &[UIElement], locator: &Locator) -> Option<UIElement> {
        elements.iter().find(|e| {
            (locator.text.is_none() || e.text == locator.text) &&
            (locator.resource_id.is_none() || e.resource_id == locator.resource_id) &&
            (locator.class_name.is_none() || Some(&e.class_name) == locator.class_name.as_ref()) &&
            (locator.xpath.is_none() || e.xpath == locator.xpath)
        }).cloned()
    }
    
    // resource_id匹配
    fn find_by_resource_id(&self, elements: &[UIElement], resource_id: &str) -> Option<UIElement> {
        elements.iter().find(|e| {
            if let Some(ref id) = e.resource_id {
                id == resource_id || id.contains(resource_id) || resource_id.contains(id)
            } else {
                false
            }
        }).cloned()
    }
    
    // xpath匹配
    fn find_by_xpath(&self, elements: &[UIElement], xpath: &str) -> Option<UIElement> {
        elements.iter().find(|e| {
            if let Some(ref e_xpath) = e.xpath {
                e_xpath == xpath
            } else {
                false
            }
        }).cloned()
    }
    
    // text匹配
    fn find_by_text(&self, elements: &[UIElement], text: &str) -> Option<UIElement> {
        elements.iter().find(|e| {
            if let Some(ref t) = e.text {
                t == text || t.contains(text) || text.contains(t)
            } else {
                false
            }
        }).cloned()
    }
    
    // 类名匹配
    fn find_by_class_name(&self, elements: &[UIElement], class_name: &str) -> Option<UIElement> {
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
    fn find_by_class_and_position(&self, elements: &[UIElement], class_name: &str, original_bounds: &Bounds) -> Option<UIElement> {
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
    
    // 指令2: 根据图像locator查找元素
    pub fn find_image_element(&self, locator_name: &str) -> Result<Point> {
        // 获取locator定义
        let locator = self.locators.get(locator_name)
            .ok_or_else(|| TkeError::ElementNotFound(format!("Locator '{}' 未定义", locator_name)))?;
        
        // 确保是图像类型
        if !matches!(locator.locator_type, LocatorType::Image) {
            return Err(TkeError::InvalidArgument(format!("Locator '{}' 不是图像类型", locator_name)));
        }
        
        let template_path = if let Some(ref path) = locator.path {
            self.project_path.join(path)
        } else {
            return Err(TkeError::InvalidArgument("图像locator缺少path字段".to_string()));
        };
        
        let screenshot_path = self.project_path.join("workarea").join("current_screenshot.png");
        
        // 执行模板匹配
        self.template_match(&screenshot_path, &template_path)
    }
    
    // 模板匹配
    fn template_match(&self, screenshot_path: &PathBuf, template_path: &PathBuf) -> Result<Point> {
        // 加载图像
        let screenshot = image::open(screenshot_path)
            .map_err(|e| TkeError::ImageError(format!("加载截图失败: {}", e)))?;
        let template = image::open(template_path)
            .map_err(|e| TkeError::ImageError(format!("加载模板失败: {}", e)))?;
        
        // 转换为灰度图进行匹配
        let screenshot_gray = screenshot.to_luma8();
        let template_gray = template.to_luma8();
        
        // 执行模板匹配
        let result = match_template(
            &screenshot_gray,
            &template_gray,
            MatchTemplateMethod::CrossCorrelationNormalized
        );
        
        // 找到最佳匹配位置
        let extremes = find_extremes(&result);
        
        // 检查置信度
        if extremes.max_value < self.confidence_threshold {
            return Err(TkeError::ElementNotFound(
                format!("图像匹配置信度不足: {:.3} < {:.3}", 
                       extremes.max_value, self.confidence_threshold)
            ));
        }
        
        // 计算中心坐标
        let (x, y) = extremes.max_value_location;
        let center_x = x as i32 + template.width() as i32 / 2;
        let center_y = y as i32 + template.height() as i32 / 2;
        
        info!("图像匹配成功，中心坐标: ({}, {}), 置信度: {:.3}", 
              center_x, center_y, extremes.max_value);
        
        Ok(Point::new(center_x, center_y))
    }
    
    // 直接根据文本查找元素
    pub fn find_element_by_text(&self, text: &str) -> Result<Point> {
        // 获取当前UI树
        let ui_tree_path = self.project_path.join("workarea").join("current_ui_tree.xml");
        let xml_content = std::fs::read_to_string(&ui_tree_path)
            .map_err(|e| TkeError::IoError(e))?;
        
        // 提取所有UI元素
        let fetcher = crate::LocatorFetcher::new();
        let elements = fetcher.fetch_elements_from_xml(&xml_content)?;
        
        // 查找匹配文本的元素
        let element = elements.iter()
            .find(|e| e.matches_text(text))
            .ok_or_else(|| TkeError::ElementNotFound(format!("未找到包含文本 '{}' 的元素", text)))?;
        
        Ok(element.center())
    }
    
    // 根据坐标查找元素
    pub fn find_element_at_position(&self, x: i32, y: i32) -> Result<UIElement> {
        // 获取当前UI树
        let ui_tree_path = self.project_path.join("workarea").join("current_ui_tree.xml");
        let xml_content = std::fs::read_to_string(&ui_tree_path)
            .map_err(|e| TkeError::IoError(e))?;
        
        // 提取所有UI元素
        let fetcher = crate::LocatorFetcher::new();
        let elements = fetcher.fetch_elements_from_xml(&xml_content)?;
        
        // 查找包含该坐标的元素
        let element = elements.iter()
            .find(|e| {
                e.bounds.x1 <= x && x <= e.bounds.x2 &&
                e.bounds.y1 <= y && y <= e.bounds.y2
            })
            .ok_or_else(|| TkeError::ElementNotFound(format!("坐标 ({}, {}) 处未找到元素", x, y)))?;
        
        Ok(element.clone())
    }
}