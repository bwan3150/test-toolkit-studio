// Element Manager - 管理元素到 element.json 的保存和查询

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// element.json 中的元素定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementDefinition {
    #[serde(rename = "type")]
    pub element_type: String, // "xml" or "ocr"

    // XML 元素属性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub className: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[i32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resourceId: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focusable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrollable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centerX: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centerY: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matchStrategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addedAt: Option<String>,
}

/// 元素管理器
pub struct ElementManager {
    /// 项目路径
    project_path: PathBuf,
    /// element.json 文件路径
    element_file: PathBuf,
    /// 已加载的元素
    elements: HashMap<String, ElementDefinition>,
    /// 元素名称计数器 (用于生成唯一名称)
    name_counters: HashMap<String, u32>,
}

impl ElementManager {
    /// 创建新的元素管理器
    pub fn new(project_path: impl Into<PathBuf>) -> Result<Self> {
        let project_path = project_path.into();
        let element_file = project_path.join("locator/element.json");

        let mut manager = Self {
            project_path,
            element_file,
            elements: HashMap::new(),
            name_counters: HashMap::new(),
        };

        // 加载现有的 element.json
        manager.load()?;

        Ok(manager)
    }

    /// 加载 element.json
    fn load(&mut self) -> Result<()> {
        if !self.element_file.exists() {
            debug!("element.json 不存在，将创建新文件");
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.element_file)
            .context("读取 element.json 失败")?;

        if content.trim().is_empty() {
            debug!("element.json 为空");
            return Ok(());
        }

        self.elements = serde_json::from_str(&content)
            .context("解析 element.json 失败")?;

        info!("已加载 {} 个元素定义", self.elements.len());

        Ok(())
    }

    /// 保存到 element.json
    pub fn save(&self) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = self.element_file.parent() {
            std::fs::create_dir_all(parent)
                .context("创建 locator 目录失败")?;
        }

        let json = serde_json::to_string_pretty(&self.elements)
            .context("序列化 element.json 失败")?;

        std::fs::write(&self.element_file, json)
            .context("写入 element.json 失败")?;

        info!("已保存 {} 个元素到 {}", self.elements.len(), self.element_file.display());

