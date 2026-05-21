// Worker Agent - 执行测试操作的核心 Agent

use crate::models::{ActionDecision, ActionParams, ActionType};
use anyhow::{Context, Result};
use rig::completion::Prompt;
use rig::providers::openai;
use tracing::info;

/// Worker Agent
pub struct WorkerAgent {
    /// 测试指令 (由 Receptionist 提供)
    test_instruction: String,

    /// 知识库内容 (由 Librarian 提供)
    knowledge: String,

    /// OpenAI 客户端
    client: openai::Client,

    /// 模型名称
    model: String,
}

impl WorkerAgent {
    /// 创建新的 Worker Agent
    pub fn new(
        test_instruction: String,
        knowledge: String,
        api_key: String,
        model: String,
    ) -> Result<Self> {
        let client = openai::Client::new(&api_key);
        Ok(Self {
            test_instruction,
            knowledge,
            client,
            model,
        })
    }

    /// 根据当前屏幕状态做出决策
    pub async fn make_decision(
        &self,
        screen_description: &str,
        round: u32,
        previous_actions: &[String],
    ) -> Result<ActionDecision> {
        info!("Worker 做出第 {} 轮决策", round);

        let prompt = format!(
            r#"你是一个移动应用自动化测试执行员。你需要根据测试目标,观察当前屏幕,灵活地决定下一步操作。

# 测试任务和目标
{}

# 参考知识
{}

# 当前屏幕状态
{}

# 已执行的操作历史 (共 {} 步)
{}

## 你的任务:
1. 仔细观察当前屏幕上有哪些可交互元素
2. 结合测试目标和已执行的操作,判断现在应该做什么
3. 灵活应对界面变化,不要死板地按照固定步骤
4. 如果看到登录成功的标志(如主界面、设备列表等),就认为测试完成

## 可用的操作类型:
- click: 点击元素 (需要 target_element_id)
- swipe: 滑动 (需要 target_element_id 作为起点, params.to 作为终点坐标)
- input: 输入文字 (需要 target_element_id, params.text)
- back: 返回 (不需要 target_element_id)
- wait: 等待 (不需要 target_element_id, params.duration 默认 1000ms)
- none: 无操作,测试已完成 (不需要 target_element_id)

## 返回格式 (严格 JSON):
{{
  "action_type": "click",
  "target_element_id": 5,
  "params": {{
    "text": "输入的文本内容",
    "duration": 1000,
    "to": [500, 800]
  }},
  "reasoning": "清楚说明为什么做这个操作,以及预期达到什么效果",
  "test_completed": false
}}

## 测试完成时的返回格式:
{{
  "action_type": "none",
  "target_element_id": null,
  "params": {{}},
  "reasoning": "测试目标已达成,原因是...",
  "test_completed": true
}}

## 重要提醒:
1. target_element_id 必须是当前屏幕元素列表中存在的 ID (如果 action_type 为 none, 则设为 null)
2. 如果测试目标已达成,将 action_type 设为 "none" 并且 test_completed 设为 true
3. 只返回 JSON,不要有任何其他文字或解释
"#,
            self.test_instruction,
            self.knowledge,
            screen_description,
            previous_actions.len(),
            if previous_actions.is_empty() {
                "无".to_string()
            } else {
                previous_actions.join("\n")
            }
        );

        let agent = self.client.agent(&self.model).build();

        let response = agent
            .prompt(&prompt)
            .await
            .context("调用 LLM 失败")?;

        // 解析 JSON 响应
        let json_str = response.trim();

        // 尝试提取 JSON (如果被代码块包裹)
        let json_str = if json_str.starts_with("```json") {
            json_str
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if json_str.starts_with("```") {
            json_str
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            json_str
        };

        serde_json::from_str(json_str)
            .context(format!("解析 LLM 响应失败: {}", json_str))
    }
}
