#[cfg(feature = "ocr-online")]
use super::types::{OnlineOcrRequest, OnlineOcrResponse, OcrResult};
#[cfg(feature = "ocr-online")]
use base64::{engine::general_purpose, Engine as _};
#[cfg(feature = "ocr-online")]
use std::error::Error as StdError;

#[cfg(feature = "ocr-online")]
pub async fn recognize_online(
    image_data: &[u8],
    ocr_url: &str,
) -> Result<OcrResult, Box<dyn StdError + Send + Sync>> {
    let base64_image = general_purpose::STANDARD.encode(image_data);
    let request_body = OnlineOcrRequest { image: base64_image };

    let client = reqwest::Client::new();
    let response = client
        .post(ocr_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("OCR service error: {}", response.status()).into());
    }

    let ocr_response: OnlineOcrResponse = response.json().await?;
    Ok(ocr_response.into())
}
