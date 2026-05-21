// Recognizer 模块 - 负责元素识别（XML/图像/OCR）

mod xml;
mod image;
mod text;
pub mod ocr;

use crate::{Result, TkeError, Locator, LocatorStrategy, Point};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// 元素识别器
pub struct Recognizer {
    project_path: PathBuf,
    locators: HashMap<String, Locator>,
    confidence_threshold: f32,
}

impl Recognizer {
    /// 创建新的识别器实例
    pub fn new(project_path: PathBuf) -> Result<Self> {
        let locators = Self::load_locators(&project_path)?;

        Ok(Self {
            project_path,
            locators,
            confidence_threshold: 0.60,
        })
    }

    /// 加载项目的 locator 定义
    fn load_locators(project_path: &PathBuf) -> Result<HashMap<String, Locator>> {
        let locator_file = project_path.join("locator").join("element.json");

        if !locator_file.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(&locator_file)
            .map_err(|e| TkeError::IoError(e))?;

        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let locators: HashMap<String, Locator> = serde_json::from_str(&content)
            .map_err(|e| TkeError::JsonError(e))?;

        info!("已加载 {} 个元素定义", locators.len());

        Ok(locators)
    }

    /// 重新加载 locator 定义
    pub fn reload_locators(&mut self) -> Result<()> {
        self.locators = Self::load_locators(&self.project_path)?;
        Ok(())
    }

    /// 设置置信度阈值（用于图像匹配）
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }

    /// 统一的元素查找入口
    pub async fn find_element(
        &self,
        locator_name: &str,
        strategy: LocatorStrategy,
    ) -> Result<Point> {
        let locator = self.locators.get(locator_name)
            .ok_or_else(|| TkeError::ElementNotFound(
                format!("元素 '{}' 未定义", locator_name)
            ))?;

        debug!("查找元素 '{}', 策略: {:?}", locator_name, strategy);

        match strategy {
            LocatorStrategy::Auto => self.find_auto(locator).await,
            LocatorStrategy::XPath => xml::find_by_xpath(&self.project_path, locator),
            LocatorStrategy::ResourceId => xml::find_by_resource_id(&self.project_path, locator),
            LocatorStrategy::Text => xml::find_by_text(&self.project_path, locator),
            LocatorStrategy::ContentDesc => xml::find_by_content_desc(&self.project_path, locator),
            LocatorStrategy::ClassName => xml::find_by_class_name(&self.project_path, locator),
            LocatorStrategy::Ocr => ocr::find_by_ocr(&self.project_path, locator).await,
            LocatorStrategy::Img => image::find_by_image(&self.project_path, locator, self.confidence_threshold),
        }
    }

    /// 自动选择策略查找: xml类 → ocr → img
    async fn find_auto(&self, locator: &Locator) -> Result<Point> {
        debug!("自动选择策略查找元素: {}", locator.name);

        // 1. 尝试 XML 类（按优先级）
        if locator.resource_id.is_some() {
            debug!("尝试 ResourceId 策略...");
            if let Ok(p) = xml::find_by_resource_id(&self.project_path, locator) {
                info!("ResourceId 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        if locator.xpath.is_some() {
            debug!("尝试 XPath 策略...");
            if let Ok(p) = xml::find_by_xpath(&self.project_path, locator) {
                info!("XPath 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        if locator.text.is_some() {
            debug!("尝试 Text 策略...");
            if let Ok(p) = xml::find_by_text(&self.project_path, locator) {
                info!("Text 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        if locator.content_desc.is_some() {
            debug!("尝试 ContentDesc 策略...");
            if let Ok(p) = xml::find_by_content_desc(&self.project_path, locator) {
                info!("ContentDesc 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        if locator.class_name.is_some() {
            debug!("尝试 ClassName 策略...");
            if let Ok(p) = xml::find_by_class_name(&self.project_path, locator) {
                info!("ClassName 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        // 2. 尝试 OCR
        if locator.ocr_text.is_some() {
            debug!("尝试 OCR 策略...");
            if let Ok(p) = ocr::find_by_ocr(&self.project_path, locator).await {
                info!("OCR 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        // 3. 尝试图片匹配
        if locator.img_path.is_some() {
            debug!("尝试 Img 策略...");
            if let Ok(p) = image::find_by_image(&self.project_path, locator, self.confidence_threshold) {
                info!("Img 策略成功找到元素 '{}'", locator.name);
                return Ok(p);
            }
        }

        Err(TkeError::ElementNotFound(
            format!("所有定位策略均失败: {}", locator.name)
        ))
    }

    /// 直接根据文本查找元素（用于脚本中的纯文本参数）
    pub fn find_element_by_text(&self, text: &str) -> Result<Point> {
        text::find_by_text(&self.project_path, text)
    }

    /// 获取 locator 定义（用于 CLI）
    pub fn get_locator(&self, name: &str) -> Option<&Locator> {
        self.locators.get(name)
    }

    /// 获取所有 locator 名称
    pub fn get_locator_names(&self) -> Vec<&String> {
        self.locators.keys().collect()
    }
}
