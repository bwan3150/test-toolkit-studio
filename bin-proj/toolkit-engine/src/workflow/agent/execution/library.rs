// 落库：把 AI 操作的目标按"三级降级"写入元素库
//   结构元素 → 结构通道 + img + ocr；仅 OCR 元素 → 空结构 + ocr + img；
//   纯视觉(看图框选) → 空结构 + 空 ocr + 仅 img。
// 直接用 AI 选中/框选的目标落库（不回查 XML），产出可被 find_auto 跨设备回放。

use std::path::Path;

use crate::tools::element::{add_element_target, OcrChannel};
use crate::{Bounds, UIElement};

use super::super::transcript::Transcript;

/// 一次落库的结果摘要（供结束时人工审核：哪些新增、哪些更新了描述）
#[derive(Debug, Clone)]
pub struct SavedInfo {
    pub name: String,
    /// 元素库中此前不存在 → 本轮新增
    pub created: bool,
    /// 已有元素，但 AI 更正了其 desc → 本轮更新描述
    pub desc_updated: bool,
}

/// 按已确定目标落库
/// - structure: Some=结构元素(写结构通道)；None=OCR/视觉元素(结构留空)
/// - bounds: img 模板裁剪范围
/// - ocr: ocr 通道写入方式
/// 返回落库摘要（失败为 None）。
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
) -> Option<SavedInfo> {
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
        Ok(info) => {
            let saved = SavedInfo {
                name: name.to_string(),
                created: info.get("created").and_then(|v| v.as_bool()).unwrap_or(false),
                desc_updated: info.get("desc_updated").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            tx.log("element_saved", serde_json::json!({ "round": round, "name": name, "result": info }));
            Some(saved)
        }
        Err(e) => {
            tx.log(
                "element_save_error",
                serde_json::json!({ "round": round, "name": name, "error": e.to_string() }),
            );
            None
        }
    }
}
