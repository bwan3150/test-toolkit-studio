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
    /// 脚本文件路径（工作流运行时记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    /// 本次运行的产物目录（工作流运行时记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_dir: Option<String>,
    /// 本次运行中启动过的 Android 包名（flow 收尾统一关闭用）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launched_packages: Vec<String>,
}

/// 单步执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub index: usize,
    pub command: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    /// 脚本中的行号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// 本步标注后截图路径（相对 run_dir）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// 本步 UI 结构文件路径（相对 run_dir）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml: Option<String>,
    /// AI 辅助驾驶：本步元素按原定位找不到、由 AI 依当前页面找回才通过（值 = 元素名）。
    /// 只影响本次执行，不改 .tks / .tklib——报告用，提示脚本定位可能需要更新。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub healed: Option<String>,
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
