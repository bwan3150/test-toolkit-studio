// 输出模型 - 定义返回的 JSON 结构

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// AI 测试员主输出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesterOutput {
    /// 是否成功
    pub success: bool,

    /// 测试用例 ID
    pub test_case_id: String,

    /// 测试状态
    pub status: TestStatus,

    /// 测试结果
    pub result: TestResult,

    /// 生成的 .tks 脚本路径
    pub script_path: String,

    /// 执行的总轮数
    pub total_rounds: u32,

    /// 开始时间
    pub start_time: DateTime<Utc>,

    /// 结束时间
    pub end_time: DateTime<Utc>,

    /// 错误信息 (如果有)
    pub error: Option<String>,

    /// 测试详细日志
    pub logs: Vec<RoundLog>,
}

/// 测试状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    /// 测试完成
    Completed,
    /// 测试失败
    Failed,
    /// 测试被中断
    Interrupted,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestResult {
    /// 测试通过 - App 功能正常
    Passed,
    /// 测试失败 - App 有 Bug
    FailedWithBug,
    /// 测试未完成
    Incomplete,
    /// 测试错误 - AI 或系统错误
    Error,
}

/// 单轮测试日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundLog {
    /// 轮次编号
    pub round: u32,

    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// Worker 的观察
    pub observation: String,

    /// Worker 的决策
    pub decision: String,

    /// 执行的操作
    pub action: String,

    /// 操作是否成功
    pub action_success: bool,

    /// 错误信息 (如果有)
    pub error: Option<String>,
}
