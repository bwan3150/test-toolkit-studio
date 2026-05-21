// 图像查找模块 - 使用 OpenCV 进行图像模板匹配

use crate::{Result, TkeError, Point, Locator, JsonOutput};
use std::path::PathBuf;
use std::process::Command;

/// 根据图像 locator 查找元素（用于脚本，返回 Point）
pub fn find_by_image(
    project_path: &PathBuf,
    locator: &Locator,
    threshold: f32,
) -> Result<Point> {
    let img_path = locator.img_path.as_ref()
        .ok_or_else(|| TkeError::ElementNotFound(
            format!("元素 '{}' 未定义 img_path 字段", locator.name)
        ))?;

    let template_path = project_path.join(img_path);
    let screenshot_path = project_path.join("workarea").join("current_screenshot.png");

    // 调用 tke-opencv 可执行文件进行模板匹配
    opencv_match(&screenshot_path, &template_path, threshold)
}

/// 根据图像 locator 查找元素（用于 CLI，直接输出 JSON）
pub fn find_by_image_json(
    project_path: &PathBuf,
    locator: &Locator,
    threshold: f32,
) -> Result<()> {
    let img_path = locator.img_path.as_ref()
        .ok_or_else(|| {
            JsonOutput::print_error(format!("元素 '{}' 未定义 img_path 字段", locator.name));
            TkeError::ElementNotFound(format!("元素 '{}' 未定义 img_path 字段", locator.name))
        })?;

    let template_path = project_path.join(img_path);
    let screenshot_path = project_path.join("workarea").join("current_screenshot.png");

    // 调用 tke-opencv 可执行文件进行模板匹配
    opencv_match_json(&screenshot_path, &template_path, threshold)?;
    Ok(())
}

/// 使用 OpenCV (Python 打包的可执行文件) 进行模板匹配（返回 Point）
fn opencv_match(screenshot_path: &PathBuf, template_path: &PathBuf, threshold: f32) -> Result<Point> {
    // tke-opencv 可执行文件路径（与当前可执行文件同目录）
    let current_exe = std::env::current_exe()
        .map_err(|e| TkeError::IoError(e))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| TkeError::InvalidArgument("无法获取可执行文件目录".to_string()))?;
    let opencv_bin = exe_dir.join("tke-opencv");

    // 检查 tke-opencv 是否存在
    if !opencv_bin.exists() {
        return Err(TkeError::ElementNotFound("找不到 tke-opencv 模块".to_string()));
    }

    // 调用 tke-opencv
    let output = Command::new(&opencv_bin)
        .arg(screenshot_path.to_str().unwrap())
        .arg(template_path.to_str().unwrap())
        .arg(threshold.to_string())
        .output()
        .map_err(|e| TkeError::ImageError(format!("调用 tke-opencv 失败: {}", e)))?;

    // 解析 JSON 输出
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| TkeError::JsonError(e))?;

    // 检查是否成功并返回 Point
    if result["success"].as_bool().unwrap_or(false) {
        let x = result["x"].as_i64()
            .ok_or_else(|| TkeError::ElementNotFound("JSON 响应缺少 x 字段".to_string()))?
            as i32;
        let y = result["y"].as_i64()
            .ok_or_else(|| TkeError::ElementNotFound("JSON 响应缺少 y 字段".to_string()))?
            as i32;

        Ok(Point::new(x, y))
    } else {
        let error = result["error"].as_str().unwrap_or("图像匹配失败");
        Err(TkeError::ElementNotFound(error.to_string()))
    }
}

/// 使用 OpenCV 进行模板匹配（直接输出 JSON）
fn opencv_match_json(screenshot_path: &PathBuf, template_path: &PathBuf, threshold: f32) -> Result<()> {
    // tke-opencv 可执行文件路径（与当前可执行文件同目录）
    let current_exe = std::env::current_exe()
        .map_err(|e| TkeError::IoError(e))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| TkeError::InvalidArgument("无法获取可执行文件目录".to_string()))?;
    let opencv_bin = exe_dir.join("tke-opencv");

    // 检查 tke-opencv 是否存在
    if !opencv_bin.exists() {
        JsonOutput::print_error("找不到 tke-opencv 模块");
        return Err(TkeError::ElementNotFound("找不到 tke-opencv 模块".to_string()));
    }

    // 调用 tke-opencv
    let output = Command::new(&opencv_bin)
        .arg(screenshot_path.to_str().unwrap())
        .arg(template_path.to_str().unwrap())
        .arg(threshold.to_string())
        .output()
        .map_err(|e| {
            JsonOutput::print_error(format!("调用 tke-opencv 失败: {}", e));
            TkeError::ImageError(format!("调用 tke-opencv 失败: {}", e))
        })?;

    // 直接输出 tke-opencv 的 JSON 结果
    let stdout = String::from_utf8_lossy(&output.stdout);
    JsonOutput::print_raw(stdout.trim());

    // 解析检查是否成功（用于返回 Result）
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| TkeError::JsonError(e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(())
    } else {
        Err(TkeError::ElementNotFound("图像匹配失败".to_string()))
    }
}
