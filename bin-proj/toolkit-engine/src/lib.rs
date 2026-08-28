// Toolkit Engine 核心库
// tke = 所有测试工具的统一入口/协调器。按职责分层（自底向上）：
//   models / utils        数据模型 / 基础设施
//   passthrough           二进制定位 (adb/chromedriver/go-ios/tke-opencv)——CLI 直通已删，见 ADR-0015
//   drivers               设备/协议对接层 (adb/wda/web) + Controller 分发
//   engines               纯逻辑引擎: fetcher(解析) / recognizer(识别) / ocr
//   atomic                ② 原子方法: refresh / fetch / recognize / control（编排 drivers+engines）
//   workflow              ③ 工作流: run / steps / harness + 脚本解析执行引擎
//   tools                 ④ 自有工具: file / app / device / element

// 基础设施
pub mod utils;
pub mod models;

// 二进制定位
pub mod passthrough;

// 设备/协议对接层
pub mod drivers;

// 纯逻辑引擎
pub mod engines;

// ② 原子方法
pub mod atomic;

// ③ 工作流
pub mod workflow;

// ④ 自有工具
pub mod tools;

// ⑤ 服务化：把本机能力开成 HTTP 接口（ADR-0022）
pub mod serve;

// ⑤'' 源码沙盒：让 harness 拿着图纸测（ADR-0025 / INV-19）
pub mod sandbox;

// ⑤' 远程客户端：TKE_REMOTE 一设，同一条命令发给远端节点（ADR-0022 D4）
pub mod remote;

// ===== 统一导出（保持 crate 根路径简洁） =====

// 基础设施
pub use utils::{JsonOutput, Workarea, TkeConfig, AiConfig, KnowledgeConfig, Params};
pub use utils::resolve_in_workspace;

// 二进制定位
pub use passthrough::ToolManager;

// 设备驱动分发
pub use drivers::{format_tabs, Controller, TabInfo};

// 纯逻辑引擎
pub use engines::{Fetcher, Recognizer};
pub use engines::ocr;
pub use engines::ocr::{ocr as run_ocr, OcrResult, OcrText};

// ② 原子方法
pub use atomic::{
    Refresh, RefreshOptions, RefreshResult,
    Fetch, Recognize, Control, ControlAction, DialogAction,
};

// ③ 工作流
pub use workflow::{
    RunEvent, RunArtifacts, ScriptRunner, FlowRunner, FlowDef, FlowResult,
    TksRunner, ScriptParser, ScriptInterpreter, ActionTrace,
};
/// 版本 + 具体是哪一次构建，如 `0.7.4-beta (30f5cde8)`。
///
/// **版本号整个 beta 期不变**，于是"这台节点装没装上某个修复"谁也说不清 ——
/// 实测吃过亏：用户更新完节点，两头都无法判断他拿到的是哪一版，只能靠猜。
/// 并进同一个串里，平台那边零改动就看得见（不用加列、不用迁移）。
pub fn version_line() -> String {
    let commit = env!("BUILD_COMMIT");
    if commit.is_empty() || commit == "unknown" {
        env!("BUILD_VERSION").to_string()
    } else {
        format!("{} ({})", env!("BUILD_VERSION"), commit)
    }
}

// ③ 工作流 - AI 探索（tke harness）
pub use workflow::agent::{
    LlmSession, LlmTool, LlmToolCall, LlmReply,
    AgentRunner, AgentRunOptions, AgentResult,
    PromptSet, PromptSpec,
    Frontend, JsonFrontend, Level, PlainFrontend, TuiFrontend, UiCommand, UiEvent,
};

// ④ 自有工具
pub use tools::{FileManager, AppManager, DeviceManager};

// 数据模型
pub use models::{
    UIElement,
    Locator,
    LocatorStrategy,
    Platform,
    AndroidLocator,
    IosLocator,
    WebLocator,
    TksScript,
    TksStep,
    TksCommand,
    TksParam,
    DeviceInfo,
    HardwareInfo,
    BatteryInfo,
    NetworkInfo,
    Point,
    Bounds,
    ExecutionResult,
    StepResult,
    TaskLog,
    Verdict,
    AppInfo,
    CurrentFocus,
};

// 错误类型
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TkeError {
    #[error("ADB错误: {0}")]
    AdbError(String),

    #[error("AAPT错误: {0}")]
    AaptError(String),

    #[error("文件IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("XML解析错误: {0}")]
    XmlError(String),

    #[error("图像处理错误: {0}")]
    ImageError(String),

    #[error("元素未找到: {0}")]
    ElementNotFound(String),

    #[error("脚本解析错误: {0}")]
    ScriptParseError(String),

    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("脚本执行错误: {0}")]
    ScriptExecuteError(String),

    #[error("无效的参数: {0}")]
    InvalidArgument(String),

    #[error("设备未连接")]
    DeviceNotConnected,

    #[error("设备错误: {0}")]
    DeviceError(String),

    #[error("项目路径无效: {0}")]
    InvalidProjectPath(String),

    #[error("OCR错误: {0}")]
    OcrError(String),

    #[error("AI模型错误: {0}")]
    LlmError(String),

    #[error("网络错误: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, TkeError>;
