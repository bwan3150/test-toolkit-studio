// Worker Parser - 解析屏幕信息并转换为 AI 友好格式

use crate::models::{
    ElementType, MergedElement, OcrText, ScreenState, UiElement,
};
use crate::tke::{OcrResult, UiElementsResult};
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

/// Worker Parser - 负责处理屏幕信息
pub struct WorkerParser {
    /// 元素字典 - 将统一 ID 映射到原始坐标
    element_map: HashMap<usize, ElementInfo>,
}

/// 元素信息 (用于字典查询)
#[derive(Debug, Clone)]
pub struct ElementInfo {
    /// 元素类型
    pub element_type: ElementType,
    /// 中心坐标
    pub center: (i32, i32),
    /// 边界 (可选)
    pub bounds: Option<(i32, i32, i32, i32)>,
}

impl WorkerParser {
    /// 创建新的 Worker Parser
    pub fn new() -> Self {
        Self {
            element_map: HashMap::new(),
        }
    }

    /// 解析屏幕状态
    ///
    /// 将 OCR 和 UI 元素合并,生成统一编号的元素列表
    pub fn parse_screen(
        &mut self,
        ocr_result: OcrResult,
        ui_elements: UiElementsResult,
        screenshot_path: String,
        xml_path: String,
    ) -> Result<ScreenState> {
        self.element_map.clear();

        // 转换 OCR 文字
        let ocr_texts: Vec<OcrText> = ocr_result
            .texts
            .into_iter()
            .map(|item| {
                let center = Self::calculate_center(&item.bbox);
                OcrText {
                    text: item.text,
                    bbox: item.bbox,
                    confidence: item.confidence,
                    center,
                }
            })
            .collect();

        // 转换 UI 元素
        let ui_elements: Vec<UiElement> = ui_elements
            .into_iter()
            .map(|item| UiElement {
                index: item.index,
                class_name: item.class_name,
                bounds: crate::models::Bounds {
                    x1: item.bounds.x1,
                    y1: item.bounds.y1,
                    x2: item.bounds.x2,
                    y2: item.bounds.y2,
                },
                text: item.text,
                content_desc: item.content_desc,
                resource_id: item.resource_id,
                hint: item.hint,
                clickable: item.clickable,
                checkable: item.checkable,
                checked: item.checked,
                focusable: item.focusable,
                focused: item.focused,
                scrollable: item.scrollable,
                selected: item.selected,
                enabled: item.enabled,
                xpath: item.xpath,
            })
            .collect();

        // 合并元素
        let merged_elements = self.merge_elements(&ocr_texts, &ui_elements);

        debug!("解析屏幕: {} OCR 文字, {} UI 元素, {} 合并元素",
            ocr_texts.len(), ui_elements.len(), merged_elements.len());

        Ok(ScreenState {
            ocr_texts,
            ui_elements,
            merged_elements,
            screenshot_path,
            xml_path,
        })
    }

    /// 合并 OCR 和 UI 元素
    fn merge_elements(
        &mut self,
        ocr_texts: &[OcrText],
        ui_elements: &[UiElement],
    ) -> Vec<MergedElement> {
        let mut merged = Vec::new();
        let mut id = 0;

        // 先添加可交互的 UI 元素
        for (idx, element) in ui_elements.iter().enumerate() {
            // 只选择可交互或有意义的元素
            if !element.clickable && !element.scrollable && !element.checkable {
                continue;
            }

            let description = Self::describe_ui_element(element);
            let center = element.bounds.center();

            // 记录到字典
            self.element_map.insert(
                id,
                ElementInfo {
                    element_type: ElementType::Xml,
                    center,
                    bounds: Some((
                        element.bounds.x1,
                        element.bounds.y1,
                        element.bounds.x2,
                        element.bounds.y2,
                    )),
                },
            );

            merged.push(MergedElement {
                id,
                element_type: ElementType::Xml,
                description,
                original_index: idx,
            });

            id += 1;
        }

        // 再添加 OCR 文字
        for (idx, ocr) in ocr_texts.iter().enumerate() {
            // 过滤置信度过低的文字
            if ocr.confidence < 0.5 {
                continue;
            }

            let description = format!("文字: \"{}\"", ocr.text);

            // 记录到字典
            self.element_map.insert(
                id,
                ElementInfo {
                    element_type: ElementType::Ocr,
                    center: ocr.center,
                    bounds: None,
                },
            );

            merged.push(MergedElement {
                id,
                element_type: ElementType::Ocr,
                description,
                original_index: idx,
            });

            id += 1;
        }

        merged
    }

    /// 描述 UI 元素 (给 AI 看的可读文本)
    fn describe_ui_element(element: &UiElement) -> String {
        let mut parts = Vec::new();

        // 类型
        let simple_class = element.class_name
            .split('.')
            .last()
            .unwrap_or(&element.class_name);
        parts.push(format!("类型: {}", simple_class));

        // 文本内容
        if let Some(text) = &element.text {
            if !text.is_empty() {
                parts.push(format!("文本: \"{}\"", text));
            }
        }

        // 内容描述
        if let Some(desc) = &element.content_desc {
            if !desc.is_empty() {
                parts.push(format!("描述: \"{}\"", desc));
            }
        }

        // 资源 ID
        if let Some(rid) = &element.resource_id {
            if !rid.is_empty() {
                let simple_rid = rid.split('/').last().unwrap_or(rid);
                parts.push(format!("ID: {}", simple_rid));
            }
        }

        // 提示
        if let Some(hint) = &element.hint {
            if !hint.is_empty() {
                parts.push(format!("提示: \"{}\"", hint));
            }
        }

        // 属性
        let mut attrs = Vec::new();
        if element.clickable {
            attrs.push("可点击");
        }
        if element.scrollable {
            attrs.push("可滚动");
        }
        if element.checkable {
            if element.checked {
                attrs.push("已勾选");
            } else {
                attrs.push("可勾选");
            }
        }
        if !attrs.is_empty() {
            parts.push(format!("属性: [{}]", attrs.join(", ")));
        }

        parts.join(", ")
    }

    /// 计算 bbox 的中心点
    fn calculate_center(bbox: &[[f32; 2]]) -> (i32, i32) {
        if bbox.is_empty() {
            return (0, 0);
        }

        let sum_x: f32 = bbox.iter().map(|p| p[0]).sum();
        let sum_y: f32 = bbox.iter().map(|p| p[1]).sum();
        let count = bbox.len() as f32;

        ((sum_x / count) as i32, (sum_y / count) as i32)
    }

    /// 根据元素 ID 查询坐标
    pub fn get_element_position(&self, element_id: usize) -> Option<(i32, i32)> {
        self.element_map.get(&element_id).map(|info| info.center)
    }

    /// 根据元素 ID 查询完整信息
    pub fn get_element_info(&self, element_id: usize) -> Option<&ElementInfo> {
        self.element_map.get(&element_id)
    }

    /// 生成给 AI 看的屏幕描述
    pub fn generate_screen_description(screen_state: &ScreenState) -> String {
        let mut description = String::new();

        description.push_str("# 当前屏幕可交互元素列表\n\n");
        description.push_str("(每个元素都有一个 ID, 你可以通过 ID 来选择要操作的元素)\n\n");

        for element in &screen_state.merged_elements {
            description.push_str(&format!(
                "[{}] {}\n",
                element.id,
                element.description
            ));
        }

        description
    }
}

impl Default for WorkerParser {
    fn default() -> Self {
        Self::new()
    }
}
