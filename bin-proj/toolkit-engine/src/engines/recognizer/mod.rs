// Recognizer 模块 - 负责元素识别（结构标识/OCR/图像）
// 平台感知：按 -d 推断平台，取元素对应平台通道的标识做匹配
//   element_path: 元素库 element.json（img 通道路径相对其所在目录）
//   workarea:     页面采集工作区（截图 + 归一化结构文件）

mod xml;
mod image;
mod text;
pub mod ocr;

use crate::{Result, TkeError, Locator, LocatorStrategy, Platform, Point, Bounds};
use crate::models::AndroidLocator;
use crate::utils::Workarea;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// 元素库默认查找路径（相对当前目录）
const DEFAULT_ELEMENT_PATHS: &[&str] = &["element.json", "locator/element.json"];

/// 元素识别器
pub struct Recognizer {
    /// 元素库文件所在目录（img 通道的路径基准）
    element_dir: PathBuf,
    workarea: Workarea,
    platform: Platform,
    locators: HashMap<String, Locator>,
    confidence_threshold: f32,
}

impl Recognizer {
    /// 创建识别器
    /// element_path 为 None 时按默认路径查找（./element.json → ./locator/element.json），
    /// 找不到元素库则为空库（仅坐标/文本定位可用）
    pub fn new(element_path: Option<&Path>, workarea: Workarea, platform: Platform) -> Result<Self> {
        let resolved = match element_path {
            Some(p) => Some(p.to_path_buf()),
            None => DEFAULT_ELEMENT_PATHS
                .iter()
                .map(PathBuf::from)
                .find(|p| p.exists()),
        };

        let (element_dir, locators) = match resolved {
            Some(path) => {
                let dir = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                (dir, Self::load_library(&path)?)
            }
            None => {
                warn!("未找到元素库文件，元素引用将不可用");
                (PathBuf::from("."), HashMap::new())
            }
        };

        Ok(Self {
            element_dir,
            workarea,
            platform,
            locators,
            confidence_threshold: 0.60,
        })
    }

    /// 加载元素库 (schema v3: {elements})，元素名 = key
    fn load_library(element_path: &Path) -> Result<HashMap<String, Locator>> {
        if !element_path.exists() {
            return Err(TkeError::InvalidArgument(format!(
                "元素库文件不存在: {}", element_path.display()
            )));
        }

        let content = std::fs::read_to_string(element_path)
            .map_err(|e| TkeError::IoError(e))?;

        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let library: crate::models::ElementLibrary = serde_json::from_str(&content)
            .map_err(|e| TkeError::JsonError(e))?;

        let mut locators = library.elements;
        for (name, locator) in locators.iter_mut() {
            locator.name = name.clone();
        }

        info!("已加载 {} 个元素定义", locators.len());

        Ok(locators)
    }

