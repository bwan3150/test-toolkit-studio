// Passthrough 模块（① 直通）- 统一的外部二进制定位与直通机制
// tke 作为所有测试工具的统一入口：tke <工具名> <原生指令>
// 工具二进制与 tke 放在同一目录（bin/<platform>/），新增工具零代码改动；
// 缺失时报 "<工具名> 可执行文件缺失或不完整"。
//
// ToolManager 是唯一的二进制定位器：
//   - resolve(name)      解析同目录二进制路径（adb/aapt/chromedriver/go-ios 等程序化调用共用）
//   - passthrough(name)  直通执行（adb 特例：-d/--device 自动注入为 adb -s <id>）
//   - list_available()   扫描同目录可直通二进制（用于 --help 动态清单）

mod manager;

pub use manager::ToolManager;
