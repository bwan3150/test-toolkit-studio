// Toolkit Engine 核心库
// tke = 所有测试工具的统一入口/协调器，四大模块：
//   passthrough ① 直通: 透传同目录任意二进制 (adb/aapt/k6/ffmpeg/...)
//   atomic      ② 原子方法: refresh / fetch / recognize / control + 驱动与识别引擎
//   workflow    ③ 工作流: run / steps / case + 脚本解析执行引擎
//   tools       ④ 自有工具: ocr / file / app / device

// 基础设施
pub mod utils;
pub mod models;

// ① 直通
pub mod passthrough;

// ② 原子方法
pub mod atomic;

// ③ 工作流
pub mod workflow;

// ④ 自有工具
pub mod tools;

// ===== 统一导出（保持 crate 根路径简洁） =====

// 基础设施
pub use utils::{JsonOutput, Workarea, TkeConfig};

// ① 直通
pub use passthrough::{ToolManager, AdbManager, AaptManager};

// ② 原子方法
pub use atomic::{
    Refresh, RefreshOptions, RefreshResult,
    Fetch, Recognize, Control, ControlAction,
    Controller, Fetcher, Recognizer,
};

// ③ 工作流
pub use workflow::{
    RunEvent, RunArtifacts, ScriptRunner, FlowRunner, FlowDef, FlowResult,
    Runner, ScriptParser, ScriptInterpreter, ActionTrace,
};

// ④ 自有工具
pub use tools::{FileManager, AppManager, DeviceManager};
pub use tools::ocr;
pub use tools::ocr::{ocr as run_ocr, OcrResult, OcrText};

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
}

pub type Result<T> = std::result::Result<T, TkeError>;
