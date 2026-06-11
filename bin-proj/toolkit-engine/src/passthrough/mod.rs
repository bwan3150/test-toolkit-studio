// Passthrough 模块（① 直通）- 统一的外部二进制直通机制
// tke 作为所有测试工具的统一入口：tke <工具名> <原生指令>
// 工具二进制与 tke 放在同一目录（bin/<platform>/），新增工具零代码改动；
// 缺失时报 "<工具名> 可执行文件缺失或不完整"
//
// adb 特例：-d/--device 自动注入为 adb -s <id>
// AdbManager/AaptManager 为内部模块定位 adb/aapt 路径的管理器

mod manager;
mod adb_manager;
mod aapt_manager;

pub use manager::ToolManager;
pub use adb_manager::AdbManager;
pub use aapt_manager::AaptManager;
