// Supervisor Agent - 审核测试结果

use anyhow::{Context, Result};
use rig::completion::Prompt;
use rig::providers::openai;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Supervisor 的审核结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SupervisorReview {
    /// 测试未完成，需要继续
    Incomplete {
        /// 建议
        feedback: String,
    },

    /// 测试完成，功能正常
    PassedNormal {
        /// 总结
        summary: String,
    },

    /// 测试完成，发现 Bug
    FailedWithBug {
        /// Bug 描述
        bug_description: String,
        /// 总结
        summary: String,
    },
}

/// Supervisor Agent
pub struct SupervisorAgent {
    /// 测试目标
    test_objective: String,

    /// OpenAI 客户端
    client: openai::Client,

    /// 模型名称
    model: String,
}

impl SupervisorAgent {
    /// 创建新的 Supervisor Agent
    pub fn new(test_objective: String, api_key: String, model: String) -> Result<Self> {
        let client = openai::Client::new(&api_key);
        Ok(Self {
            test_objective,
            client,
            model,
        })
    }

    /// 审核测试结果
    pub async fn review_test(
        &self,
        recent_rounds: &[String],
        worker_completion_claim: &str,
    ) -> Result<SupervisorReview> {
        info!("Supervisor 审核测试结果");

        let prompt = format!(
            r#"你是一个测试审核员。

# 测试目标
{}

# Worker 最近的测试执行记录
{}

# Worker 声明
{}

请审核以上测试执行，判断：
1. 测试是否真的完成了？
2. 如果完成了，应用功能是否正常？是否发现 Bug？
3. 如果没完成，应该如何继续？

请严格以 JSON 格式返回审核结果，格式如下：

如果测试未完成:
{{
  "status": "incomplete",
  "feedback": "建议继续...（具体建议）"
}}

如果测试完成且功能正常:
{{
  "status": "passed_normal",
  "summary": "测试通过...（总结）"
}}

如果测试完成但发现 Bug:
{{
  "status": "failed_with_bug",
  "bug_description": "发现的 Bug 描述",
  "summary": "测试总结"
}}

只返回 JSON，不要有任何其他文字。
"#,
            self.test_objective,
            recent_rounds.join("\n\n"),
            worker_completion_claim
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
