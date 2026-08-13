// CLI 模块 - 命令翻译层（只解析 CLI 参数 → 调用 tke 库，不含业务逻辑）
// 按四大块组织，与库层一一对应：① 直通 ② 原子方法 ③ 工作流 ④ 自有工具

// --help 总览渲染
pub mod help;

/// tke fix：补齐缺失的运行依赖（唯一会联网下载的命令）
pub mod fix;

/// tke report：把散落的多批检查汇总成一份全流程报告
pub mod report;

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
pub use workflow::{RunArgs, StepsArgs, HarnessArgs};
pub use fix::FixArgs;
pub use report::ReportArgs;
pub use tools::{FileCommands, AppCommands, DeviceCommands, ElementCommands};
