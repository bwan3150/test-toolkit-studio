// 设备信息相关数据结构

use serde::{Deserialize, Serialize};

/// 硬件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU 型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_model: Option<String>,
    /// CPU 核心数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    /// CPU 架构 (如 arm64-v8a)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_abi: Option<String>,
    /// 总内存 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_memory_mb: Option<u64>,
    /// 可用内存 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_memory_mb: Option<u64>,
    /// 总存储空间 (GB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_storage_gb: Option<f64>,
    /// 可用存储空间 (GB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_storage_gb: Option<f64>,
}

/// 电池信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// 电池电量 (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// 电池温度 (摄氏度)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 电池健康状态 (Good, Overheat, Dead, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// 充电状态 (Charging, Discharging, Not charging, Full, Unknown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// 是否正在充电
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_charging: Option<bool>,
    /// 电源类型 (AC, USB, Wireless, None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_source: Option<String>,
}

/// 网络信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// WiFi 连接状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_enabled: Option<bool>,
    /// WiFi SSID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_ssid: Option<String>,
    /// 移动网络类型 (LTE, 5G, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile_network_type: Option<String>,
    /// 运营商名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_name: Option<String>,
    /// 运营商代码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_code: Option<String>,
    /// 国家代码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_iso: Option<String>,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    pub android_version: Option<String>,
    pub screen_width: u32,
    pub screen_height: u32,
    /// 硬件信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareInfo>,
    /// 电池信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<BatteryInfo>,
    /// 网络信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
}
