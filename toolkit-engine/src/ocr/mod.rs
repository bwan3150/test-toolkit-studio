//! OCR 模块 - 在线/离线文字识别

#[cfg(feature = "ocr-offline")]
mod offline;
#[cfg(feature = "ocr-online")]
mod online;
mod types;

pub use types::{OcrResult, OcrText};
use std::error::Error as StdError;

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
