// OCR 和 XML 处理模块

use std::path::Path;
use image::DynamicImage;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, debug, error};

use crate::core::error::{Result, AiTesterError};
use crate::models::{OcrResult, UIElement};

/// OCR 服务接口
#[async_trait::async_trait]
pub trait OcrService: Send + Sync {
    async fn extract_text(&self, image_path: &Path) -> Result<Vec<OcrResult>>;
}

/// OpenAI Vision OCR 实现
pub struct OpenAiVisionOcr {
    client: Client,
    api_key: String,
}

impl OpenAiVisionOcr {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// 将图片转为base64
    fn image_to_base64(&self, image_path: &Path) -> Result<String> {
        let image_data = std::fs::read(image_path)
            .map_err(|e| AiTesterError::FileSystemError(format!("读取图片失败: {}", e)))?;

        Ok(STANDARD.encode(&image_data))
    }
}

#[async_trait::async_trait]
impl OcrService for OpenAiVisionOcr {
    async fn extract_text(&self, image_path: &Path) -> Result<Vec<OcrResult>> {
        info!("使用 OpenAI Vision 进行 OCR: {:?}", image_path);

        let base64_image = self.image_to_base64(image_path)?;

        let request_body = json!({
            "model": "gpt-4-vision-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "请识别图片中的所有文字，返回JSON格式：[{\"text\": \"文字内容\", \"bounds\": [x1, y1, x2, y2]}]"
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", base64_image)
                        }
                    }
                ]
            }],
            "max_tokens": 1000
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiTesterError::NetworkError(format!("OCR请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiTesterError::OcrError(format!("OCR API错误: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiTesterError::ParseError(format!("解析OCR响应失败: {}", e)))?;

        // 解析响应中的文字
        let content = response_json
            .get("choices")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("message"))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiTesterError::ParseError("OCR响应格式错误".to_string()))?;

        // 解析JSON格式的OCR结果
        let ocr_results: Vec<OcrResultJson> = serde_json::from_str(content)
            .unwrap_or_else(|_| {
                // 如果解析失败，尝试简单处理
                vec![OcrResultJson {
                    text: content.to_string(),
                    bounds: [0, 0, 100, 50],
                }]
            });

        Ok(ocr_results.into_iter().map(|r| OcrResult {
            text: r.text,
            confidence: 0.9, // OpenAI 不提供置信度，使用默认值
            bounds: r.bounds,
        }).collect())
    }
}

#[derive(Deserialize)]
struct OcrResultJson {
    text: String,
    bounds: [i32; 4],
}

/// 本地 OCR 实现（使用 tesseract 或其他本地OCR）
pub struct LocalOcr;

impl LocalOcr {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl OcrService for LocalOcr {
    async fn extract_text(&self, image_path: &Path) -> Result<Vec<OcrResult>> {
        info!("使用本地 OCR: {:?}", image_path);

        // TODO: 实现本地OCR逻辑
        // 可以调用 tesseract 命令行工具或使用 rust 的 tesseract 绑定

        warn!("本地OCR尚未实现，返回空结果");
        Ok(Vec::new())
    }
}

/// XML 处理器
pub struct XmlProcessor;

impl XmlProcessor {
    pub fn new() -> Self {
        Self
    }

    /// 解析 UI XML 文件
    pub fn parse_ui_xml(&self, xml_path: &Path) -> Result<Vec<UIElement>> {
        info!("解析 UI XML: {:?}", xml_path);

        let xml_content = std::fs::read_to_string(xml_path)
            .map_err(|e| AiTesterError::FileSystemError(format!("读取XML失败: {}", e)))?;

        // TODO: 实现XML解析逻辑
        // 这里应该解析Android的UI Automator XML格式

        self.parse_xml_string(&xml_content)
    }

    /// 解析 XML 字符串
    fn parse_xml_string(&self, xml: &str) -> Result<Vec<UIElement>> {
        // 简单的XML解析示例
        // 实际应该使用 xml-rs 或 quick-xml 库进行解析

        let mut elements = Vec::new();

        // 这是一个简化的示例，实际需要完整的XML解析
        for (index, line) in xml.lines().enumerate() {
            if line.contains("class=") {
                // 提取属性（简化处理）
                let class = self.extract_attribute(line, "class").unwrap_or_default();
                let text = self.extract_attribute(line, "text");
                let resource_id = self.extract_attribute(line, "resource-id");
                let content_desc = self.extract_attribute(line, "content-desc");
                let bounds = self.parse_bounds_from_string(
                    &self.extract_attribute(line, "bounds").unwrap_or_default()
                );
                let clickable = self.extract_attribute(line, "clickable")
                    .map(|v| v == "true")
                    .unwrap_or(false);
                let scrollable = self.extract_attribute(line, "scrollable")
                    .map(|v| v == "true")
                    .unwrap_or(false);

                elements.push(UIElement {
                    index,
                    class,
                    text,
                    resource_id,
                    content_desc,
                    bounds,
                    clickable,
                    scrollable,
                });
            }
        }

        Ok(elements)
    }

    /// 提取XML属性值
    fn extract_attribute(&self, line: &str, attr: &str) -> Option<String> {
        let pattern = format!(r#"{}="([^"]*)""#, attr);
        let re = regex::Regex::new(&pattern).ok()?;
        re.captures(line)?.get(1).map(|m| m.as_str().to_string())
    }

    /// 解析bounds字符串 "[x1,y1][x2,y2]"
    fn parse_bounds_from_string(&self, bounds_str: &str) -> [i32; 4] {
        let re = regex::Regex::new(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]").unwrap();
        if let Some(caps) = re.captures(bounds_str) {
            return [
                caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                caps.get(4).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            ];
        }
        [0, 0, 0, 0]
    }

    /// 过滤有用的UI元素
    pub fn filter_useful_elements(&self, elements: Vec<UIElement>) -> Vec<UIElement> {
        elements.into_iter()
            .filter(|elem| {
                // 过滤掉没有实际内容的元素
                elem.text.is_some() ||
                elem.content_desc.is_some() ||
                elem.clickable ||
                elem.scrollable
            })
            .collect()
    }
}

use tracing::warn;