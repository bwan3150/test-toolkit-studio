// App 管理模块 - 管理设备上的应用信息

use crate::Result;
use crate::models::AppInfo;
use crate::utils::AdbManager;
use crate::TkeError;
use std::process::Command;

pub struct AppManager {
    device_id: Option<String>,
    adb_manager: AdbManager,
}

impl AppManager {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let adb_manager = AdbManager::new()?;
        Ok(Self {
            device_id,
            adb_manager,
        })
    }

    /// 获取设备上所有第三方应用的列表(包含版本信息)
    pub async fn list_third_party_apps(&self) -> Result<Vec<AppInfo>> {
        // 构建 adb 命令
        let mut cmd = Command::new(self.adb_manager.adb_path());

        // 如果指定了设备ID，添加 -s 参数
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        // 获取所有第三方应用的包名列表
        let output = cmd
            .args(&["shell", "pm", "list", "packages", "-3"])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 adb 命令失败: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("获取应用列表失败: {}", error)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let package_lines: Vec<&str> = stdout
            .lines()
            .filter(|line| line.starts_with("package:"))
            .collect();

        let mut apps = Vec::new();

        for line in package_lines {
            let package_name = line.trim_start_matches("package:").trim();

            // 获取每个应用的详细信息
            if let Ok(app_info) = self.get_app_info(package_name).await {
                apps.push(app_info);
            } else {
                // 如果获取详细信息失败,至少保留包名
                apps.push(AppInfo {
                    package_name: package_name.to_string(),
                    version_name: None,
                    version_code: None,
                    apk_path: None,
                });
            }
        }

        Ok(apps)
    }

    /// 获取单个应用的详细信息
    async fn get_app_info(&self, package_name: &str) -> Result<AppInfo> {
        // 构建 adb 命令
        let mut cmd = Command::new(self.adb_manager.adb_path());

        // 如果指定了设备ID，添加 -s 参数
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        // 使用 dumpsys package 获取应用详细信息
        let output = cmd
            .args(&["shell", "dumpsys", "package", package_name])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 dumpsys 命令失败: {}", e)))?;

        if !output.status.success() {
            return Err(TkeError::AdbError(format!("获取应用信息失败: {}", package_name)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut version_name = None;
        let mut version_code = None;
        let mut apk_path = None;

        for line in stdout.lines() {
            let line = line.trim();

            // 解析 versionName
            if line.contains("versionName=") {
                if let Some(start) = line.find("versionName=") {
                    let version_str = &line[start + 12..];
                    // 提取到空格或行尾
                    let end = version_str.find(|c: char| c.is_whitespace()).unwrap_or(version_str.len());
                    version_name = Some(version_str[..end].to_string());
                }
            }

            // 解析 versionCode
            if line.contains("versionCode=") {
                if let Some(start) = line.find("versionCode=") {
                    let version_str = &line[start + 12..];
                    // 提取数字部分
                    let end = version_str.find(|c: char| !c.is_numeric()).unwrap_or(version_str.len());
                    if let Ok(code) = version_str[..end].parse::<i64>() {
                        version_code = Some(code);
                    }
                }
            }

            // 解析 APK 路径 (codePath 字段)
            if line.starts_with("codePath=") {
                apk_path = Some(line.trim_start_matches("codePath=").to_string());
            }
        }

        Ok(AppInfo {
            package_name: package_name.to_string(),
            version_name,
            version_code,
            apk_path,
        })
    }

    /// 强制卸载应用
    /// 参数:
    /// - package_name: 应用包名
    /// 返回: (success, message)
    pub async fn uninstall_app(&self, package_name: &str) -> Result<(bool, String)> {
        // 构建 adb 命令
        let mut cmd = Command::new(self.adb_manager.adb_path());

        // 如果指定了设备ID，添加 -s 参数
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        // 使用 pm uninstall 卸载应用
        // 注意: 不使用 -k 参数(保留数据), 实现完全卸载
        let output = cmd
            .args(&["shell", "pm", "uninstall", package_name])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行卸载命令失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        // 检查卸载结果
        // 成功时输出: "Success"
        // 失败时输出错误信息，如: "Failure [DELETE_FAILED_INTERNAL_ERROR]"
        if stdout.contains("Success") {
            Ok((true, format!("应用 {} 卸载成功", package_name)))
        } else if !stderr.is_empty() {
            Ok((false, format!("卸载失败: {}", stderr)))
        } else if stdout.contains("Failure") {
            Ok((false, format!("卸载失败: {}", stdout)))
        } else {
            // 未知情况
            Ok((false, format!("卸载结果未知: stdout={}, stderr={}", stdout, stderr)))
        }
    }
}
