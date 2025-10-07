#[cfg(feature = "ocr-online")]
use super::types::{OnlineOcrRequest, OnlineOcrResponse, OcrResult};
#[cfg(feature = "ocr-online")]
use base64::{engine::general_purpose, Engine as _};
#[cfg(feature = "ocr-online")]
use std::error::Error;

#[cfg(feature = "ocr-online")]
pub async fn recognize_online(
    image_data: &[u8],
    ocr_host: &str,
) -> Result<OcrResult, Box<dyn Error>> {
    let base64_image = general_purpose::STANDARD.encode(image_data);
    let request_body = OnlineOcrRequest { image: base64_image };
    let url = format!("{}/ocr", ocr_host.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
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
