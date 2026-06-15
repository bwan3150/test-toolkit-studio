// Toolkit Engine 核心库
// tke = 所有测试工具的统一入口/协调器。按职责分层（自底向上）：
//   models / utils        数据模型 / 基础设施
//   passthrough           ① 直通: 透传同目录任意二进制 + 二进制定位 (adb/aapt/k6/...)
//   drivers               设备/协议对接层 (adb/wda/web) + Controller 分发
//   engines               纯逻辑引擎: fetcher(解析) / recognizer(识别) / ocr
//   atomic                ② 原子方法: refresh / fetch / recognize / control（编排 drivers+engines）
//   workflow              ③ 工作流: run / steps / case + 脚本解析执行引擎
//   tools                 ④ 自有工具: file / app / device / element

// 基础设施
pub mod utils;
pub mod models;

// ① 直通 + 二进制定位
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

// ===== 统一导出（保持 crate 根路径简洁） =====

// 基础设施
pub use utils::{JsonOutput, Workarea, TkeConfig, AiConfig, KnowledgeConfig};

// ① 直通 + 二进制定位
pub use passthrough::ToolManager;

// 设备驱动分发
pub use drivers::Controller;

// 纯逻辑引擎
pub use engines::{Fetcher, Recognizer};
pub use engines::ocr;
pub use engines::ocr::{ocr as run_ocr, OcrResult, OcrText};

// ② 原子方法
pub use atomic::{
    Refresh, RefreshOptions, RefreshResult,
    Fetch, Recognize, Control, ControlAction,
};

// ③ 工作流
pub use workflow::{
    RunEvent, RunArtifacts, ScriptRunner, FlowRunner, FlowDef, FlowResult,
    Runner, ScriptParser, ScriptInterpreter, ActionTrace,
};
// ③ 工作流 - AI 探索（tke case）
pub use workflow::agent::{
    LlmSession, LlmTool, LlmToolCall, LlmReply,
    AgentRunner, AgentRunOptions, AgentResult,
    PromptSet, PromptSpec,
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
}

pub type Result<T> = std::result::Result<T, TkeError>;
