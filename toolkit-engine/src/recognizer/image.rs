// 图像查找模块 - 使用OpenCV进行图像匹配

use crate::{Result, TkeError, Point, Locator, LocatorType};
use std::path::PathBuf;
use std::collections::HashMap;
use std::process::Command;

/// 根据图像locator查找元素（用于CLI，直接输出JSON）
pub fn find_by_locator_json(
    project_path: &PathBuf,
    locators: &HashMap<String, Locator>,
    locator_name: &str,
    threshold: f32
) -> Result<()> {
    // 获取locator定义
    let locator = locators.get(locator_name)
        .ok_or_else(|| {
            let json = serde_json::json!({
                "success": false,
                "error": format!("Locator '{}' 未定义", locator_name)
            });
            println!("{}", serde_json::to_string(&json).unwrap());
            TkeError::ElementNotFound(format!("Locator '{}' 未定义", locator_name))
        })?;

    // 确保是图像类型
    if !matches!(locator.locator_type, LocatorType::Image) {
        let json = serde_json::json!({
            "success": false,
            "error": format!("Locator '{}' 不是图像类型", locator_name)
        });
        println!("{}", serde_json::to_string(&json).unwrap());
        return Err(TkeError::InvalidArgument(format!("Locator '{}' 不是图像类型", locator_name)));
    }

    let template_path = if let Some(ref path) = locator.path {
        project_path.join(path)
    } else {
        let json = serde_json::json!({
            "success": false,
            "error": "图像locator缺少path字段"
        });
        println!("{}", serde_json::to_string(&json).unwrap());
        return Err(TkeError::InvalidArgument("图像locator缺少path字段".to_string()));
    };

    let screenshot_path = project_path.join("workarea").join("current_screenshot.png");

    // 调用 tke-opencv 可执行文件进行模板匹配
    opencv_match_json(&screenshot_path, &template_path, threshold)?;
    Ok(())
}

/// 根据图像locator查找元素（用于脚本，返回Point）
pub fn find_by_locator(
    project_path: &PathBuf,
    locators: &HashMap<String, Locator>,
    locator_name: &str,
    threshold: f32
) -> Result<Point> {
    // 获取locator定义
    let locator = locators.get(locator_name)
        .ok_or_else(|| TkeError::ElementNotFound(format!("Locator '{}' 未定义", locator_name)))?;

    // 确保是图像类型
    if !matches!(locator.locator_type, LocatorType::Image) {
        return Err(TkeError::InvalidArgument(format!("Locator '{}' 不是图像类型", locator_name)));
    }

    let template_path = if let Some(ref path) = locator.path {
        project_path.join(path)
    } else {
        return Err(TkeError::InvalidArgument("图像locator缺少path字段".to_string()));
    };

    let screenshot_path = project_path.join("workarea").join("current_screenshot.png");

    // 调用 tke-opencv 可执行文件进行模板匹配
    opencv_match(&screenshot_path, &template_path, threshold)
}

/// 使用 OpenCV (Python 打包的可执行文件) 进行模板匹配（返回Point）
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

/// 使用 OpenCV 进行模板匹配（直接输出JSON）
fn opencv_match_json(screenshot_path: &PathBuf, template_path: &PathBuf, threshold: f32) -> Result<()> {
    // tke-opencv 可执行文件路径（与当前可执行文件同目录）
    let current_exe = std::env::current_exe()
        .map_err(|e| TkeError::IoError(e))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| TkeError::InvalidArgument("无法获取可执行文件目录".to_string()))?;
    let opencv_bin = exe_dir.join("tke-opencv");

    // 检查 tke-opencv 是否存在
    if !opencv_bin.exists() {
        let json = serde_json::json!({
            "success": false,
            "error": "找不到 tke-opencv 模块"
        });
        println!("{}", serde_json::to_string(&json).unwrap());
        return Err(TkeError::ElementNotFound("找不到 tke-opencv 模块".to_string()));
    }

    // 调用 tke-opencv
    let output = Command::new(&opencv_bin)
        .arg(screenshot_path.to_str().unwrap())
        .arg(template_path.to_str().unwrap())
        .arg(threshold.to_string())
        .output()
        .map_err(|e| {
            let json = serde_json::json!({
                "success": false,
                "error": format!("调用 tke-opencv 失败: {}", e)
            });
            println!("{}", serde_json::to_string(&json).unwrap());
            TkeError::ImageError(format!("调用 tke-opencv 失败: {}", e))
        })?;

    // 直接输出 tke-opencv 的 JSON 结果
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout.trim());

    // 解析检查是否成功（用于返回Result）
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| TkeError::JsonError(e))?;

    if result["success"].as_bool().unwrap_or(false) {
        Ok(())
    } else {
        Err(TkeError::ElementNotFound("图像匹配失败".to_string()))
    }
}
