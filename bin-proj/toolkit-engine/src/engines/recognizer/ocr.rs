// OCR 在线识别模块 - 通过 OCR 服务识别屏幕文字并定位元素
// 返回 (中心点, 识别文字的实时边界框)

use crate::{Result, TkeError, Point, Bounds, Locator};
use crate::engines::ocr::{ocr, OcrResult};
use std::path::Path;
use tracing::debug;

// OCR 服务地址改由参数层提供（决策 5：可配；默认在 utils::params）

/// 通过 OCR 在线识别查找元素
pub async fn find_by_ocr(
    screenshot_path: &Path,
    locator: &Locator,
) -> Result<(Point, Bounds)> {
    let ocr_text = locator.ocr.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 ocr 通道", locator.name)
        ))?;

    debug!("OCR 查找文字: {}", ocr_text);

    if !screenshot_path.exists() {
        return Err(TkeError::ElementNotFound(
            "截图文件不存在，请先执行采集".to_string()
        ));
    }

    let image_data = std::fs::read(screenshot_path)
        .map_err(|e| TkeError::IoError(e))?;

    // 调用在线 OCR（地址来自参数层，单一来源、可配）
    let ocr_url = crate::utils::params::ocr_url();
    let result = ocr(&image_data, true, &ocr_url).await
        .map_err(|e| TkeError::OcrError(format!("OCR 识别失败: {}", e)))?;

    debug!("OCR 识别到 {} 个文字区域", result.texts.len());

    // 查找匹配文本
    find_matching_text(&result, ocr_text, &locator.name)
}

/// 在 OCR 结果中查找匹配的文本
fn find_matching_text(result: &OcrResult, target: &str, locator_name: &str) -> Result<(Point, Bounds)> {
    let target_lower = target.to_lowercase();

    // 优先精确匹配
    for text_item in &result.texts {
        if text_item.text.to_lowercase() == target_lower {
            let (point, bounds) = bbox_geometry(&text_item.bbox);
            debug!("OCR 精确匹配: '{}' -> ({}, {})", text_item.text, point.x, point.y);
            return Ok((point, bounds));
        }
    }

    // 其次包含匹配
    for text_item in &result.texts {
        let text_lower = text_item.text.to_lowercase();
        if text_lower.contains(&target_lower) || target_lower.contains(&text_lower) {
            let (point, bounds) = bbox_geometry(&text_item.bbox);
            debug!("OCR 包含匹配: '{}' 包含 '{}' -> ({}, {})",
                   text_item.text, target, point.x, point.y);
            return Ok((point, bounds));
        }
    }

    Err(TkeError::ElementNotFound(
        format!("OCR 未找到文字 '{}' (元素: {})", target, locator_name)
    ))
}

/// 从 OCR bbox 计算中心点和边界框
/// bbox 格式: [[x1,y1], [x2,y1], [x2,y2], [x1,y2]]
fn bbox_geometry(bbox: &Vec<[f32; 2]>) -> (Point, Bounds) {
    if bbox.len() < 4 {
        return (Point::new(0, 0), Bounds::new(0, 0, 0, 0));
    }

    let x1 = bbox[0][0] as i32;
    let y1 = bbox[0][1] as i32;
    let x2 = bbox[2][0] as i32;
    let y2 = bbox[2][1] as i32;

    (Point::new((x1 + x2) / 2, (y1 + y2) / 2), Bounds::new(x1, y1, x2, y2))
}
