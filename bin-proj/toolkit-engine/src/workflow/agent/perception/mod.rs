// 【感知】采集页面状态 + 渲染元素列表
//   capture  refresh(截图+XML) + 解析元素 + 存截图快照
//   render   元素列表 → 喂给 AI 的文本

pub mod capture;
pub mod render;

pub use capture::{capture, Perceived};
pub use render::render_element_list;
