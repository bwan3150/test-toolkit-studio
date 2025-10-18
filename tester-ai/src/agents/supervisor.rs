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
        current_screen: &str,
        all_rounds: &[String],
        worker_completion_claim: &str,
    ) -> Result<SupervisorReview> {
        info!("Supervisor 审核测试结果");

        let prompt = format!(
            r#"你是一个测试审核员，负责审核 Worker 的测试执行结果。

# 测试目标
{}

# 当前屏幕状态（Worker 声称完成时的屏幕）
{}

# 完整的测试执行历史（从第一轮到最后一轮）
{}

# Worker 的完成声明
{}

## 你的审核任务：
1. **检查操作正确性**：仔细审查每一轮的操作，看 Worker 是否有误操作
   - 是否把某个信息输入到了其他错误的输入框？
   - 是否忘记输入某个必填字段？
   - 是否点击了错误的按钮？
   - 其他此类问题

2. **检查当前屏幕状态**：根据测试目标，判断当前屏幕是否符合预期
   - 如果测试目标是登录成功，当前是否在主界面？
   - 当前屏幕是否有错误提示（如 "Invalid email", "Wrong password" 等）？

3. **判断测试完成度**：
   - 测试目标是否真的达成了？
   - Worker 的操作步骤是否完整？

4. **发现潜在 Bug**：
   - 应用是否有异常行为？
   - 是否有功能不正常？

## 返回格式（严格 JSON）：

如果测试未完成（Worker 还有步骤没做，或者操作有误需要重试）:
{{
  "status": "incomplete",
  "feedback": "具体的问题和建议，例如：Worker 在第3轮将密码输入到了邮箱框，需要重新正确输入"
}}

如果测试完成且功能正常:
{{
  "status": "passed_normal",
  "summary": "测试通过的总结，说明达成了什么目标"
}}

如果发现应用 Bug（不是 Worker 的操作问题，而是应用本身的问题）:
{{
  "status": "failed_with_bug",
  "bug_description": "Bug 的详细描述，例如：输入了正确的邮箱密码，但应用提示'Invalid email address'",
  "summary": "测试总结"
}}

只返回 JSON，不要有任何其他文字。
"#,
            self.test_objective,
            current_screen,
            all_rounds.join("\n\n"),
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
