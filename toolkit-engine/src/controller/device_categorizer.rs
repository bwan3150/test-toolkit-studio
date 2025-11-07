// 设备分类器 - 负责将设备按连接状态和保存状态分类
// 分类逻辑:
// 1. 组A: 已连接但未保存的iOS设备
// 2. 组B: 已连接但未保存的Android设备
// 3. 组C: 已保存且已连接的iOS设备
// 4. 组D: 已保存且已连接的Android设备
// 5. 组E: 已保存但未连接的iOS设备
// 6. 组F: 已保存但未连接的Android设备
// 最终排列: iOS: A - C - E, Android: B - D - F

use crate::{Result, TkeError};
use std::path::PathBuf;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

/// 已保存设备信息
#[derive(Debug, Default)]
pub struct SavedDevicesInfo {
    pub android: Vec<String>,
    pub ios: Vec<String>,
}

/// 分类后的设备信息
#[derive(Debug, Serialize, Deserialize)]
pub struct CategorizedDevices {
    pub ios: IosCategorized,
    pub android: AndroidCategorized,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IosCategorized {
    pub unsaved_connected: Vec<String>,
    pub saved_connected: Vec<String>,
    pub saved_disconnected: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AndroidCategorized {
    pub unsaved_connected: Vec<String>,
    pub saved_connected: Vec<String>,
    pub saved_disconnected: Vec<String>,
}

pub struct DeviceCategorizer;

impl DeviceCategorizer {
    /// 加载项目中保存的设备配置
    pub fn load_saved_devices(project_path: &PathBuf) -> Result<SavedDevicesInfo> {
        use std::fs;

        let devices_dir = project_path.join("devices");
        let mut android_saved = Vec::new();
        let mut ios_saved = Vec::new();

        if !devices_dir.exists() {
            return Ok(SavedDevicesInfo::default());
        }

        let entries = fs::read_dir(&devices_dir)
            .map_err(|e| TkeError::IoError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| TkeError::IoError(e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                // 读取yaml文件
                let content = fs::read_to_string(&path)
                    .map_err(|e| TkeError::IoError(e))?;

                // 简单解析yaml来获取platform和设备标识
                if content.contains("platform: android") || content.contains("platform: Android") {
                    // Android设备，查找deviceId
                    if let Some(device_id) = Self::extract_yaml_field(&content, "deviceId") {
                        android_saved.push(device_id);
                    }
                } else if content.contains("platform: ios") || content.contains("platform: iOS") {
                    // iOS设备，查找udid
                    if let Some(udid) = Self::extract_yaml_field(&content, "udid") {
                        ios_saved.push(udid);
                    }
                }
            }
        }

        Ok(SavedDevicesInfo {
            android: android_saved,
            ios: ios_saved,
        })
    }

    /// 从yaml内容中提取字段值
    fn extract_yaml_field(content: &str, field: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("{}:", field)) {
                let value = trimmed.trim_start_matches(&format!("{}:", field)).trim();
                // 移除引号
                let value = value.trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    /// 分类设备
    ///
    /// # 参数
    /// - `connected_android`: 所有已连接的Android设备ID列表
    /// - `connected_ios`: 所有已连接的iOS设备UDID列表
    /// - `saved_devices`: 项目中已保存的设备配置信息
    ///
    /// # 返回
    /// 分类后的设备信息，按照 iOS: A-C-E, Android: B-D-F 的顺序排列
    pub fn categorize_devices(
        connected_android: Vec<String>,
        connected_ios: Vec<String>,
        saved_devices: SavedDevicesInfo,
    ) -> CategorizedDevices {
        let saved_android_set: HashSet<_> = saved_devices.android.iter().cloned().collect();
        let saved_ios_set: HashSet<_> = saved_devices.ios.iter().cloned().collect();

        let connected_android_set: HashSet<_> = connected_android.iter().cloned().collect();
        let connected_ios_set: HashSet<_> = connected_ios.iter().cloned().collect();

        // 组A: 已连接但未保存的iOS设备
        let group_a: Vec<_> = connected_ios.iter()
            .filter(|id| !saved_ios_set.contains(*id))
            .cloned()
            .collect();

        // 组B: 已连接但未保存的Android设备
        let group_b: Vec<_> = connected_android.iter()
            .filter(|id| !saved_android_set.contains(*id))
            .cloned()
            .collect();

        // 组C: 已保存且已连接的iOS设备
        let group_c: Vec<_> = saved_devices.ios.iter()
            .filter(|id| connected_ios_set.contains(*id))
            .cloned()
            .collect();

        // 组D: 已保存且已连接的Android设备
        let group_d: Vec<_> = saved_devices.android.iter()
            .filter(|id| connected_android_set.contains(*id))
            .cloned()
            .collect();

        // 组E: 已保存但未连接的iOS设备
        let group_e: Vec<_> = saved_devices.ios.iter()
            .filter(|id| !connected_ios_set.contains(*id))
            .cloned()
            .collect();

        // 组F: 已保存但未连接的Android设备
        let group_f: Vec<_> = saved_devices.android.iter()
            .filter(|id| !connected_android_set.contains(*id))
            .cloned()
            .collect();

        CategorizedDevices {
            ios: IosCategorized {
                unsaved_connected: group_a,
                saved_connected: group_c,
                saved_disconnected: group_e,
            },
            android: AndroidCategorized {
                unsaved_connected: group_b,
                saved_connected: group_d,
                saved_disconnected: group_f,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_devices() {
        let connected_android = vec!["device1".to_string(), "device2".to_string()];
        let connected_ios = vec!["ios1".to_string()];

        let saved_devices = SavedDevicesInfo {
            android: vec!["device2".to_string(), "device3".to_string()],
            ios: vec!["ios2".to_string()],
        };

        let result = DeviceCategorizer::categorize_devices(
            connected_android,
            connected_ios,
            saved_devices,
        );

        // 组B: device1 (已连接但未保存)
        assert_eq!(result.android.unsaved_connected, vec!["device1"]);
        // 组D: device2 (已保存且已连接)
        assert_eq!(result.android.saved_connected, vec!["device2"]);
        // 组F: device3 (已保存但未连接)
        assert_eq!(result.android.saved_disconnected, vec!["device3"]);

        // 组A: ios1 (已连接但未保存)
        assert_eq!(result.ios.unsaved_connected, vec!["ios1"]);
        // 组C: 空
        assert_eq!(result.ios.saved_connected.len(), 0);
        // 组E: ios2 (已保存但未连接)
        assert_eq!(result.ios.saved_disconnected, vec!["ios2"]);
    }

    #[test]
    fn test_extract_yaml_field() {
        let yaml = r#"
deviceName: Test Device
deviceId: abc123
platform: android
        "#;

        assert_eq!(
            DeviceCategorizer::extract_yaml_field(yaml, "deviceId"),
            Some("abc123".to_string())
        );
        assert_eq!(
            DeviceCategorizer::extract_yaml_field(yaml, "platform"),
            Some("android".to_string())
        );
        assert_eq!(
            DeviceCategorizer::extract_yaml_field(yaml, "notexist"),
            None
        );
    }
}
