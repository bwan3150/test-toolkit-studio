// App 管理模块 - 管理设备上的应用信息

use crate::Result;
use crate::models::{AppInfo, CurrentFocus};
use crate::ToolManager;
use crate::TkeError;
use std::process::Command;

pub struct AppManager {
    device_id: Option<String>,
    adb_path: std::path::PathBuf,
}

impl AppManager {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let adb_path = ToolManager::resolve("adb")?;
        Ok(Self {
            device_id,
            adb_path,
        })
    }

    /// 获取设备上所有第三方应用的列表(包含版本信息)
    pub async fn list_third_party_apps(&self) -> Result<Vec<AppInfo>> {
        // 构建 adb 命令
        let mut cmd = Command::new(&self.adb_path);

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
                    launch_activity: None,
                });
            }
        }

        Ok(apps)
    }

    /// 获取单个应用的详细信息
    async fn get_app_info(&self, package_name: &str) -> Result<AppInfo> {
        // 构建 adb 命令
        let mut cmd = Command::new(&self.adb_path);

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
        let mut launch_activity = None;

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

            // 解析启动 Activity (查找 MAIN Activity)
            // 格式示例: 1234567 com.example.app/.MainActivity filter abcdef
            if line.contains(package_name) && line.contains("/") && launch_activity.is_none() {
                if let Some(slash_pos) = line.find('/') {
                    let after_slash = &line[slash_pos + 1..];
                    // 提取到空格或行尾
                    let end = after_slash.find(|c: char| c.is_whitespace())
                        .unwrap_or(after_slash.len());
                    let activity = after_slash[..end].trim();

                    if !activity.is_empty() {
                        launch_activity = Some(activity.to_string());
                    }
                }
            }
        }

        Ok(AppInfo {
            package_name: package_name.to_string(),
            version_name,
            version_code,
            apk_path,
            launch_activity,
        })
    }

    /// 强制卸载应用
    /// 参数:
    /// - package_name: 应用包名
    /// 返回: (success, message)
    pub async fn uninstall_app(&self, package_name: &str) -> Result<(bool, String)> {
        // 构建 adb 命令
        let mut cmd = Command::new(&self.adb_path);

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

    /// 获取当前聚焦的应用信息
    /// 返回当前显示在前台的应用的包名和 Activity 名称
    pub async fn get_current_focus(&self) -> Result<CurrentFocus> {
        // 构建 adb 命令
        let mut cmd = Command::new(&self.adb_path);

        // 如果指定了设备ID，添加 -s 参数
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        // 执行 dumpsys window 并查找 mCurrentFocus
        let output = cmd
            .args(&["shell", "dumpsys", "window"])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 dumpsys window 命令失败: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("获取窗口信息失败: {}", error)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 查找 mCurrentFocus 行
        // 格式示例: mCurrentFocus=Window{1bc3577 u0 com.konec.smarthome/com.konec.smarthome.test.ui.activity.MainActivity}
        for line in stdout.lines() {
            if line.contains("mCurrentFocus") {
                let trimmed = line.trim();

                // 提取包名和 Activity
                // 查找 u0 后面的部分
                if let Some(start) = trimmed.find("u0 ") {
                    let info_part = &trimmed[start + 3..]; // 跳过 "u0 "

                    // 去掉末尾的 }
                    let info_part = info_part.trim_end_matches('}').trim();

                    // 分割包名和 Activity (格式: package/activity)
                    if let Some(slash_pos) = info_part.find('/') {
                        let package_name = info_part[..slash_pos].to_string();
                        let activity_name = info_part[slash_pos + 1..].to_string();

                        return Ok(CurrentFocus {
                            package_name,
                            activity_name,
                            window_info: Some(trimmed.to_string()),
                        });
                    }
                }
            }
        }

        Err(TkeError::AdbError("未找到当前聚焦的窗口信息".to_string()))
    }

    /// 启动应用
    /// 使用 am start 命令启动应用
    pub async fn launch_app(&self, package_name: &str, activity_name: &str) -> Result<(bool, String)> {
        let mut cmd = Command::new(&self.adb_path);

        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        // 构建完整的组件名称: package_name/activity_name
        let component = format!("{}/{}", package_name, activity_name);

        let output = cmd
            .args(&["shell", "am", "start", "-n", &component])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 am start 命令失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // am start 成功时会输出 "Starting: Intent"
        if stdout.contains("Starting: Intent") || output.status.success() {
            Ok((true, format!("应用 {} 启动成功", package_name)))
        } else {
            Ok((false, format!("启动失败: {}", stdout)))
        }
    }

    /// 关闭应用
    /// 使用 am force-stop 命令强制停止应用
    pub async fn stop_app(&self, package_name: &str) -> Result<(bool, String)> {
        let mut cmd = Command::new(&self.adb_path);

        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        let output = cmd
            .args(&["shell", "am", "force-stop", package_name])
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 am force-stop 命令失败: {}", e)))?;

        // am force-stop 成功时不会有输出,只需检查退出码
        if output.status.success() {
            Ok((true, format!("应用 {} 已关闭", package_name)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok((false, format!("关闭失败: {}", stderr)))
        }
    }

    /// 取设备日志（logcat）——**App 崩了、点了没反应，真因往往只在这里**。
    ///
    /// 这是 tke 唯一必须补的直通替代品：删掉 CLI 直通之后，没有它就没法看崩溃堆栈。
    ///
    /// - `package`：只看这个包的日志（按 PID 过滤，比 grep 包名准——堆栈行里不带包名）
    /// - `lines`：取最后多少行
    /// - `level`：最低级别（V/D/I/W/E），默认 W——默认拉全量会把 AI 的上下文冲爆
    ///
    /// 一律**取一次就返回**（`-d` 模式），不做 follow：一条 CLI 命令挂在那儿等日志，
    /// 调用方只能超时杀掉，拿不到东西。要盯变化就前后各取一次。
    pub async fn logcat(
        &self,
        package: Option<&str>,
        lines: usize,
        level: &str,
    ) -> Result<String> {
        // 按包名过滤要先拿 PID：崩溃堆栈那几行里**不含包名**，直接 grep 包名会把最有用的
        // 那段滤掉。拿不到 PID（进程没起来/已崩溃退出）就退回全量——有总比没有强
        let pid = match package {
            Some(pkg) => self.pid_of(pkg).await,
            None => None,
        };

        let mut cmd = Command::new(&self.adb_path);
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }
        cmd.args(["logcat", "-d", "-v", "time", "-t", &lines.to_string()]);
        if let Some(p) = pid {
            cmd.args(["--pid", &p.to_string()]);
        }
        // `*:W` = 所有 tag 至少 W 级
        cmd.arg(format!("*:{}", level.trim().to_uppercase()));

        let output = cmd
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行 logcat 失败: {}", e)))?;
        if !output.status.success() {
            return Err(TkeError::AdbError(format!(
                "logcat 失败: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 包名 → 当前 PID（进程没在跑就是 None）
    async fn pid_of(&self, package: &str) -> Option<u32> {
        let mut cmd = Command::new(&self.adb_path);
        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }
        let out = cmd.args(["shell", "pidof", package]).output().ok()?;
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }
}
