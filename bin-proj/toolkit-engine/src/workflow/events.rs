// 工作流实时事件 - 以 NDJSON 逐行输出到 stdout
// 用于 CI 实时监控和 Electron App 编辑器逐行跟踪

use serde::Serialize;

/// 运行事件（每个事件序列化为一行 JSON）
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum RunEvent {
    /// 脚本开始
    RunStart {
        script: String,
        script_name: String,
        total_steps: usize,
        run_dir: String,
        start_time: String,
    },
    /// 单步开始（运行中）
    StepStart {
        index: usize,
        line: usize,
        command: String,
    },
    /// 单步结束（成功/失败 + 完整报错 + 产物路径）
    StepEnd {
        index: usize,
        line: usize,
        command: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        duration_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        screenshot: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        xml: Option<String>,
    },
    /// 脚本结束
    RunEnd {
        success: bool,
        total_steps: usize,
        successful_steps: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        run_dir: String,
        log: String,
    },
    /// Flow 开始
    FlowStart {
        flow: String,
        total_scripts: usize,
        run_dir: String,
    },
    /// Flow 中单个脚本开始
    ScriptStart {
        index: usize,
        script: String,
    },
    /// Flow 中单个脚本结束
    ScriptEnd {
        index: usize,
        script: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Flow 结束
    FlowEnd {
        success: bool,
        total_scripts: usize,
        successful_scripts: usize,
        run_dir: String,
        log: String,
    },
}
