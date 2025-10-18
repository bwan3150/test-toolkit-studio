// Recognizer模块 - 负责元素识别(XML匹配、图像匹配、文本匹配)

mod xml;
mod image;
mod text;

use crate::{Result, Locator, Point};
use std::collections::HashMap;
use std::path::PathBuf;

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
            confidence_threshold: 0.60,
        })
    }

    // 加载项目的locator定义
    fn load_locators(project_path: &PathBuf) -> Result<HashMap<String, Locator>> {
        let locator_file = project_path.join("locator").join("element.json");

        if !locator_file.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(&locator_file)
            .map_err(|e| crate::TkeError::IoError(e))?;

        let locators: HashMap<String, Locator> = serde_json::from_str(&content)
            .map_err(|e| crate::TkeError::JsonError(e))?;

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

    // === XML 元素查找 ===

    /// 根据XML locator查找元素
    pub fn find_xml_element(&self, locator_name: &str) -> Result<Point> {
        xml::find_by_locator(&self.project_path, &self.locators, locator_name)
    }

    // === 图像匹配查找 ===

    /// 根据图像locator查找元素（用于脚本，返回Point）
    pub fn find_image_element(&self, locator_name: &str) -> Result<Point> {
        image::find_by_locator(&self.project_path, &self.locators, locator_name, self.confidence_threshold)
    }

    /// 根据图像locator查找元素（用于CLI，直接输出JSON）
    pub fn find_image_element_json(&self, locator_name: &str, threshold: f32) -> Result<()> {
        image::find_by_locator_json(&self.project_path, &self.locators, locator_name, threshold)
    }

    // === 文本查找 ===

    /// 直接根据文本查找元素
    pub fn find_element_by_text(&self, text: &str) -> Result<Point> {
        text::find_by_text(&self.project_path, text)
    }
}
