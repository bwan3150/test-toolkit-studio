// 落库：把 AI 操作的元素按坐标写入元素库（复用 tools::element::add_element）

use std::path::Path;

use super::super::transcript::Transcript;

/// 按坐标落库（cached=true 复用本轮 refresh 的工作区状态，避免重复采集）
pub async fn save_element(
    device: &str,
    element_path: &Path,
    name: &str,
    desc: &Option<String>,
    x: i32,
    y: i32,
    tx: &mut Transcript,
    round: usize,
) {
    match crate::tools::element::add_element(
        device.to_string(),
        Some(element_path),
        name,
        desc.clone(),
        x,
        y,
        true,  // cached
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
