// Worker Agent - 负责执行具体的测试操作

use async_trait::async_trait;
use rig_core::{
    agent::{Agent as RigAgent, AgentBuilder},
    completion::Prompt,
    providers::openai::{self, responses_api::ResponsesCompletionModel},
    client::CompletionClient,
};
use tracing::{info, debug};

use crate::core::error::{Result, AiTesterError};
use crate::models::*;
use super::Agent;

pub struct Worker {
    rig_agent: RigAgent<ResponsesCompletionModel>,
    test_context: TestContext,
}

#[derive(Clone)]
struct TestContext {
    test_case: TestCase,
    analysis: String,
    knowledge: Vec<String>,
    action_history: Vec<TestAction>,
}

impl Worker {
    pub fn new(
        api_key: &str,
        test_case: TestCase,
        analysis: String,
        knowledge: Vec<String>,
    ) -> Result<Self> {
        let client = openai::Client::new(api_key);
        let model = client.completion_model("gpt-4-turbo");

        // 构建包含测试上下文的提示词
        let preamble = format!(
            "{}\n\n测试用例: {}\n描述: {}\n\n分析结果:\n{}\n\n相关知识:\n{}",
            WORKER_BASE_PROMPT,
            test_case.name,
            test_case.description,
            analysis,
            knowledge.join("\n---\n")
        );

        let agent = AgentBuilder::new(model)
            .preamble(&preamble)
            .build();

        Ok(Self {
            rig_agent: agent,
            test_context: TestContext {
                test_case,
                analysis,
                knowledge,
                action_history: Vec::new(),
            },
        })
    }

    /// 根据当前屏幕状态决定下一步操作
    pub async fn decide_action(
        &mut self,
        ui_elements: Vec<UIElement>,
        ocr_results: Vec<OcrResult>,
    ) -> Result<(TestAction, String)> {
        info!("Worker 开始决策下一步操作");

        // 构建当前状态描述
        let state_description = self.build_state_description(&ui_elements, &ocr_results);

        // 构建历史记录
        let history = self.build_action_history();

        let prompt = format!(
            "当前屏幕状态:\n{}\n\n已执行操作:\n{}\n\n请决定下一步操作。返回JSON格式：\n{{\n  \"action\": {{}},\n  \"reasoning\": \"操作理由\",\n  \"completed\": false\n}}",
            state_description,
            history
        );

        let response = self.rig_agent
            .prompt(&prompt)
            .await
            .map_err(|e| AiTesterError::AgentError(format!("Worker prompt 错误: {}", e)))?;

        // 解析响应
        let decision = self.parse_action_decision(&response)?;

        // 记录操作历史
        self.test_context.action_history.push(decision.0.clone());

        debug!("Worker 决策完成: {:?}", decision);
        Ok(decision)
    }

    /// 判断测试是否完成
    pub async fn is_test_complete(&self) -> Result<bool> {
        if self.test_context.action_history.len() < 2 {
            return Ok(false);
        }

        let prompt = format!(
            "基于以下测试执行历史，判断测试是否已完成。\n\n测试目标: {}\n\n执行历史:\n{}\n\n仅回答 'true' 或 'false'",
            self.test_context.test_case.description,
            self.build_action_history()
        );

        let response = self.rig_agent
            .prompt(&prompt)
            .await
            .map_err(|e| AiTesterError::AgentError(format!("Worker prompt 错误: {}", e)))?;

        Ok(response.trim().to_lowercase() == "true")
    }

