// 屏幕信息模型

use serde::{Deserialize, Serialize};

/// 屏幕状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenState {
    /// OCR 提取的文字列表
    pub ocr_texts: Vec<OcrText>,

    /// XML 提取的可交互元素
    pub ui_elements: Vec<UiElement>,

    /// 合并后的元素列表 (带统一序号)
    pub merged_elements: Vec<MergedElement>,

    /// 截图路径
    pub screenshot_path: String,

    /// UI 树 XML 路径
    pub xml_path: String,
}

/// OCR 识别的文字
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrText {
    /// 文字内容
    pub text: String,

    /// 边界框 (四个点的坐标)
    pub bbox: Vec<[f32; 2]>,

    /// 置信度
    pub confidence: f32,

    /// 中心点坐标
    pub center: (i32, i32),
}

/// UI 元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    /// 元素索引
    pub index: usize,

    /// 类名
    pub class_name: String,

    /// 边界
    pub bounds: Bounds,

    /// 文本
    pub text: Option<String>,

    /// 内容描述
    pub content_desc: Option<String>,

    /// 资源 ID
    pub resource_id: Option<String>,

    /// 提示文本
    pub hint: Option<String>,

    /// 是否可点击
    pub clickable: bool,

    /// 是否可勾选
    pub checkable: bool,

    /// 是否已勾选
    pub checked: bool,

    /// 是否可获取焦点
    pub focusable: bool,

    /// 是否已获取焦点
    pub focused: bool,

    /// 是否可滚动
    pub scrollable: bool,

    /// 是否已选中
    pub selected: bool,

    /// 是否启用
    pub enabled: bool,

    /// XPath
    pub xpath: String,
}

/// 边界框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Bounds {
    /// 计算中心点
    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }
}

/// 合并后的元素 (统一编号, 用于给 AI 看)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedElement {
    /// 统一序号
    pub id: usize,

    /// 元素类型
    pub element_type: ElementType,

    /// 可读描述 (给 AI 看)
    pub description: String,

    /// 原始索引 (用于查字典)
    pub original_index: usize,
}

/// 元素类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    /// OCR 文字
    Ocr,
    /// XML UI 元素
    Xml,
}
