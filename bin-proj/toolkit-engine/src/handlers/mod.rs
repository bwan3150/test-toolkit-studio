// Handlers 模块 - 统一管理所有命令处理器
// 按三大块组织：① 工具直通 ② 原子方法 ③ 工作流 (+ legacy 兼容命令)

// ① 工具直通
pub mod tools;
pub mod adb;
pub mod aapt;

// ② 原子方法
pub mod fetch;
pub mod recognize;
pub mod control;

// ③ 工作流
pub mod runner;

// legacy 兼容命令（Electron App handlers 仍在调用）
pub mod controller;
pub mod fetcher;
pub mod recognizer;
pub mod ocr;
pub mod file;
pub mod app;
pub mod device;

// 重新导出命令枚举，方便 main.rs 使用
pub use fetch::FetchArgs;
pub use recognize::RecognizeArgs;
pub use control::ControlCommands;
pub use runner::RunCommands;
pub use controller::ControllerCommands;
pub use fetcher::FetcherCommands;
pub use recognizer::RecognizerCommands;
pub use file::FileCommands;
pub use app::AppCommands;
pub use device::DeviceCommands;
