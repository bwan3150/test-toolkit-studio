// 错误处理

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiTesterError {
    #[error("TKE执行错误: {0}")]
    TkeError(String),

    #[error("OCR服务错误: {0}")]
    OcrError(String),

    #[error("AI Agent错误: {0}")]
    AgentError(String),

    #[error("设备连接错误: {0}")]
    DeviceError(String),

    #[error("文件系统错误: {0}")]
    FileSystemError(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("知识库错误: {0}")]
    KnowledgeBaseError(String),

    #[error("未知错误: {0}")]
    Unknown(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AiTesterError>;