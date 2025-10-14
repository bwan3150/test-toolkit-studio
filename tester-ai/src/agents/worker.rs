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
            r#"你是一个移动应用自动化测试执行员。

# 测试任务
{}

# 参考知识
{}

# 当前屏幕状态
{}

# 已执行的操作 (共 {} 步)
{}

请根据以上信息，决定下一步操作。

可用的操作类型:
- click: 点击元素 (需要 target_element_id)
- swipe: 滑动 (需要 target_element_id 作为起点, params.to 作为终点坐标)
- input: 输入文字 (需要 target_element_id, params.text)
- back: 返回 (不需要 target_element_id)
- wait: 等待 (不需要 target_element_id, params.duration 默认 1000ms)

请严格以 JSON 格式返回决策，格式如下：
{{
  "action_type": "click",
  "target_element_id": 5,
  "params": {{
    "text": "输入的文本",
    "duration": 1000,
    "to": [500, 800]
  }},
  "reasoning": "决策理由 - 解释为什么选择这个操作",
  "test_completed": false
}}

注意:
1. target_element_id 必须是屏幕元素列表中的 ID
2. 如果你认为测试已经完成（达到了测试目标），请将 test_completed 设置为 true
3. 只返回 JSON，不要有任何其他文字
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
