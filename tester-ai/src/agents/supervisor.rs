// Supervisor Agent - 负责审核测试结果

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

pub struct Supervisor {
    rig_agent: RigAgent<ResponsesCompletionModel>,
}

impl Supervisor {
    pub fn new(api_key: &str) -> Result<Self> {
        let client = openai::Client::new(api_key);
        let model = client.completion_model("gpt-4-turbo");

        let agent = AgentBuilder::new(model)
            .preamble(SUPERVISOR_PROMPT)
            .build();

        Ok(Self {
            rig_agent: agent,
        })
    }

    /// 审核测试执行结果
    pub async fn review_test_execution(
        &self,
        test_case: &TestCase,
        action_history: &[TestAction],
        final_state: &str,
    ) -> Result<SupervisorReview> {
        info!("Supervisor 开始审核测试执行: {}", test_case.name);

        let prompt = format!(
            r#"测试用例: {}
描述: {}

执行的操作序列:
{}

最终屏幕状态:
{}

请审核这个测试执行：
1. 测试是否已完成？(true/false)
2. 如果完成，测试是否通过？(true/false)
3. 提供反馈意见

返回JSON格式：
{{
    "completed": true/false,
    "passed": true/false,
    "feedback": "详细反馈"
}}"#,
            test_case.name,
            test_case.description,
            Self::format_action_history(action_history),
            final_state
        );

        let response = self.rig_agent
            .prompt(&prompt)
            .await
            .map_err(|e| AiTesterError::AgentError(format!("Supervisor prompt 错误: {}", e)))?;

        let review = self.parse_review(&response)?;

        debug!("Supervisor 审核完成: {:?}", review);
        Ok(review)
    }

    /// 格式化操作历史
    fn format_action_history(actions: &[TestAction]) -> String {
        actions
            .iter()
            .enumerate()
            .map(|(i, action)| format!("{}. {:?}", i + 1, action))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 解析审核结果
    fn parse_review(&self, response: &str) -> Result<SupervisorReview> {
        let json: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| AiTesterError::ParseError(format!("解析Supervisor响应失败: {}", e)))?;

        Ok(SupervisorReview {
            completed: json.get("completed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            passed: json.get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            feedback: json.get("feedback")
                .and_then(|v| v.as_str())
                .unwrap_or("无反馈")
                .to_string(),
        })
    }
}

#[async_trait]
impl Agent for Supervisor {
    fn name(&self) -> &str {
        "Supervisor"
    }

    fn description(&self) -> &str {
        "负责审核测试执行结果"
    }

    async fn process(&self, message: AgentMessage) -> Result<AgentMessage> {
        match message {
            AgentMessage::SupervisorReview { .. } => Ok(message),
            _ => Err(AiTesterError::AgentError(
                format!("{} 不处理此类消息", self.name())
            )),
        }
    }
}

// SupervisorReview 现在在 models 模块中定义
use crate::models::SupervisorReview;

const SUPERVISOR_PROMPT: &str = r#"
你是一个严格的测试审核专家，负责审核自动化测试的执行结果。

审核标准：
1. 测试是否覆盖了测试用例的所有要求
2. 操作序列是否合理有效
3. 是否达到了预期的测试目标
4. 是否发现了任何异常或问题

判断原则：
- completed: 测试流程是否执行完整
- passed: 应用功能是否正常工作
- feedback: 提供具体的改进建议或问题说明

注意：
1. 严格按照测试用例的要求进行判断
2. 如果测试未完成，明确指出缺少的步骤
3. 如果发现bug，详细描述问题现象
4. 提供可执行的改进建议
"#;