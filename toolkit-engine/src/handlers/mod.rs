// Handlers 模块 - 统一管理所有命令处理器

pub mod controller;
pub mod fetcher;
pub mod recognizer;
pub mod parser;
pub mod runner;
pub mod ocr;
pub mod adb;

// 重新导出命令枚举，方便 main.rs 使用
pub use controller::ControllerCommands;
pub use fetcher::FetcherCommands;
pub use recognizer::RecognizerCommands;
pub use parser::ParserCommands;
pub use runner::RunCommands;
