// Atomic 模块 - 内部单项原子方法
// 三个核心指令，均要求指定设备 (-d/--device)：
//   fetch     采集页面：整张截图 + UI XML (+ OCR文字 + 元素列表 + 裁剪元素图)
//   recognize 定位元素：按策略（xml/ocr/图像匹配）找到元素在当前页面的坐标
//   control   执行操作：对坐标执行统一操作名（click/press/swipe/input...）
// 当前驱动为 adb（Android）；wda（iOS）/playwright（Web）为未来扩展位

pub mod fetch;
pub mod recognize;
pub mod control;

pub use fetch::{Fetch, FetchOptions, FetchResult};
pub use recognize::Recognize;
pub use control::{Control, ControlAction};
