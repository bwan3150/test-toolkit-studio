// Handlers 模块 - CLI 命令处理器
// 按四大块组织：① 直通 ② 原子方法 ③ 工作流 ④ 自有工具

// ① 直通（统一处理所有外部二进制，含 adb -d 注入特例）
pub mod passthrough;

// ② 原子方法
pub mod atomic;

// ③ 工作流
pub mod workflow;

// ④ 自有工具
pub mod tools;

// 重新导出命令参数类型，方便 main.rs 使用
pub use atomic::{RefreshArgs, FetchArgs, RecognizeArgs, ControlCommands};
pub use workflow::{RunArgs, StepsArgs, CaseArgs};
pub use tools::{FileCommands, AppCommands, DeviceCommands};
