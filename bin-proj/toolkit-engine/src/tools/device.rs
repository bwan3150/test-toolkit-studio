// Device模块 - 负责获取设备详细信息

use crate::{Result, TkeError, DeviceInfo, HardwareInfo, BatteryInfo, NetworkInfo, ToolManager};
use std::process::Command;

pub struct DeviceManager {
    device_id: Option<String>,
    adb_path: std::path::PathBuf,
}

impl DeviceManager {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        // adb 由 ToolManager 统一在 tke 同目录定位
        let adb_path = ToolManager::resolve("adb")?;
        Ok(Self {
            device_id,
            adb_path,
        })
    }

    /// 获取完整的设备信息（包括硬件、电池、网络）
    /// 没指定 `-d` 时 adb 连的那台的序列号。查不到就留空——
    /// 这是补充信息，查询失败不该让整条命令失败
    fn only_serial(&self) -> String {
        std::process::Command::new(&self.adb_path)
            .args(["devices"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .and_then(|out| {
                let mut it = out
                    .lines()
                    .skip(1)
                    .filter_map(|l| {
                        let mut p = l.split_whitespace();
                        match (p.next(), p.next()) {
                            (Some(sn), Some("device")) => Some(sn.to_string()),
                            _ => None,
                        }
                    });
                it.next()
            })
            .unwrap_or_default()
    }

    pub fn get_full_device_info(&self) -> Result<DeviceInfo> {
        let model = self.get_device_prop("ro.product.model")?;
        let manufacturer = self.get_device_prop("ro.product.manufacturer")?;
        let android_version = self.get_device_prop("ro.build.version.release")?;

        // 获取屏幕尺寸
        let size_output = self.run_adb_command_output(&["shell", "wm", "size"])?;
        let (width, height) = self.parse_screen_size(&size_output);

        // 获取硬件信息
        let hardware = Some(self.get_hardware_info()?);

        // 获取电池信息
        let battery = Some(self.get_battery_info()?);

        // 获取网络信息
        let network = Some(self.get_network_info()?);

        Ok(DeviceInfo {
            // 不带 `-d` 时也要说清**到底连的哪台**：adb 会自己挑唯一那台，
            // 但结果里 id 空着，人根本不知道刚才看的是谁的信息（多设备时更要命）
            id: self.device_id.clone().unwrap_or_else(|| self.only_serial()),
            model: Some(model),
            manufacturer: Some(manufacturer),
            android_version: Some(android_version),
            screen_width: width,
            screen_height: height,
            hardware,
            battery,
            network,
        })
    }

    /// 获取设备的单个 prop 属性值（支持别名）
    pub fn get_device_prop(&self, prop: &str) -> Result<String> {
        // 将别名映射到实际的 prop 名称
        let actual_prop = self.resolve_prop_alias(prop);
        let output = self.run_adb_command_output(&["shell", "getprop", actual_prop])?;
        Ok(output.trim().to_string())
    }

    /// 将简短别名解析为实际的 prop 名称
    fn resolve_prop_alias<'a>(&self, alias: &'a str) -> &'a str {
        match alias {
            // 基础信息
            "version" | "android_version" => "ro.build.version.release",
            "sdk" | "sdk_version" => "ro.build.version.sdk",
            "model" => "ro.product.model",
            "manufacturer" => "ro.product.manufacturer",
            "brand" => "ro.product.brand",
            "device" => "ro.product.device",
            "board" => "ro.product.board",
            "serial" | "serialno" => "ro.serialno",

            // 硬件信息
            "cpu_abi" | "abi" => "ro.product.cpu.abi",
            "hardware" => "ro.hardware",

            // 构建信息
            "build_id" => "ro.build.id",
            "build_type" => "ro.build.type",
            "build_tags" => "ro.build.tags",
            "build_fingerprint" | "fingerprint" => "ro.build.fingerprint",
            "build_date" => "ro.build.date",

            // 网络信息
            "operator" | "carrier" => "gsm.operator.alpha",
            "operator_code" => "gsm.operator.numeric",
            "country" | "country_code" => "gsm.operator.iso-country",
            "network_type" => "gsm.network.type",

            // 如果不是别名，直接返回原始字符串（可能是完整的 prop 名称）
            _ => alias,
        }
    }

    /// 获取硬件信息
    fn get_hardware_info(&self) -> Result<HardwareInfo> {
        // CPU 架构
        let cpu_abi = self.get_device_prop("ro.product.cpu.abi").ok();

        // CPU 型号（从 /proc/cpuinfo）
        let cpu_model = self.get_cpu_model().ok();

        // CPU 核心数
        let cpu_cores = self.get_cpu_cores().ok();

        // 内存信息
        let (total_memory_mb, available_memory_mb) = self.get_memory_info()?;

        // 存储信息
        let (total_storage_gb, available_storage_gb) = self.get_storage_info()?;

        Ok(HardwareInfo {
            cpu_model,
            cpu_cores,
            cpu_abi,
            total_memory_mb: Some(total_memory_mb),
            available_memory_mb: Some(available_memory_mb),
            total_storage_gb: Some(total_storage_gb),
            available_storage_gb: Some(available_storage_gb),
        })
    }

    /// 获取电池信息
    fn get_battery_info(&self) -> Result<BatteryInfo> {
        // 使用 dumpsys battery 获取电池信息
        let battery_output = self.run_adb_command_output(&["shell", "dumpsys", "battery"])?;

        let mut level = None;
        let mut temperature = None;
        let mut health = None;
        let mut status = None;
        let mut is_charging = None;
        let mut power_source = None;

        for line in battery_output.lines() {
            let line = line.trim();

            // 电量百分比
            if line.starts_with("level:") {
                if let Some(value) = line.split(':').nth(1) {
                    level = value.trim().parse().ok();
                }
            }
            // 温度（需要除以10得到摄氏度）
            else if line.starts_with("temperature:") {
                if let Some(value) = line.split(':').nth(1) {
                    if let Ok(temp_raw) = value.trim().parse::<i32>() {
                        temperature = Some(temp_raw as f32 / 10.0);
                    }
                }
            }
            // 健康状态
            else if line.starts_with("health:") {
                if let Some(value) = line.split(':').nth(1) {
                    health = Some(self.parse_battery_health(value.trim()));
                }
            }
            // 充电状态
            else if line.starts_with("status:") {
                if let Some(value) = line.split(':').nth(1) {
                    let status_str = self.parse_battery_status(value.trim());
                    is_charging = Some(status_str.contains("Charging"));
                    status = Some(status_str);
                }
            }
            // 电源类型
            else if line.starts_with("AC powered:") || line.starts_with("USB powered:") || line.starts_with("Wireless powered:") {
                if line.contains("true") {
                    if line.starts_with("AC") {
                        power_source = Some("AC".to_string());
                    } else if line.starts_with("USB") {
                        power_source = Some("USB".to_string());
                    } else if line.starts_with("Wireless") {
                        power_source = Some("Wireless".to_string());
                    }
                }
            }
        }

        // 如果没有检测到电源类型但正在充电，设置为 Unknown
        if is_charging == Some(true) && power_source.is_none() {
            power_source = Some("Unknown".to_string());
        } else if is_charging == Some(false) {
            power_source = Some("None".to_string());
        }

        Ok(BatteryInfo {
            level,
            temperature,
            health,
            status,
            is_charging,
            power_source,
        })
    }

    /// 获取网络信息
    fn get_network_info(&self) -> Result<NetworkInfo> {
        // WiFi 状态
        let wifi_enabled = self.is_wifi_enabled().ok();

        // WiFi SSID
        let wifi_ssid = self.get_wifi_ssid().ok();

        // 移动网络类型
        let mobile_network_type = self.get_device_prop("gsm.network.type").ok();

        // 运营商名称
        let operator_name = self.get_device_prop("gsm.operator.alpha").ok()
            .filter(|s| !s.is_empty());

        // 运营商代码
        let operator_code = self.get_device_prop("gsm.operator.numeric").ok()
            .filter(|s| !s.is_empty());

        // 国家代码
        let country_iso = self.get_device_prop("gsm.operator.iso-country").ok()
            .filter(|s| !s.is_empty());

        Ok(NetworkInfo {
            wifi_enabled,
            wifi_ssid,
            mobile_network_type,
            operator_name,
            operator_code,
            country_iso,
        })
    }

    // ========== 辅助方法 ==========

    /// 获取 CPU 型号
    fn get_cpu_model(&self) -> Result<String> {
        let output = self.run_adb_command_output(&["shell", "cat", "/proc/cpuinfo"])?;

        // 查找 Hardware 或 model name
        for line in output.lines() {
            if line.starts_with("Hardware") || line.starts_with("model name") {
                if let Some(value) = line.split(':').nth(1) {
                    return Ok(value.trim().to_string());
                }
            }
        }

        Err(TkeError::DeviceError("无法获取 CPU 型号".to_string()))
    }

    /// 获取 CPU 核心数
    fn get_cpu_cores(&self) -> Result<u32> {
        let output = self.run_adb_command_output(&["shell", "cat", "/proc/cpuinfo"])?;

        let processor_count = output.lines()
            .filter(|line| line.starts_with("processor"))
            .count();

        if processor_count > 0 {
            Ok(processor_count as u32)
        } else {
            Err(TkeError::DeviceError("无法获取 CPU 核心数".to_string()))
        }
    }

    /// 获取内存信息（返回总内存和可用内存，单位 MB）
    fn get_memory_info(&self) -> Result<(u64, u64)> {
        let output = self.run_adb_command_output(&["shell", "cat", "/proc/meminfo"])?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;

        for line in output.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    total_kb = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    available_kb = value.parse().unwrap_or(0);
                }
            }
        }

        Ok((total_kb / 1024, available_kb / 1024))
    }

    /// 获取存储信息（返回总存储和可用存储，单位 GB）
    fn get_storage_info(&self) -> Result<(f64, f64)> {
        // 使用 df /data 获取存储信息
        let output = self.run_adb_command_output(&["shell", "df", "/data"])?;

        // 输出格式示例:
        // Filesystem           1K-blocks      Used Available Use% Mounted on
        // /dev/block/dm-2       57671680  48234560   9437120  84% /data

        if let Some(data_line) = output.lines().nth(1) {
            let parts: Vec<&str> = data_line.split_whitespace().collect();
            if parts.len() >= 4 {
                let total_kb: u64 = parts[1].parse().unwrap_or(0);
                let available_kb: u64 = parts[3].parse().unwrap_or(0);

                // 转换为 GB
                let total_gb = total_kb as f64 / 1024.0 / 1024.0;
                let available_gb = available_kb as f64 / 1024.0 / 1024.0;

                return Ok((total_gb, available_gb));
            }
        }

        Err(TkeError::DeviceError("无法获取存储信息".to_string()))
    }

    /// 检查 WiFi 是否启用
    fn is_wifi_enabled(&self) -> Result<bool> {
        let output = self.run_adb_command_output(&["shell", "settings", "get", "global", "wifi_on"])?;
        Ok(output.trim() == "1")
    }

    /// 获取 WiFi SSID
    fn get_wifi_ssid(&self) -> Result<String> {
        let output = self.run_adb_command_output(&["shell", "dumpsys", "wifi"])?;

        // 查找当前连接的 SSID
        for line in output.lines() {
            if line.contains("mWifiInfo") && line.contains("SSID:") {
                if let Some(ssid_start) = line.find("SSID: ") {
                    let ssid_part = &line[ssid_start + 6..];
                    if let Some(comma_pos) = ssid_part.find(',') {
                        let ssid = ssid_part[..comma_pos].trim().to_string();
                        // 移除引号
                        let ssid = ssid.trim_matches('"').to_string();
                        if !ssid.is_empty() && ssid != "<unknown ssid>" {
                            return Ok(ssid);
                        }
                    }
                }
            }
        }

        Err(TkeError::DeviceError("未连接到 WiFi".to_string()))
    }

    /// 解析电池健康状态码
    fn parse_battery_health(&self, health_code: &str) -> String {
        match health_code {
            "1" => "Unknown",
            "2" => "Good",
            "3" => "Overheat",
            "4" => "Dead",
            "5" => "Over voltage",
            "6" => "Unspecified failure",
            "7" => "Cold",
            _ => "Unknown",
        }.to_string()
    }

    /// 解析电池充电状态码
    fn parse_battery_status(&self, status_code: &str) -> String {
        match status_code {
            "1" => "Unknown",
            "2" => "Charging",
            "3" => "Discharging",
            "4" => "Not charging",
            "5" => "Full",
            _ => "Unknown",
        }.to_string()
    }

    /// 解析屏幕尺寸
    fn parse_screen_size(&self, output: &str) -> (u32, u32) {
        // 输出格式: "Physical size: 1080x1920"
        if let Some(pos) = output.find(": ") {
            let size_str = &output[pos + 2..];
            if let Some(x_pos) = size_str.find('x') {
                let width = size_str[..x_pos].trim().parse().unwrap_or(1080);
                let height = size_str[x_pos + 1..].trim().parse().unwrap_or(1920);
                return (width, height);
            }
        }
        (1080, 1920)  // 默认值
    }

    /// 执行ADB命令并获取输出
    fn run_adb_command_output(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.adb_path);

        // 如果指定了设备ID，添加-s参数
        if let Some(ref device_id) = self.device_id {
            cmd.arg("-s").arg(device_id);
        }

        cmd.args(args);

        let output = cmd.output()
            .map_err(|e| TkeError::AdbError(format!("执行ADB命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("ADB命令执行失败: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
