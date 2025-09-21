// Receptionist Agent - 负责接收和分析测试用例

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

pub struct Receptionist {
    rig_agent: RigAgent<ResponsesCompletionModel>,
}

impl Receptionist {
    pub fn new(api_key: &str) -> Result<Self> {
        let client = openai::Client::new(api_key);
        let model = client.completion_model("gpt-4-turbo");

        let agent = AgentBuilder::new(model)
            .preamble(RECEPTIONIST_PROMPT)
            .build();

        Ok(Self {
            rig_agent: agent,
        })
    }

    /// 分析测试用例
    pub async fn analyze_test_case(&self, test_case: &TestCase) -> Result<String> {
        info!("Receptionist 开始分析测试用例: {}", test_case.name);

        let prompt = format!(
            "测试用例名称: {}\n测试用例描述: {}\n\n请分析这个测试用例，给出测试策略和注意事项。",
            test_case.name, test_case.description
        );

        let response = self.rig_agent
            .prompt(&prompt)
            .await
            .map_err(|e| AiTesterError::AgentError(format!("Receptionist prompt 错误: {}", e)))?;

        debug!("Receptionist 分析完成");
        Ok(response)
    }
}

#[async_trait]
impl Agent for Receptionist {
    fn name(&self) -> &str {
        "Receptionist"
    }

    fn description(&self) -> &str {
        "负责接收和分析测试用例，制定测试策略"
    }

    async fn process(&self, message: AgentMessage) -> Result<AgentMessage> {
        match message {
            AgentMessage::TestCaseAnalysis { test_case, .. } => {
                let analysis = self.analyze_test_case(&test_case).await?;
                Ok(AgentMessage::TestCaseAnalysis {
                    test_case,
                    analysis,
                })
            }
            _ => Err(AiTesterError::AgentError(
                format!("{} 不处理此类消息", self.name())
            )),
        }
    }
}

const RECEPTIONIST_PROMPT: &str = r#"
你是一个专业的移动应用测试专家，负责分析测试用例并制定测试策略。

你的职责：
1. 理解测试用例的目标和要求
2. 分析测试的关键点和潜在风险
3. 制定合理的测试策略
4. 给出简洁但重要的注意事项

注意：
- 不要过于详细，因为具体实施由Worker负责
- 关注测试的核心目标
- 考虑可能的边界情况
- 保持策略的通用性和灵活性
"#;