// 统一 JSON 输出模块
// 所有 JSON 输出命令必须通过此模块输出，确保格式统一且无额外的 Error 信息

use serde::Serialize;
use std::process;

/// 统一 JSON 输出处理器
pub struct JsonOutput;

impl JsonOutput {
    /// 输出成功的 JSON 数据并正常退出（退出码 0）
    ///
    /// # 参数
    /// - `data`: 任何可序列化的数据结构
    ///
    /// # 示例
    /// ```
    /// JsonOutput::success(serde_json::json!({
    ///     "success": true,
    ///     "x": 100,
    ///     "y": 200
    /// }));
    /// ```
    pub fn success<T: Serialize>(data: T) {
        match serde_json::to_string(&data) {
            Ok(json) => {
                println!("{}", json);
                process::exit(0);
            }
            Err(e) => {
                Self::error(format!("JSON 序列化失败: {}", e));
            }
        }
    }

    /// 输出成功的 JSON 数据但不退出（用于后续还有操作的场景）
    ///
    /// # 参数
    /// - `data`: 任何可序列化的数据结构
    pub fn print<T: Serialize>(data: T) {
        match serde_json::to_string(&data) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                Self::error(format!("JSON 序列化失败: {}", e));
            }
        }
    }

    /// 输出错误的 JSON 并以退出码 1 退出（不打印额外的 Error: 信息）
    ///
    /// # 参数
    /// - `message`: 错误消息
    ///
    /// # 示例
    /// ```
    /// JsonOutput::error("未找到匹配点");
    /// ```
    pub fn error(message: impl AsRef<str>) -> ! {
        let json = serde_json::json!({
            "success": false,
            "error": message.as_ref()
        });

        match serde_json::to_string(&json) {
            Ok(json_str) => println!("{}", json_str),
            Err(_) => println!(r#"{{"success":false,"error":"JSON 序列化失败"}}"#),
        }

        process::exit(1);
    }

    /// 从 Result 中输出，成功返回值，失败则输出错误 JSON 并退出
    ///
    /// # 参数
    /// - `result`: Result 类型
    ///
    /// # 返回
    /// - 成功时返回 T
    /// - 失败时输出错误 JSON 并退出，不返回
    pub fn unwrap_or_exit<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(e) => Self::error(e.to_string()),
        }
    }

    /// 输出成功 JSON 并退出（简化版，自动包装 success: true）
    ///
    /// # 参数
    /// - `data`: 任何可序列化的数据结构
    ///
    /// # 示例
    /// ```
    /// JsonOutput::success_with(serde_json::json!({
    ///     "x": 100,
    ///     "y": 200
    /// }));
    /// // 输出: {"success":true,"x":100,"y":200}
    /// ```
    pub fn success_with<T: Serialize>(data: T) {
        let mut json = serde_json::json!({"success": true});

        if let serde_json::Value::Object(map) = data.serialize(serde_json::value::Serializer).unwrap_or(serde_json::Value::Null) {
            if let serde_json::Value::Object(ref mut base_map) = json {
                for (k, v) in map {
                    base_map.insert(k, v);
                }
            }
        }

        Self::success(json);
    }
}
