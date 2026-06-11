#[cfg(feature = "ocr-offline")]
use super::types::{OcrResult, OcrText};
#[cfg(feature = "ocr-offline")]
use std::error::Error as StdError;
#[cfg(feature = "ocr-offline")]
use tesseract_rs::TesseractAPI;

#[cfg(feature = "ocr-offline")]
pub fn recognize_offline(
    image_data: &[u8],
    language: &str,
) -> Result<OcrResult, Box<dyn StdError + Send + Sync>> {
    let img = image::load_from_memory(image_data)?;

    let api = TesseractAPI::new();

    api.init("", language)
        .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;

    let gray_img = img.to_luma8();
    let (width, height) = gray_img.dimensions();

    api.set_image(
        gray_img.as_raw(),
        width as i32,
        height as i32,
        1,
        width as i32,
    ).map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;

    let text = api.get_utf8_text()
        .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)?;

    let mut texts = Vec::new();

    if !text.trim().is_empty() {
        // Note: mean_confidence() method not available in current tesseract-rs version
        // Using a default confidence value
        let confidence = 0.9; // Default confidence value
        let bbox = vec![
            [0.0, 0.0],
            [width as f32, 0.0],
            [width as f32, height as f32],
            [0.0, height as f32],
        ];

        texts.push(OcrText {
            text: text.trim().to_string(),
            bbox,
            confidence,
        });
    }

    Ok(OcrResult { texts })
}
