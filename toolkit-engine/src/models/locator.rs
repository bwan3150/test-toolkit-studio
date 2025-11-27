// 元素定位器数据结构 - 统一的元素模型

use serde::{Deserialize, Serialize};
use super::Bounds;

/// 定位策略枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorStrategy {
    // XML 类定位
    XPath,
    ResourceId,
    Text,
    ContentDesc,
    ClassName,
    // OCR 文字识别
    Ocr,
    // 图片模板匹配
    Img,
    // 自动选择（按优先级：xml类 → ocr → img）
    Auto,
}

impl Default for LocatorStrategy {
    fn default() -> Self {
        LocatorStrategy::Auto
    }
}

impl LocatorStrategy {
    /// 从字符串解析策略
    pub fn from_str(s: &str) -> Self {
        match s {
            "xpath" => Self::XPath,
            "resourceId" => Self::ResourceId,
            "text" => Self::Text,
            "contentDesc" => Self::ContentDesc,
            "className" => Self::ClassName,
            "ocr" => Self::Ocr,
            "img" => Self::Img,
            _ => Self::Auto,
        }
    }
}

/// 统一的元素定位器
/// 一个元素可以同时拥有多种定位方式（xpath/resourceId/text/ocr/img等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locator {
    /// 元素名称
    pub name: String,
    /// 备注说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    // ========== XML 定位属性 ==========
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    // ========== OCR 定位属性 ==========
    /// OCR 识别的文字
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_text: Option<String>,

    // ========== 图片定位属性 ==========
    /// 图片模板路径（相对于项目根目录）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img_path: Option<String>,

    // ========== 通用属性 ==========
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Locator {
    /// 检查是否有任何 XML 类定位属性
    pub fn has_xml_locator(&self) -> bool {
        self.xpath.is_some()
            || self.resource_id.is_some()
            || self.text.is_some()
            || self.content_desc.is_some()
            || self.class_name.is_some()
    }

    /// 检查是否有 OCR 定位属性
    pub fn has_ocr_locator(&self) -> bool {
        self.ocr_text.is_some()
    }

    /// 检查是否有图片定位属性
    pub fn has_img_locator(&self) -> bool {
        self.img_path.is_some()
    }

    /// 获取所有可用的定位策略
    pub fn available_strategies(&self) -> Vec<LocatorStrategy> {
        let mut strategies = Vec::new();

        if self.resource_id.is_some() {
            strategies.push(LocatorStrategy::ResourceId);
        }
        if self.xpath.is_some() {
            strategies.push(LocatorStrategy::XPath);
        }
        if self.text.is_some() {
            strategies.push(LocatorStrategy::Text);
        }
        if self.content_desc.is_some() {
            strategies.push(LocatorStrategy::ContentDesc);
        }
        if self.class_name.is_some() {
            strategies.push(LocatorStrategy::ClassName);
        }
        if self.ocr_text.is_some() {
            strategies.push(LocatorStrategy::Ocr);
        }
        if self.img_path.is_some() {
            strategies.push(LocatorStrategy::Img);
        }

        strategies
    }
}
