// Supervisor Agent - 审核测试结果

use anyhow::Result;
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

    /// LLM 占位
    _placeholder: (),
}

impl SupervisorAgent {
    /// 创建新的 Supervisor Agent
    pub fn new(test_objective: String) -> Self {
        Self {
            test_objective,
            _placeholder: (),
        }
    }

    /// 审核测试结果
    pub async fn review_test(
        &self,
        recent_rounds: &[String],
        worker_completion_claim: &str,
    ) -> Result<SupervisorReview> {
        info!("Supervisor 审核测试结果");

        // TODO: 使用 RIG 框架调用 LLM
        // 输入:
        // 1. 测试目标 (test_objective)
        // 2. Worker 最近几轮的操作和观察 (recent_rounds)
        // 3. Worker 的完成声明 (worker_completion_claim)
        //
        // 输出: SupervisorReview (JSON 格式)
        //
        // Prompt 应该包含:
        // - 测试目标
        // - Worker 的执行历史
        // - 要求判断: 测试是否完成? 如果完成，是否发现 Bug?

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

请以 JSON 格式返回审核结果，格式如下：

如果测试未完成:
{{
  "status": "incomplete",
  "feedback": "建议继续..."
}}

如果测试完成且功能正常:
{{
  "status": "passed_normal",
  "summary": "测试通过..."
}}

如果测试完成但发现 Bug:
{{
  "status": "failed_with_bug",
  "bug_description": "发现的 Bug...",
  "summary": "测试总结..."
}}
"#,
            self.test_objective,
            recent_rounds.join("\n\n"),
            worker_completion_claim
        );

        // 暂时返回模拟结果
        Ok(SupervisorReview::Incomplete {
            feedback: "继续测试".to_string(),
        })
    }
}