        Ok(())
    }

    /// 添加元素 (从 XML 元素信息)
    pub fn add_element_from_xml(
        &mut self,
        element_id: u64,
        class_name: &str,
        text: Option<&str>,
        resource_id: Option<&str>,
        bounds: (i32, i32, i32, i32),
        clickable: bool,
    ) -> String {
        // 生成元素名称
        let element_name = self.generate_element_name(class_name, text, resource_id);

        // 计算中心点和宽高
        let center_x = (bounds.0 + bounds.2) / 2;
        let center_y = (bounds.1 + bounds.3) / 2;
        let width = bounds.2 - bounds.0;
        let height = bounds.3 - bounds.1;

        // 创建元素定义
        let definition = ElementDefinition {
            element_type: "xml".to_string(),
            className: Some(class_name.to_string()),
            bounds: Some([bounds.0, bounds.1, bounds.2, bounds.3]),
            text: text.map(|s| s.to_string()),
            resourceId: resource_id.map(|s| s.to_string()),
            clickable: Some(clickable),
            focusable: None,
            scrollable: None,
            enabled: None,
            selected: None,
            checkable: None,
            checked: None,
            xpath: None,
            description: Some(format!("{}(text={}, id={})",
                Self::simplify_class_name(class_name),
                text.unwrap_or(""),
                resource_id.unwrap_or("")
            )),
            centerX: Some(center_x),
            centerY: Some(center_y),
            width: Some(width),
            height: Some(height),
            matchStrategy: if resource_id.is_some() {
                Some("resourceId".to_string())
            } else if text.is_some() {
                Some("text".to_string())
            } else {
                Some("bounds".to_string())
            },
            addedAt: Some(chrono::Utc::now().to_rfc3339()),
        };

        self.elements.insert(element_name.clone(), definition);

        debug!("添加元素: {} (ID: {})", element_name, element_id);

        element_name
    }

    /// 添加元素 (从 OCR 文本信息)
    pub fn add_element_from_ocr(
        &mut self,
        element_id: u64,
        text: &str,
        bounds: (i32, i32, i32, i32),
    ) -> String {
        // 生成元素名称
        let element_name = self.generate_ocr_element_name(text);

        // 计算中心点和宽高
        let center_x = (bounds.0 + bounds.2) / 2;
        let center_y = (bounds.1 + bounds.3) / 2;
        let width = bounds.2 - bounds.0;
        let height = bounds.3 - bounds.1;

        // 创建元素定义
        let definition = ElementDefinition {
            element_type: "ocr".to_string(),
            className: None,
            bounds: Some([bounds.0, bounds.1, bounds.2, bounds.3]),
            text: Some(text.to_string()),
            resourceId: None,
            clickable: None,
            focusable: None,
            scrollable: None,
            enabled: None,
            selected: None,
            checkable: None,
            checked: None,
            xpath: None,
            description: Some(format!("OCR文本: {}", text)),
            centerX: Some(center_x),
            centerY: Some(center_y),
            width: Some(width),
            height: Some(height),
            matchStrategy: Some("ocr".to_string()),
            addedAt: Some(chrono::Utc::now().to_rfc3339()),
        };

        self.elements.insert(element_name.clone(), definition);

        debug!("添加 OCR 元素: {} (ID: {})", element_name, element_id);

        element_name
    }

    /// 查询元素名称
    pub fn get_element_name(&self, name: &str) -> Option<&ElementDefinition> {
        self.elements.get(name)
    }

    /// 生成元素名称 (基于 XML 元素信息)
    fn generate_element_name(
        &mut self,
        class_name: &str,
        text: Option<&str>,
        resource_id: Option<&str>,
    ) -> String {
        // 优先使用 resource_id
        if let Some(rid) = resource_id {
            if let Some(simple_id) = rid.split('/').last() {
                if !simple_id.is_empty() {
                    return self.make_unique_name(simple_id);
                }
            }
        }

        // 其次使用 text
        if let Some(txt) = text {
            if !txt.trim().is_empty() && txt.len() <= 20 {
                let clean_text = txt
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || Self::is_chinese_char(*c))
                    .collect::<String>();

                if !clean_text.is_empty() {
                    return self.make_unique_name(&clean_text);
                }
            }
        }

        // 最后使用类名
        let simple_class = Self::simplify_class_name(class_name);
        self.make_unique_name(&simple_class)
    }

    /// 生成 OCR 元素名称
    fn generate_ocr_element_name(&mut self, text: &str) -> String {
        let clean_text = text
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || Self::is_chinese_char(*c))
            .take(15)
            .collect::<String>();

        if clean_text.is_empty() {
            return self.make_unique_name("ocr文本");
        }

        self.make_unique_name(&clean_text)
    }

    /// 简化类名
    fn simplify_class_name(class_name: &str) -> String {
        class_name
            .split('.')
            .last()
            .unwrap_or(class_name)
            .to_string()
    }

    /// 判断是否为中文字符
    fn is_chinese_char(c: char) -> bool {
        matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{20000}'..='\u{2A6DF}')
    }

    /// 确保名称唯一 (如果重复则添加数字后缀)
    fn make_unique_name(&mut self, base_name: &str) -> String {
        if !self.elements.contains_key(base_name) {
            return base_name.to_string();
        }

        // 已存在，添加数字后缀
        let counter = self.name_counters.entry(base_name.to_string()).or_insert(1);
        loop {
            *counter += 1;
            let candidate = format!("{}_{}", base_name, counter);
            if !self.elements.contains_key(&candidate) {
                return candidate;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_class_name() {
        assert_eq!(
            ElementManager::simplify_class_name("android.widget.Button"),
            "Button"
        );
        assert_eq!(
            ElementManager::simplify_class_name("Button"),
            "Button"
        );
    }

    #[test]
    fn test_is_chinese_char() {
        assert!(ElementManager::is_chinese_char('中'));
        assert!(ElementManager::is_chinese_char('文'));
        assert!(!ElementManager::is_chinese_char('a'));
        assert!(!ElementManager::is_chinese_char('1'));
    }
}
