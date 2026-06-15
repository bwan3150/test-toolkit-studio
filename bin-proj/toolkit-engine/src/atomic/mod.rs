// Atomic 模块（② 原子方法）- 四个核心原子指令，均要求指定设备 (-d/--device)
//   refresh   刷新页面状态：截图 + UI XML 采集到设备缓存工作区 (+ OCR / 剪裁元素图)
//   fetch     提取元素：解析当前页面的全部 UI 元素列表（含 xpath）
//   recognize 定位元素：按策略（xml/ocr/图像匹配）找到元素在当前页面的坐标
//   control   执行操作：对坐标执行统一操作名（click/press/swipe/input...）
//
// 原子方法是「编排层」：组合 drivers（设备驱动）+ engines（解析/识别引擎）完成单步动作，
// 自身不实现协议对接，也不实现解析算法。

pub mod refresh;
pub mod fetch;
pub mod recognize;
pub mod control;

pub use refresh::{Refresh, RefreshOptions, RefreshResult};
pub use fetch::Fetch;
pub use recognize::Recognize;
pub use control::{Control, ControlAction};
