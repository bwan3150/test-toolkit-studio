// Toolkit Engine 核心库
// 提供自动化测试的核心功能

// 核心模块
pub mod utils;
pub mod models;

// 功能模块（对应 tke 命令）
pub mod adb;
pub mod aapt;
pub mod ocr;
pub mod controller;
pub mod fetcher;
pub mod recognizer;
pub mod runner;

// 导出工具类
pub use utils::{JsonOutput, AdbManager, AaptManager};

// 导出功能模块
pub use controller::Controller;
pub use fetcher::Fetcher;
pub use recognizer::Recognizer;
pub use runner::{Runner, ScriptParser, ScriptInterpreter};

// 导出 OCR 功能
pub use ocr::{ocr, OcrResult, OcrText};

// 导出模型
pub use models::{
    UIElement,
    Locator,
    LocatorType,
    TksScript,
    TksStep,
    TksCommand,
    TksParam,
    DeviceInfo,
    Point,
    Bounds,
    ExecutionResult,
    StepResult,
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

    #[error("脚本执行错误: {0}")]
    ScriptExecuteError(String),

    #[error("无效的参数: {0}")]
    InvalidArgument(String),

    #[error("设备未连接")]
    DeviceNotConnected,

    #[error("项目路径无效: {0}")]
    InvalidProjectPath(String),

    #[error("OCR错误: {0}")]
    OcrError(String),
}

pub type Result<T> = std::result::Result<T, TkeError>;
