// Engines 模块 - 纯逻辑引擎（无设备 IO，只做解析 / 匹配 / 识别）
//   fetcher    UI XML 解析器：归一化结构文件 → 元素列表（含生成的 xpath）
//   recognizer 元素识别引擎：xml / ocr / image / text 四通道按平台匹配
//   ocr        OCR 引擎：离线 tesseract / 在线 HTTP API
// 这些引擎被原子方法（atomic）与工具（tools）编排调用，自身不直接驱动设备。

pub mod fetcher;
pub mod recognizer;
pub mod ocr;

pub use fetcher::Fetcher;
pub use recognizer::Recognizer;
