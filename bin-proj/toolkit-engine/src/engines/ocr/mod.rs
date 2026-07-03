//! OCR 模块 - 在线/离线文字识别

mod enrich;
#[cfg(feature = "ocr-offline")]
mod offline;
#[cfg(feature = "ocr-online")]
mod online;
mod types;

pub use enrich::{enrich_with_ocr, resolve_ocr, OcrSource};
pub use types::{OcrResult, OcrText};
use std::error::Error as StdError;
use std::sync::OnceLock;

/// 进程级 OCR 来源（在线 URL / 离线语言）：由 `tke run --ocr` / `tke harness --ocr` 设置一次，
/// 识别引擎（recognizer::ocr）查询以决定回放时元素/断言走 online 还是 offline。
/// 未设置则回退「在线 + params::ocr_url」（保持旧行为）。
/// 注册表放在类型的老家（本模块）——此前挂在 utils/params，构成底层 utils 反向依赖上层 engines。
static OCR_SOURCE: OnceLock<OcrSource> = OnceLock::new();

/// 设置进程级 OCR 来源（run/harness 处理器在跑脚本前调用一次）
pub fn set_ocr_source(src: OcrSource) {
    let _ = OCR_SOURCE.set(src);
}

/// 查询进程级 OCR 来源；未显式设置返回 None（调用方回退「在线 + ocr_url」）
pub fn ocr_source() -> Option<OcrSource> {
    OCR_SOURCE.get().cloned()
}

/// OCR 识别（统一入口）
///
/// # 参数
/// - image_data: 图片字节数据
/// - online: true=在线OCR, false=离线OCR
/// - param: 在线模式=完整URL(如"http://localhost:8000/ocr"), 离线模式=语言代码(如"eng","chi_sim")
pub async fn ocr(
    image_data: &[u8],
    online: bool,
    param: &str,
) -> Result<OcrResult, Box<dyn StdError + Send + Sync>> {
    if online {
        #[cfg(not(feature = "ocr-online"))]
        return Err("ocr-online feature not enabled".into());

        #[cfg(feature = "ocr-online")]
        online::recognize_online(image_data, param).await
    } else {
        #[cfg(not(feature = "ocr-offline"))]
        return Err("ocr-offline feature not enabled".into());

        #[cfg(feature = "ocr-offline")]
        {
            let image_data = image_data.to_vec();
            let param = param.to_string();
            tokio::task::spawn_blocking(move || {
                offline::recognize_offline(&image_data, &param)
            })
            .await?
        }
    }
}
