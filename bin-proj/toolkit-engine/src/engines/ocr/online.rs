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

    // 短超时 + 复用客户端：此前 Client::new() 无任何超时——OCR 服务挂掉(尤其 TCP 黑洞)时
    // 每轮采集都要陪 OS 级超时等一两分钟。一张截图的 OCR 几秒内该回，10s 封顶。
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    });
    let response = client
        .post(ocr_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| -> Box<dyn StdError + Send + Sync> {
            // reqwest 顶层 Display 只有 "error sending request"，真实原因（连接拒绝=服务没起 /
            // 超时=服务卡死 / DNS 解析失败=地址写错）藏在错误链里——全部展开，让人能对症
            let mut msg = format!("{}", e);
            let mut src = e.source();
            while let Some(s) = src {
                msg.push_str(&format!("：{}", s));
                src = s.source();
            }
            msg.into()
        })?;

    if !response.status().is_success() {
        return Err(format!("OCR service error: {}", response.status()).into());
    }

    let ocr_response: OnlineOcrResponse = response.json().await?;
    Ok(ocr_response.into())
}
