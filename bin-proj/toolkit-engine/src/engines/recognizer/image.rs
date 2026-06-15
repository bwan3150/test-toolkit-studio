// 图像查找模块 - 使用 OpenCV (tke-opencv) 进行图像模板匹配
// 元素图路径相对 element.json 所在目录；边界框由模板尺寸反推

use crate::{Result, TkeError, Point, Bounds, Locator};
use std::path::Path;
use std::process::Command;

/// 根据 locator 的 img 通道查找元素，返回 (中心点, 模板尺寸反推的边界框)
pub fn find_by_image(
    screenshot_path: &Path,
    element_dir: &Path,
    locator: &Locator,
    threshold: f32,
) -> Result<(Point, Bounds)> {
    let img = locator.img.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 img 通道", locator.name)
        ))?;

    // 元素图路径相对元素库文件所在目录
    let template_path = element_dir.join(img);
    if !template_path.exists() {
        return Err(TkeError::ElementNotFound(format!(
            "元素图不存在: {}", template_path.display()
        )));
    }

    let point = opencv_match(screenshot_path, &template_path, threshold)?;

    // 用模板图尺寸反推边界框（中心点 ± 宽高/2）
    let bounds = match image::image_dimensions(&template_path) {
        Ok((w, h)) => Bounds::new(
            point.x - (w as i32) / 2,
            point.y - (h as i32) / 2,
            point.x + (w as i32) / 2,
            point.y + (h as i32) / 2,
        ),
        Err(_) => Bounds::new(point.x, point.y, point.x, point.y),
    };

    Ok((point, bounds))
}

/// 调用 tke-opencv（与 tke 同目录）进行模板匹配
fn opencv_match(screenshot_path: &Path, template_path: &Path, threshold: f32) -> Result<Point> {
    // 与其它同目录工具一样，由统一的 ToolManager 定位
    let opencv_bin = crate::ToolManager::resolve("tke-opencv")?;

    let output = Command::new(&opencv_bin)
        .arg(screenshot_path.to_str().unwrap_or_default())
        .arg(template_path.to_str().unwrap_or_default())
        .arg(threshold.to_string())
        .output()
        .map_err(|e| TkeError::ImageError(format!("调用 tke-opencv 失败: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| TkeError::JsonError(e))?;

    if result["success"].as_bool().unwrap_or(false) {
        let x = result["x"].as_i64()
            .ok_or_else(|| TkeError::ElementNotFound("JSON 响应缺少 x 字段".to_string()))? as i32;
        let y = result["y"].as_i64()
            .ok_or_else(|| TkeError::ElementNotFound("JSON 响应缺少 y 字段".to_string()))? as i32;

        Ok(Point::new(x, y))
    } else {
        let error = result["error"].as_str().unwrap_or("图像匹配失败");
        Err(TkeError::ElementNotFound(error.to_string()))
    }
}
