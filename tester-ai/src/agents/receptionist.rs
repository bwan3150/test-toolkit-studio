// Receptionist Agent - 接收测试用例并生成测试意见

use anyhow::Result;
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
    /// LLM 客户端 (暂时占位, 后续使用 RIG 实现)
    _placeholder: (),
}

impl ReceptionistAgent {
    /// 创建新的 Receptionist Agent
    pub fn new() -> Self {
        Self {
            _placeholder: (),
        }
    }

    /// 分析测试用例
    pub async fn analyze_test_case(
        &self,
        test_case_name: &str,
        test_case_description: &str,
        app_package: &str,
    ) -> Result<ReceptionistAnalysis> {
        info!("Receptionist 分析测试用例: {}", test_case_name);

        // TODO: 使用 RIG 框架调用 LLM
        // 这里先返回一个示例结果

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

以 JSON 格式返回，格式如下：
{{
  "test_objective": "测试目标",
  "suggested_approach": ["步骤1", "步骤2", "步骤3"],
  "key_points": ["要点1", "要点2"],
  "expected_outcome": "预期结果"
}}
"#,
            test_case_name, test_case_description, app_package
        );

        // 暂时返回模拟数据
        Ok(ReceptionistAnalysis {
            test_objective: format!("测试 {} 应用的 {} 功能", app_package, test_case_name),
            suggested_approach: vec![
                "启动应用".to_string(),
                "导航到目标功能".to_string(),
                "执行测试操作".to_string(),
                "验证结果".to_string(),
            ],
            key_points: vec![
                "注意边界情况".to_string(),
                "检查错误处理".to_string(),
            ],
            expected_outcome: "功能正常工作，无错误".to_string(),
        })
    }
}

impl Default for ReceptionistAgent {
    fn default() -> Self {
        Self::new()
    }
}
