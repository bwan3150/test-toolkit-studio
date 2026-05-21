// Handlers 模块 - 统一管理所有命令处理器

pub mod controller;
pub mod fetcher;
pub mod recognizer;
pub mod runner;
pub mod ocr;
pub mod adb;
pub mod aapt;
pub mod file;
pub mod app;
pub mod device;

// 重新导出命令枚举，方便 main.rs 使用
pub use controller::ControllerCommands;
pub use fetcher::FetcherCommands;
pub use recognizer::RecognizerCommands;
pub use runner::RunCommands;
pub use file::FileCommands;
pub use app::AppCommands;
pub use device::DeviceCommands;
