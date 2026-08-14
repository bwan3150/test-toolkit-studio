// UI元素数据结构

use serde::{Deserialize, Serialize};
use super::{Point, Bounds};

/// UI元素 - 从XML UI树解析出的元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub index: usize,
    pub class_name: String,
    pub bounds: Bounds,
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
    pub xpath: Option<String>,
    /// `<select>` 的可选项（web 独有）。闭合状态下 `<option>` 自身采不到，
    /// 只能挂在 select 上带出来——没有它 AI 就不知道这个下拉框能选什么
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// 用于前端渲染的z-index，基于元素面积计算
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<usize>,
    /// 父元素的index
    #[serde(skip)]
    pub parent_index: Option<usize>,
    /// 层级深度（从0开始）
    #[serde(skip)]
    pub depth: usize,
    /// 在同一父元素下，相同class的第几个（从1开始）
    #[serde(skip)]
    pub sibling_index: usize,
}

impl UIElement {
    /// 获取中心点坐标
    pub fn center(&self) -> Point {
        self.bounds.center()
    }

    /// 判断元素是否可见
    pub fn is_visible(&self) -> bool {
        self.bounds.is_visible()
    }

    /// 判断元素是否匹配给定文本（不区分大小写，包含匹配）
    pub fn matches_text(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();

        if let Some(ref t) = self.text {
            if t.to_lowercase().contains(&text_lower) {
                return true;
            }
        }

        if let Some(ref desc) = self.content_desc {
            if desc.to_lowercase().contains(&text_lower) {
                return true;
            }
        }

        if let Some(ref hint) = self.hint {
            if hint.to_lowercase().contains(&text_lower) {
                return true;
            }
        }

        false
    }

    /// 转换为AI可读的文本格式
    pub fn to_ai_text(&self) -> String {
        let simple_class = self.class_name.split('.').last().unwrap_or(&self.class_name);
        let mut attrs = Vec::new();

        if let Some(ref text) = self.text {
            attrs.push(format!("text={}", text));
        }
        if let Some(ref hint) = self.hint {
            attrs.push(format!("hint={}", hint));
        }
        if let Some(ref desc) = self.content_desc {
            attrs.push(format!("content-desc={}", desc));
        }
        if let Some(ref id) = self.resource_id {
            let id_part = id.split(':').last().unwrap_or(id);
            attrs.push(format!("id={}", id_part));
        }

        if self.checked {
            attrs.push("checked=true".to_string());
        }
        if self.focused {
            attrs.push("focused=true".to_string());
        }
        if self.selected {
            attrs.push("selected=true".to_string());
        }
        if !self.enabled {
            attrs.push("enabled=false".to_string());
        }

        if attrs.is_empty() {
            format!("{}()", simple_class)
        } else {
            format!("{}({})", simple_class, attrs.join(", "))
        }
    }
}
