// Tools 模块（④ 自有工具）- tke 内置的设备实用工具
//   file    Android 设备文件系统管理
//   app     设备应用管理（列表/卸载/启停/聚焦）
//   device  设备详细信息（硬件/电池/网络/prop）
//   element 元素库管理（编排 recognize + 截图剪裁 + ocr 落库）
// 注：OCR 引擎已移至 engines::ocr（纯逻辑引擎）；此处仅保留设备相关工具。

pub mod element;
pub mod file;
pub mod app;
pub mod device;
pub mod discover;

pub use file::FileManager;
pub use app::AppManager;
pub use device::DeviceManager;
