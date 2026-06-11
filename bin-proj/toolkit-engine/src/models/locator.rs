// 元素定位器数据结构 - 统一的元素模型 (schema v2)
//
// 元素库 element.json: { "<元素名>": { ...Locator } }
//   - key 即元素名，是唯一主键（人类可读，脚本中 {元素名} 直接引用）
//   - 一个完整元素包含多端标识 + 两条通用识别通道，字段可为 null 但不可缺：
//       xml: Android (uiautomator)    wda: iOS    dom: Web
//       img: UI 元素图（图像模板匹配，路径相对 element.json 所在目录）
//       ocr: 文字内容（OCR 识别）
//   - 不存储 bounds/clickable 等设备相关快照（换设备/分辨率即失效），
//     截图标注所需的元素框改为运行时从实际匹配到的元素获取

use serde::{Deserialize, Serialize};

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

/// Android XML (uiautomator) 标识
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XmlLocator {
    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content_desc: Option<String>,
    #[serde(default)]
    pub class_name: Option<String>,
}

impl XmlLocator {
    /// 是否有任何可用标识
    pub fn has_any(&self) -> bool {
        self.xpath.is_some()
            || self.resource_id.is_some()
            || self.text.is_some()
            || self.content_desc.is_some()
            || self.class_name.is_some()
    }
}

/// 统一的元素定位器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locator {
    /// 元素名（即元素库 key，加载时注入，不序列化进文件）
    #[serde(skip)]
    pub name: String,

    /// 描述（便于人工检索和 AI 选元素）
    #[serde(default)]
    pub desc: Option<String>,

    // ========== 多端标识（可 null 不可缺） ==========
    /// Android XML 标识
    #[serde(default)]
    pub xml: Option<XmlLocator>,
    /// iOS WDA 标识（预留）
    #[serde(default)]
    pub wda: Option<serde_json::Value>,
    /// Web DOM 标识（预留）
    #[serde(default)]
    pub dom: Option<serde_json::Value>,

    // ========== 通用识别通道（可 null 不可缺） ==========
    /// UI 元素图路径（相对 element.json 所在目录）
    #[serde(default)]
    pub img: Option<String>,
    /// OCR 文字内容
    #[serde(default)]
    pub ocr: Option<String>,
}

impl Locator {
    /// 获取所有可用的定位策略（按 auto 兜底顺序）
    pub fn available_strategies(&self) -> Vec<LocatorStrategy> {
        let mut strategies = Vec::new();

        if let Some(xml) = &self.xml {
            if xml.resource_id.is_some() {
                strategies.push(LocatorStrategy::ResourceId);
            }
            if xml.xpath.is_some() {
                strategies.push(LocatorStrategy::XPath);
            }
            if xml.text.is_some() {
                strategies.push(LocatorStrategy::Text);
            }
            if xml.content_desc.is_some() {
                strategies.push(LocatorStrategy::ContentDesc);
            }
            if xml.class_name.is_some() {
                strategies.push(LocatorStrategy::ClassName);
            }
        }
        if self.ocr.is_some() {
            strategies.push(LocatorStrategy::Ocr);
        }
        if self.img.is_some() {
            strategies.push(LocatorStrategy::Img);
        }

        strategies
    }
}
