// TKE 命令定义

use serde::{Deserialize, Serialize};

/// TKE 截图返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub success: bool,
    pub screenshot: String,
    pub xml: String,
}

/// TKE 操作返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// TKE OCR 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub texts: Vec<OcrTextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTextItem {
    pub text: String,
    pub bbox: Vec<[f32; 2]>,
    pub confidence: f32,
}

/// TKE Fetcher 提取 UI 元素返回
pub type UiElementsResult = Vec<UiElementItem>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElementItem {
    pub index: usize,
    pub class_name: String,
    pub bounds: BoundsItem,
    pub text: Option<String>,
    pub content_desc: Option<String>,
    pub resource_id: Option<String>,
    pub hint: Option<String>,
    pub clickable: bool,
    pub checkable: bool,
    pub checked: bool,
    pub focusable: bool,
    pub focused: bool,
    pub scrollable: bool,
    pub selected: bool,
    pub enabled: bool,
    pub xpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsItem {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}
