// Atomic 模块（② 原子方法）- 内部单项原子指令
// 四个核心指令，均要求指定设备 (-d/--device)：
//   refresh   刷新页面状态：截图 + UI XML 采集到设备缓存工作区 (+ OCR / 剪裁元素图)
//   fetch     提取元素：解析当前页面的全部 UI 元素列表（含 xpath）
//   recognize 定位元素：按策略（xml/ocr/图像匹配）找到元素在当前页面的坐标
//   control   执行操作：对坐标执行统一操作名（click/press/swipe/input...）
// 支撑引擎：
//   controller 设备驱动（当前 adb；wda/playwright 为未来扩展位）
//   fetcher    UI XML 解析器（元素提取 / xpath 生成）
//   recognizer 元素识别引擎（xml/ocr/图像三通道）

pub mod refresh;
pub mod fetch;
pub mod recognize;
pub mod control;

pub mod controller;
pub mod fetcher;
pub mod recognizer;

pub use refresh::{Refresh, RefreshOptions, RefreshResult};
pub use fetch::Fetch;
pub use recognize::Recognize;
pub use control::{Control, ControlAction};
pub use controller::Controller;
pub use fetcher::Fetcher;
pub use recognizer::Recognizer;
