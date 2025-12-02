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
use rusqlite::Connection;

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
    /// 从数据库加载项目中保存的设备配置
    pub fn load_saved_devices(project_path: &PathBuf) -> Result<SavedDevicesInfo> {
        let db_path = project_path.join("project.db");
        let mut android_saved = Vec::new();
        let mut ios_saved = Vec::new();

        // 如果数据库不存在，返回空
        if !db_path.exists() {
            return Ok(SavedDevicesInfo::default());
        }

        // 打开数据库连接
        let conn = Connection::open(&db_path)
            .map_err(|e| TkeError::DatabaseError(format!("无法打开数据库: {}", e)))?;

        // 检查 devices 表是否存在
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='devices'",
                [],
                |row| row.get(0),
            )
            .map(|count: i32| count > 0)
            .unwrap_or(false);

        if !table_exists {
            return Ok(SavedDevicesInfo::default());
        }

        // 查询所有设备
        let mut stmt = conn
            .prepare("SELECT platform, device_id FROM devices")
            .map_err(|e| TkeError::DatabaseError(format!("SQL准备失败: {}", e)))?;

        let device_iter = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| TkeError::DatabaseError(format!("查询失败: {}", e)))?;

        for device_result in device_iter {
            if let Ok((platform, device_id)) = device_result {
                if let Some(id) = device_id {
                    if !id.is_empty() {
                        match platform.to_lowercase().as_str() {
                            "android" => android_saved.push(id),
                            "ios" => ios_saved.push(id),
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(SavedDevicesInfo {
            android: android_saved,
            ios: ios_saved,
        })
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

}