    /// 设置置信度阈值（用于图像匹配）
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }

    /// 工作区引用
    pub fn workarea(&self) -> &Workarea {
        &self.workarea
    }

    /// 当前平台
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// 统一的元素查找入口（仅中心点）
    pub async fn find_element(
        &self,
        name: &str,
        strategy: LocatorStrategy,
    ) -> Result<Point> {
        self.find_element_detailed(name, strategy).await.map(|(p, _)| p)
    }

    /// 统一的元素查找入口，返回 (中心点, 实时边界框)
    /// 边界框来自当前页面实际匹配到的元素（结构=元素框, ocr=文字框, img=模板尺寸）
    pub async fn find_element_detailed(
        &self,
        name: &str,
        strategy: LocatorStrategy,
    ) -> Result<(Point, Bounds)> {
        let locator = self.locators.get(name)
            .ok_or_else(|| TkeError::ElementNotFound(
                format!("元素 '{}' 未定义", name)
            ))?;

        debug!("查找元素 '{}', 平台: {}, 策略: {:?}", name, self.platform.name(), strategy);

        // 取当前平台的结构标识（统一映射为匹配引擎形态）
        let structural = locator.structural(self.platform);

        let ui_tree = self.workarea.ui_tree_path();
        let screenshot = self.workarea.screenshot_path();

        let missing = || TkeError::ElementNotFound(format!(
            "元素 '{}' 未定义 {} 通道", name, self.platform.name()
        ));

        match strategy {
            LocatorStrategy::Auto => self.find_auto(locator, structural.as_ref()).await,
            LocatorStrategy::XPath => {
                xml::find_by_xpath(&ui_tree, name, structural.as_ref().ok_or_else(missing)?)
            }
            LocatorStrategy::ResourceId => {
                xml::find_by_resource_id(&ui_tree, name, structural.as_ref().ok_or_else(missing)?)
            }
            LocatorStrategy::Text => {
                xml::find_by_text(&ui_tree, name, structural.as_ref().ok_or_else(missing)?)
            }
            LocatorStrategy::ContentDesc => {
                xml::find_by_content_desc(&ui_tree, name, structural.as_ref().ok_or_else(missing)?)
            }
            LocatorStrategy::ClassName => {
                xml::find_by_class_name(&ui_tree, name, structural.as_ref().ok_or_else(missing)?)
            }
            LocatorStrategy::Ocr => ocr::find_by_ocr(&screenshot, locator).await,
            LocatorStrategy::Img => {
                image::find_by_image(&screenshot, &self.element_dir, locator, self.confidence_threshold)
            }
        }
    }

    /// 自动选择策略查找: 平台结构标识 → ocr → img
    async fn find_auto(
        &self,
        locator: &Locator,
        structural: Option<&AndroidLocator>,
    ) -> Result<(Point, Bounds)> {
        debug!("自动选择策略查找元素: {}", locator.name);

        let ui_tree = self.workarea.ui_tree_path();
        let screenshot = self.workarea.screenshot_path();

        // 1. 尝试结构标识（按优先级）
        if let Some(st) = structural {
            if st.resource_id.is_some() {
                if let Ok(r) = xml::find_by_resource_id(&ui_tree, &locator.name, st) {
                    info!("ResourceId/Id 策略成功找到元素 '{}'", locator.name);
                    return Ok(r);
                }
            }
            if st.xpath.is_some() {
                if let Ok(r) = xml::find_by_xpath(&ui_tree, &locator.name, st) {
                    info!("XPath 策略成功找到元素 '{}'", locator.name);
                    return Ok(r);
                }
            }
            if st.text.is_some() {
                if let Ok(r) = xml::find_by_text(&ui_tree, &locator.name, st) {
                    info!("Text 策略成功找到元素 '{}'", locator.name);
                    return Ok(r);
                }
            }
            if st.content_desc.is_some() {
                if let Ok(r) = xml::find_by_content_desc(&ui_tree, &locator.name, st) {
                    info!("ContentDesc/Aria 策略成功找到元素 '{}'", locator.name);
                    return Ok(r);
                }
            }
            if st.class_name.is_some() {
                if let Ok(r) = xml::find_by_class_name(&ui_tree, &locator.name, st) {
                    info!("ClassName/Tag 策略成功找到元素 '{}'", locator.name);
                    return Ok(r);
                }
            }
        }

        // 2. 尝试 OCR 通道
        if locator.ocr.is_some() {
            if let Ok(r) = ocr::find_by_ocr(&screenshot, locator).await {
                info!("OCR 策略成功找到元素 '{}'", locator.name);
                return Ok(r);
            }
        }

        // 3. 尝试图像通道
        if locator.img.is_some() {
            if let Ok(r) = image::find_by_image(&screenshot, &self.element_dir, locator, self.confidence_threshold) {
                info!("Img 策略成功找到元素 '{}'", locator.name);
                return Ok(r);
            }
        }

        Err(TkeError::ElementNotFound(
            format!("所有定位策略均失败: {}", locator.name)
        ))
    }

    /// 直接根据文本查找元素（用于脚本中的纯文本参数）
    pub fn find_element_by_text(&self, text: &str) -> Result<(Point, Bounds)> {
        text::find_by_text(&self.workarea.ui_tree_path(), text)
    }

    /// 获取元素定义
    pub fn get_locator(&self, name: &str) -> Option<&Locator> {
        self.locators.get(name)
    }

    /// 获取所有元素名称
    pub fn get_locator_names(&self) -> Vec<&String> {
        self.locators.keys().collect()
    }
}
