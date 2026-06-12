// 元素定位器数据结构 - 统一的元素模型 (schema v3)
//
// 元素库 element.json:
// {
//   "elements": { "<元素名>": { ...Locator } }
// }
//
//   - elements 的 key 即元素名（唯一主键，人类可读，脚本中 {元素名} 直接引用）
//   - 每个元素按平台分通道存标识 + 两条通用识别通道，字段可为 null 但不可缺：
//       android: uiautomator 标识    ios: WDA 标识(预留)    web: DOM 标识
//       img: UI 元素图（图像模板匹配，路径相对 element.json 所在目录）
//       ocr: 文字内容（OCR 识别）
//   - 不存储 bounds/clickable 等设备相关快照，截图标注所需的元素框
//     运行时从实际匹配到的元素获取

use serde::{Deserialize, Serialize};

/// 目标平台（由 -d 设备 ID 推断）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    Android,
    Ios,
    Web,
}

impl Platform {
    /// 从设备 ID 推断平台: web* → Web, wda:*/iOS UDID → Ios, 其他 → Android
    pub fn from_device(device_id: Option<&str>) -> Self {
        match device_id {
            Some(d) if d == "web" || d.starts_with("web:") => Platform::Web,
            Some(d) if d.starts_with("wda:") || is_ios_udid(d) => Platform::Ios,
            _ => Platform::Android,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Ios => "ios",
            Platform::Web => "web",
        }
    }
}

/// 是否为 iOS 设备 UDID（现代格式: 8位hex-16位hex, 如 00008101-000C75842192001E）
fn is_ios_udid(d: &str) -> bool {
    d.len() == 25
        && d.as_bytes()[8] == b'-'
        && d.chars().enumerate().all(|(i, c)| i == 8 || c.is_ascii_hexdigit())
}

/// 定位策略枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocatorStrategy {
    // 结构标识类定位（android/web 通道字段）
    XPath,
    ResourceId,
    Text,
    ContentDesc,
    ClassName,
    // OCR 文字识别
    Ocr,
    // 图片模板匹配
    Img,
    // 自动选择（按优先级：结构标识 → ocr → img）
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

/// Android (uiautomator) 标识
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AndroidLocator {
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

/// iOS (XCUI/WDA) 标识
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IosLocator {
    #[serde(default)]
    pub xpath: Option<String>,
    /// accessibility identifier (XCUI name 属性)
    #[serde(default)]
    pub name: Option<String>,
    /// accessibility label
    #[serde(default)]
    pub label: Option<String>,
    /// 元素值（文本框内容/静态文本）
    #[serde(default)]
    pub value: Option<String>,
    /// 元素类型 (XCUIElementTypeButton 等)
    #[serde(default)]
    pub class_name: Option<String>,
}

/// Web (DOM) 标识
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebLocator {
    /// CSS 选择器（预留：需驱动在线查找，当前离线匹配不使用）
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub xpath: Option<String>,
    /// DOM 元素 id 属性
    #[serde(default)]
    pub id: Option<String>,
    /// 直接文本内容
    #[serde(default)]
    pub text: Option<String>,
    /// aria-label
    #[serde(default)]
    pub aria: Option<String>,
    /// 标签名 (button/a/input...)
    #[serde(default)]
    pub tag: Option<String>,
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

    // ========== 各平台标识（可 null 不可缺） ==========
    #[serde(default)]
    pub android: Option<AndroidLocator>,
    /// iOS WDA 标识
    #[serde(default)]
    pub ios: Option<IosLocator>,
    #[serde(default)]
    pub web: Option<WebLocator>,

    // ========== 通用识别通道（可 null 不可缺） ==========
    /// UI 元素图路径（相对 element.json 所在目录）
    #[serde(default)]
    pub img: Option<String>,
    /// OCR 文字内容
    #[serde(default)]
    pub ocr: Option<String>,
}

impl Locator {
    /// 取当前平台的结构标识，统一映射为 AndroidLocator 形态供匹配引擎使用
    /// （web/ios 页面均已归一化为 uiautomator 风格 XML:
    ///   web: id→resource-id, aria→content-desc, tag→class
    ///   ios: name→resource-id, label→content-desc, value|label→text, type→class）
    pub fn structural(&self, platform: Platform) -> Option<AndroidLocator> {
        let loc = match platform {
            Platform::Android => self.android.clone(),
            Platform::Web => self.web.as_ref().map(|w| AndroidLocator {
                xpath: w.xpath.clone(),
                resource_id: w.id.clone(),
                text: w.text.clone(),
                content_desc: w.aria.clone(),
                class_name: w.tag.clone(),
            }),
            Platform::Ios => self.ios.as_ref().map(|i| AndroidLocator {
                xpath: i.xpath.clone(),
                resource_id: i.name.clone(),
                text: i.value.clone().or_else(|| i.label.clone()),
                content_desc: i.label.clone(),
                class_name: i.class_name.clone(),
            }),
        };
        // 字段全空的通道视为无结构标识（否则会无条件匹配任意元素）
        // class_name 单独存在不构成有效标识（页面上同类元素遍地都是）
        loc.filter(|l| {
            l.xpath.is_some() || l.resource_id.is_some() || l.text.is_some() || l.content_desc.is_some()
        })
    }
}

/// 元素库文件 (element.json 顶层结构)
#[derive(Debug, Default, Deserialize)]
pub struct ElementLibrary {
    #[serde(default)]
    pub elements: std::collections::HashMap<String, Locator>,
}
