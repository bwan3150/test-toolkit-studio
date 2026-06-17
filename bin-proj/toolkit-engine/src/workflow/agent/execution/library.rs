// 落库：把 AI 操作的目标按"三级降级"写入元素库
//   结构元素 → 结构通道 + img + ocr；仅 OCR 元素 → 空结构 + ocr + img；
//   纯视觉(看图框选) → 空结构 + 空 ocr + 仅 img。
// 直接用 AI 选中/框选的目标落库（不回查 XML），产出可被 find_auto 跨设备回放。

use std::path::Path;

use crate::tools::element::{add_element_target, OcrChannel};
use crate::{Bounds, UIElement};

use super::super::transcript::Transcript;

/// 按已确定目标落库
/// - structure: Some=结构元素(写结构通道)；None=OCR/视觉元素(结构留空)
/// - bounds: img 模板裁剪范围
/// - ocr: ocr 通道写入方式
pub async fn save_target(
    device: &str,
    element_path: &Path,
    name: &str,
    desc: &Option<String>,
    bounds: Bounds,
    structure: Option<&UIElement>,
    ocr: OcrChannel,
    tx: &mut Transcript,
    round: usize,
) {
    match add_element_target(
        device.to_string(),
        element_path,
        name,
        desc.clone(),
        bounds,
        structure,
        ocr,
        false, // 不强制覆盖已有 img/ocr
    )
    .await
    {
        Ok(info) => tx.log(
            "element_saved",
            serde_json::json!({ "round": round, "name": name, "result": info }),
        ),
        Err(e) => tx.log(
            "element_save_error",
            serde_json::json!({ "round": round, "name": name, "error": e.to_string() }),
        ),
    }
}
