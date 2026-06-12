// Tools 模块（④ 自有工具）- tke 内置的设备/文件/识别工具
//   ocr     图片文字识别（离线 tesseract / 在线 API）
//   element 元素库管理（按坐标取元素落库, 自动 crop 模板图 + ocr）
//   file    Android 设备文件系统管理
//   app     设备应用管理（列表/卸载/启停/聚焦）
//   device  设备详细信息（硬件/电池/网络/prop）

pub mod ocr;
pub mod element;
pub mod file;
pub mod app;
pub mod device;

pub use file::FileManager;
pub use app::AppManager;
pub use device::DeviceManager;
