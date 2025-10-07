// Toolkit Engine 核心库
// 提供自动化测试的核心功能

pub mod controller;
pub mod locator_fetcher;
pub mod recognizer;
pub mod script_interpreter;
pub mod script_parser;
pub mod runner;
pub mod models;
pub mod adb_manager;
pub mod ocr;

// 导出主要类型和功能
pub use controller::Controller;
pub use locator_fetcher::LocatorFetcher;
pub use recognizer::Recognizer;
pub use script_interpreter::ScriptInterpreter;
pub use script_parser::ScriptParser;
pub use runner::Runner;
pub use adb_manager::AdbManager;

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