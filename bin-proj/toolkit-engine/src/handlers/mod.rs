// Handlers 模块 - 统一管理所有命令处理器
// 按三大块组织：① 工具直通 ② 原子方法 ③ 工作流 (+ 设备工具命令)

// ① 工具直通
pub mod tools;
pub mod adb;
pub mod aapt;

// ② 原子方法
pub mod refresh;
pub mod fetch;
pub mod recognize;
pub mod control;

// ③ 工作流
pub mod runner;
pub mod steps;
pub mod case_cmd;
pub mod printer;

// 设备工具命令
pub mod ocr;
pub mod file;
pub mod app;
pub mod device;

// 重新导出命令枚举，方便 main.rs 使用
pub use refresh::RefreshArgs;
pub use fetch::FetchArgs;
pub use recognize::RecognizeArgs;
pub use control::ControlCommands;
pub use runner::RunArgs;
pub use steps::StepsArgs;
pub use case_cmd::CaseArgs;
pub use printer::EventPrinter;
pub use file::FileCommands;
pub use app::AppCommands;
pub use device::DeviceCommands;
