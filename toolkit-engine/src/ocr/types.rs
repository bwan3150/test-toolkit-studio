use serde::{Deserialize, Serialize};

/// OCR 识别结果中的单个文本区域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrText {
    pub text: String,
    pub bbox: Vec<[f32; 2]>, // [[x1,y1], [x2,y2], [x3,y3], [x4,y4]]
    pub confidence: f32,
}

/// OCR 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub texts: Vec<OcrText>,
}

/// 在线 OCR 请求
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OnlineOcrRequest {
    pub image: String,
}

/// 在线 OCR 响应
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OnlineOcrResponse {
    pub processing_time: f64,
    pub results: Vec<OnlineOcrTextResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OnlineOcrTextResult {
    pub text: String,
    pub bounding_box: Vec<[f32; 2]>,
    pub confidence: f32,
}

impl From<OnlineOcrResponse> for OcrResult {
    fn from(resp: OnlineOcrResponse) -> Self {
        OcrResult {
            texts: resp.results.into_iter().map(|r| OcrText {
                text: r.text,
                bbox: r.bounding_box,
                confidence: r.confidence,
            }).collect(),
        }
    }
}
