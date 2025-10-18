// Receptionist Agent - 接收测试用例并生成测试意见

use anyhow::{Context, Result};
use rig::completion::Prompt;
use rig::providers::openai;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Receptionist 的分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceptionistAnalysis {
    /// 测试目标摘要
    pub test_objective: String,

    /// 测试步骤建议 (大致方向, 不需要过于详细)
    pub suggested_approach: Vec<String>,

    /// 需要关注的要点
    pub key_points: Vec<String>,

    /// 预期结果
    pub expected_outcome: String,
}

/// Receptionist Agent
pub struct ReceptionistAgent {
    /// OpenAI 客户端
    client: openai::Client,
    /// 模型名称
    model: String,
}

impl ReceptionistAgent {
    /// 创建新的 Receptionist Agent
    pub fn new(api_key: String, model: String) -> Result<Self> {
        let client = openai::Client::new(&api_key);
        Ok(Self { client, model })
    }

    /// 分析测试用例
    pub async fn analyze_test_case(
        &self,
        test_case_name: &str,
        test_case_description: &str,
        app_package: &str,
    ) -> Result<ReceptionistAnalysis> {
        info!("Receptionist 分析测试用例: {}", test_case_name);

        let prompt = format!(
            r#"你是一个专业的移动应用测试员接待员。

请分析以下测试用例，给出测试意见：

测试用例名称: {}
测试用例描述: {}
被测应用包名: {}

请提供：
1. 测试目标摘要 (一句话概括)
2. 大致的测试步骤建议 (3-5个大方向，不需要过于详细)
3. 需要关注的要点 (可能的边界情况、特殊场景等)
4. 预期结果

请严格以 JSON 格式返回，格式如下：
{{
  "test_objective": "测试目标",
  "suggested_approach": ["步骤1", "步骤2", "步骤3"],
  "key_points": ["要点1", "要点2"],
  "expected_outcome": "预期结果"
}}

只返回 JSON，不要有任何其他文字。
"#,
            test_case_name, test_case_description, app_package
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
