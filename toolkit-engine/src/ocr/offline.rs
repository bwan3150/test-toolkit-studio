#[cfg(feature = "ocr-offline")]
use super::types::{OcrResult, OcrText};
#[cfg(feature = "ocr-offline")]
use std::error::Error;
#[cfg(feature = "ocr-offline")]
use tesseract::TesseractAPI;

#[cfg(feature = "ocr-offline")]
pub fn recognize_offline(
    image_data: &[u8],
    language: &str,
) -> Result<OcrResult, Box<dyn Error>> {
    let img = image::load_from_memory(image_data)?;
    let mut api = TesseractAPI::new()?;
    api.init("", language)?;

    let gray_img = img.to_luma8();
    let (width, height) = gray_img.dimensions();

    api.set_image(
        gray_img.as_raw(),
        width as i32,
        height as i32,
        1,
        width as i32,
    )?;

    let text = api.get_utf8_text()?;
    let mut texts = Vec::new();

    if !text.trim().is_empty() {
        let confidence = api.get_mean_confidence() as f32 / 100.0;
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
