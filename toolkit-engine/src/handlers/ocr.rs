// OCR 命令处理器

use tke::{Result, TkeError, JsonOutput};
use std::path::PathBuf;

/// 处理 OCR 命令
pub async fn handle(
    image_path: PathBuf,
    online: bool,
    url: Option<String>,
    lang: String,
) -> Result<()> {
    let image_data = std::fs::read(&image_path)
        .map_err(|e| TkeError::IoError(e))?;

    let result = if online {
        let url = url.ok_or_else(|| {
            TkeError::InvalidArgument("在线模式需要提供 --url 参数".to_string())
        })?;
        tke::ocr(&image_data, true, &url).await
    } else {
        tke::ocr(&image_data, false, &lang).await
    };

    match result {
        Ok(ocr_result) => {
            JsonOutput::print(&ocr_result);
            Ok(())
        }
        Err(e) => {
            JsonOutput::print(serde_json::json!({
                "error": e.to_string()
            }));
            Err(TkeError::OcrError(e.to_string()))
        }
    }
}
