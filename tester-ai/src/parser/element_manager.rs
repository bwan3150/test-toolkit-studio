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
        let simple_class = Self::simplify_class_name(class_name);

        // 策略 1: 如果有 text，使用 text + 类型后缀
        if let Some(txt) = text {
            let clean_text = txt
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || Self::is_chinese_char(*c))
                .take(10)  // 限制长度
                .collect::<String>();

            if !clean_text.is_empty() {
                let suffix = Self::get_type_suffix(&simple_class);
                let name = if suffix.is_empty() {
                    clean_text
                } else {
                    format!("{}{}", clean_text, suffix)
                };
                return self.make_unique_name(&name);
            }
        }

        // 策略 2: 如果有 resourceId，提取有意义的部分 + 类型后缀
        if let Some(rid) = resource_id {
            if let Some(simple_id) = rid.split('/').last() {
                if !simple_id.is_empty() {
                    // 提取有意义的词（如 email_input -> 邮箱输入框）
                    let meaningful_name = Self::extract_meaningful_name(simple_id, &simple_class);
                    return self.make_unique_name(&meaningful_name);
                }
            }
        }

        // 策略 3: 纯类型名称
        self.make_unique_name(&simple_class)
    }

    /// 根据类型获取中文后缀
    fn get_type_suffix(class_name: &str) -> &'static str {
        match class_name {
            "Button" => "按钮",
            "EditText" => "输入框",
            "TextView" => "文本",
            "ImageView" => "图标",
            "CheckBox" => "复选框",
            "RadioButton" => "单选按钮",
            "Switch" => "开关",
            "SeekBar" => "滑块",
            "ProgressBar" => "进度条",
            "Spinner" => "下拉框",
            _ => "",
        }
    }

    /// 从 resourceId 提取有意义的名称
    fn extract_meaningful_name(resource_id: &str, class_name: &str) -> String {
        // 常见的模式转换
        let meaningful = resource_id
            .replace("_btn", "")
            .replace("_button", "")
            .replace("_txt", "")
            .replace("_text", "")
            .replace("_et", "")
            .replace("_edit", "")
            .replace("_iv", "")
            .replace("_img", "")
            .replace("_image", "")
            .replace("_", "");

        // 常见单词的中文映射
        let translated = match meaningful.to_lowercase().as_str() {
            "email" | "mail" => "邮箱",
            "password" | "pwd" | "pass" => "密码",
            "username" | "user" | "name" => "用户名",
            "login" | "signin" => "登录",
            "signup" | "register" => "注册",
            "submit" | "confirm" => "确认",
            "cancel" => "取消",
            "back" => "返回",
            "next" => "下一步",
            "finish" | "done" => "完成",
            "search" => "搜索",
            "phone" | "mobile" => "手机号",
            "code" | "verify" | "captcha" => "验证码",
            "close" | "dismiss" => "关闭",
            "menu" => "菜单",
            "settings" => "设置",
            "home" => "首页",
            "profile" => "个人",
            "message" | "msg" => "消息",
            _ => &meaningful,
        };

        // 添加类型后缀
        let suffix = Self::get_type_suffix(class_name);
        if suffix.is_empty() || translated.contains("按钮") || translated.contains("输入框") {
            translated.to_string()
        } else {
            format!("{}{}", translated, suffix)
        }
    }

    /// 生成 OCR 元素名称
    fn generate_ocr_element_name(&mut self, text: &str) -> String {
        // 先清理文本，保留字母数字和中文
        let clean_text = text
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || Self::is_chinese_char(*c))
            .take(10)
            .collect::<String>();

        if clean_text.is_empty() {
            return self.make_unique_name("OCR文本");
        }

        // 尝试识别常见的 UI 文本并添加后缀
        let name_with_suffix = match clean_text.to_lowercase().as_str() {
            txt if txt.contains("login") || txt.contains("signin") || txt == "登录" => format!("{}按钮", clean_text),
            txt if txt.contains("sign") && txt.contains("up") || txt == "注册" => format!("{}按钮", clean_text),
            txt if txt.contains("continue") || txt == "继续" => format!("{}按钮", clean_text),
            txt if txt.contains("submit") || txt == "提交" => format!("{}按钮", clean_text),
            txt if txt.contains("confirm") || txt == "确认" => format!("{}按钮", clean_text),
            txt if txt.contains("cancel") || txt == "取消" => format!("{}按钮", clean_text),
            txt if txt.contains("back") || txt == "返回" => format!("{}按钮", clean_text),
            txt if txt.contains("next") || txt == "下一步" => format!("{}按钮", clean_text),
            txt if txt.contains("welcome") || txt.starts_with("欢迎") => format!("{}文本", clean_text),
            txt if txt.contains("password") || txt == "密码" => format!("{}文本", clean_text),
            _ => clean_text.clone(),
        };

        self.make_unique_name(&name_with_suffix)
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