    /// 构建屏幕状态描述
    fn build_state_description(
        &self,
        ui_elements: &[UIElement],
        ocr_results: &[OcrResult],
    ) -> String {
        let mut description = String::new();

        // UI元素描述
        description.push_str("UI元素:\n");
        for (i, elem) in ui_elements.iter().enumerate().take(20) {
            description.push_str(&format!(
                "  [{}] {} - ",
                i, elem.class
            ));

            if let Some(text) = &elem.text {
                description.push_str(&format!("文本: '{}' ", text));
            }
            if let Some(id) = &elem.resource_id {
                description.push_str(&format!("ID: {} ", id));
            }
            if elem.clickable {
                description.push_str("(可点击) ");
            }
            if elem.scrollable {
                description.push_str("(可滚动) ");
            }

            description.push_str(&format!(
                "位置: [{}, {}, {}, {}]\n",
                elem.bounds[0], elem.bounds[1], elem.bounds[2], elem.bounds[3]
            ));
        }

        // OCR结果描述
        if !ocr_results.is_empty() {
            description.push_str("\nOCR识别文字:\n");
            for result in ocr_results.iter().take(10) {
                description.push_str(&format!(
                    "  '{}' - 位置: [{}, {}, {}, {}]\n",
                    result.text,
                    result.bounds[0], result.bounds[1], result.bounds[2], result.bounds[3]
                ));
            }
        }

        description
    }

    /// 构建操作历史描述
    fn build_action_history(&self) -> String {
        if self.test_context.action_history.is_empty() {
            return "无".to_string();
        }

        self.test_context.action_history
            .iter()
            .enumerate()
            .map(|(i, action)| format!("{}. {:?}", i + 1, action))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 解析AI返回的操作决策
    fn parse_action_decision(&self, response: &str) -> Result<(TestAction, String)> {
        // 尝试解析JSON响应
        let json: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| AiTesterError::ParseError(format!("解析Worker响应失败: {}", e)))?;

        let action_json = json.get("action")
            .ok_or_else(|| AiTesterError::ParseError("响应中缺少action字段".to_string()))?;

        let reasoning = json.get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("无")
            .to_string();

        // 解析具体的操作类型
        let action = if let Some(tap) = action_json.get("tap") {
            TestAction::Tap {
                x: tap.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: tap.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            }
        } else if let Some(swipe) = action_json.get("swipe") {
            TestAction::Swipe {
                x1: swipe.get("x1").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y1: swipe.get("y1").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                x2: swipe.get("x2").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y2: swipe.get("y2").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                duration: swipe.get("duration").and_then(|v| v.as_u64()).unwrap_or(300) as u32,
            }
        } else if let Some(input) = action_json.get("input") {
            TestAction::Input {
                text: input.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }
        } else if action_json.get("back").is_some() {
            TestAction::Back
        } else if action_json.get("home").is_some() {
            TestAction::Home
        } else if let Some(wait) = action_json.get("wait") {
            TestAction::Wait {
                seconds: wait.get("seconds").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
            }
        } else {
            TestAction::Screenshot
        };

        Ok((action, reasoning))
    }
}

#[async_trait]
impl Agent for Worker {
    fn name(&self) -> &str {
        "Worker"
    }

    fn description(&self) -> &str {
        "负责执行具体的测试操作"
    }

    async fn process(&self, message: AgentMessage) -> Result<AgentMessage> {
        Err(AiTesterError::AgentError(
            "Worker 通过专用方法处理，不使用通用process接口".to_string()
        ))
    }
}

const WORKER_BASE_PROMPT: &str = r#"
你是一个专业的移动应用自动化测试执行者。

你的任务：
1. 根据当前屏幕状态，决定下一步操作
2. 每次只执行一个操作
3. 逐步完成测试目标

可用操作类型：
- tap: 点击指定坐标 {"tap": {"x": 100, "y": 200}}
- swipe: 滑动 {"swipe": {"x1": 100, "y1": 200, "x2": 100, "y2": 500, "duration": 300}}
- input: 输入文本 {"input": {"text": "hello"}}
- back: 返回键 {"back": {}}
- home: 主页键 {"home": {}}
- wait: 等待 {"wait": {"seconds": 2}}
- screenshot: 截图 {"screenshot": {}}

注意事项：
1. 优先使用UI元素的坐标，其次使用OCR识别的文字位置
2. 点击坐标应该在元素的中心位置
3. 每个操作都要有明确的目的
4. 避免重复无效的操作
5. 如果遇到异常情况，尝试使用back键返回
"#;