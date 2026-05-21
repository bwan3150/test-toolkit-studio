// 执行结果和应用信息相关数据结构

use serde::{Deserialize, Serialize};

/// 脚本执行结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub case_id: String,
    pub script_name: String,
    pub start_time: String,
    pub end_time: String,
    pub steps: Vec<StepResult>,
    pub error: Option<String>,
}

/// 单步执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub index: usize,
    pub command: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// 包名
    pub package_name: String,
    /// 版本名称 (如 8.8.76.667)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    /// 版本号 (如 105910845)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i64>,
    /// APK 路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apk_path: Option<String>,
    /// 启动 Activity (用于启动应用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_activity: Option<String>,
}

/// 当前聚焦的应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFocus {
    /// 包名
    pub package_name: String,
    /// Activity 名称
    pub activity_name: String,
    /// 完整的窗口信息 (原始输出)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_info: Option<String>,
}
