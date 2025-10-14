// Worker Agent - 执行测试操作的核心 Agent

use crate::models::{ActionDecision, ActionParams, ActionType};
use anyhow::Result;
use tracing::info;

/// Worker Agent
pub struct WorkerAgent {
    /// 测试指令 (由 Receptionist 提供)
    test_instruction: String,

    /// 知识库内容 (由 Librarian 提供)
    knowledge: String,

    /// LLM 占位
    _placeholder: (),
}

impl WorkerAgent {
    /// 创建新的 Worker Agent
    pub fn new(test_instruction: String, knowledge: String) -> Self {
        Self {
            test_instruction,
            knowledge,
            _placeholder: (),
        }
    }

    /// 根据当前屏幕状态做出决策
    pub async fn make_decision(
        &self,
        screen_description: &str,
        round: u32,
        previous_actions: &[String],
    ) -> Result<ActionDecision> {
        info!("Worker 做出第 {} 轮决策", round);

        // TODO: 使用 RIG 框架调用 LLM
        // 输入:
        // 1. 测试指令 (test_instruction)
        // 2. 知识库内容 (knowledge)
        // 3. 当前屏幕描述 (screen_description)
        // 4. 之前的操作历史 (previous_actions)
        //
        // 输出: ActionDecision (JSON 格式)
        //
        // Prompt 应该包含:
        // - 测试目标
        // - 当前可交互元素列表
        // - 可用的操作类型
        // - 要求返回 JSON 格式的决策

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
- click: 点击元素
- swipe: 滑动
- input: 输入文字
- back: 返回
- wait: 等待

请以 JSON 格式返回决策，格式如下：
{{
  "action_type": "click",
  "target_element_id": 5,
  "params": {{}},
  "reasoning": "决策理由",
  "test_completed": false
}}

如果你认为测试已经完成，请将 test_completed 设置为 true。
"#,
            self.test_instruction,
            self.knowledge,
            screen_description,
            previous_actions.len(),
            previous_actions.join("\n")
        );

        // 暂时返回模拟决策
        Ok(ActionDecision {
            action_type: ActionType::Wait,
            target_element_id: None,
            params: ActionParams {
                duration: Some(1000),
                ..Default::default()
            },
            reasoning: "等待屏幕加载".to_string(),
            test_completed: false,
        })
    }
}
