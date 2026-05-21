// scrcpy-server 管理模块
// 负责在手机上启动 scrcpy-server.jar

use anyhow::{Result, Context, anyhow};
use tokio::process::Command;
use std::process::Stdio;
use std::path::PathBuf;
use tracing::{debug, info, warn};

// 常量定义（对应 ws-scrcpy 的 Constants.ts）
const SERVER_PACKAGE: &str = "com.genymobile.scrcpy.Server";
const SERVER_PORT: u16 = 8886;
const SERVER_VERSION: &str = "1.19-ws6";
const SERVER_TYPE: &str = "web";
const LOG_LEVEL: &str = "ERROR";
const SCRCPY_LISTENS_ON_ALL_INTERFACES: bool = true; // ws-scrcpy 使用 true
const SERVER_PROCESS_NAME: &str = "app_process";

const TEMP_PATH: &str = "/data/local/tmp/";
const FILE_NAME: &str = "scrcpy-server.jar";

/// scrcpy-server 管理器
pub struct ScrcpyServer {
    adb_path: String,
}

impl ScrcpyServer {
    /// 创建新的 ScrcpyServer 实例
    pub fn new(adb_path: String) -> Self {
        Self { adb_path }
    }

    /// 检查是否使用 tke
    fn is_using_tke(&self) -> bool {
        self.adb_path.contains("tke")
    }

    /// 构建 adb 命令参数
    /// 如果使用 tke，需要在前面加上 "adb" 子命令
    fn build_adb_args(&self, args: &[&str]) -> Vec<String> {
        if self.is_using_tke() {
            let mut result = vec!["adb".to_string()];
            result.extend(args.iter().map(|s| s.to_string()));
            result
        } else {
            args.iter().map(|s| s.to_string()).collect()
        }
    }

    /// 获取 scrcpy-server.jar 的路径
    fn get_server_jar_path() -> Result<PathBuf> {
        // 获取可执行文件所在目录
        let exe_path = std::env::current_exe()
            .context("无法获取可执行文件路径")?;
        info!("📁 可执行文件路径: {}", exe_path.display());

        let exe_dir = exe_path.parent()
            .ok_or_else(|| anyhow!("无法获取可执行文件目录"))?;
        info!("📁 可执行文件目录: {}", exe_dir.display());

        // scrcpy-server.jar 直接放在可执行文件旁边
        let jar_path = exe_dir.join("scrcpy-server.jar");
        info!("🔍 查找 jar 文件: {}", jar_path.display());

        if !jar_path.exists() {
            // 列出目录下的所有文件，帮助调试
            if let Ok(entries) = std::fs::read_dir(exe_dir) {
                info!("📂 可执行文件目录下的文件:");
                for entry in entries.flatten() {
                    info!("  - {}", entry.path().display());
                }
            }

            return Err(anyhow!(
                "scrcpy-server.jar 不存在: {}，请确保将 scrcpy-server.jar 放在可执行文件旁边",
                jar_path.display()
            ));
        }

        info!("✅ 找到 jar 文件: {}", jar_path.display());
        Ok(jar_path)
    }

    /// 检查 scrcpy-server 是否已在手机上运行
    async fn is_server_running(&self, udid: &str) -> Result<bool> {
        debug!("检查 scrcpy-server 是否在设备 {} 上运行", udid);

        // 使用更可靠的方法: 检查 /proc/*/cmdline
        // 先获取所有 app_process 进程
        let args = self.build_adb_args(&["-s", udid, "shell", "ps | grep app_process"]);
        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 ps 命令失败")?;

        // grep 没找到匹配时会返回非 0 状态码,这是正常的
        if !output.status.success() {
            debug!("没有找到 app_process 进程");
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析 ps 输出,提取 PID
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let pid = parts[1];

                // 检查 cmdline 是否包含 scrcpy-server 的特征
                let cmd = format!("cat /proc/{}/cmdline", pid);
                let args = self.build_adb_args(&["-s", udid, "shell", &cmd]);
                let cmdline_output = Command::new(&self.adb_path)
                    .args(&args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                if let Ok(output) = cmdline_output {
                    let cmdline = String::from_utf8_lossy(&output.stdout);
                    // 检查是否是 scrcpy.Server (不是 CleanUp)
                    if cmdline.contains(SERVER_PACKAGE) &&
                       cmdline.contains("Server") &&
                       !cmdline.contains("CleanUp") {
                        info!("发现已运行的 scrcpy-server (PID: {})", pid);
                        return Ok(true);
                    }
                }
            }
        }

        debug!("没有找到运行中的 scrcpy-server");
        Ok(false)
    }

    /// 将 scrcpy-server.jar 推送到手机
    async fn push_server(&self, udid: &str) -> Result<()> {
        info!("开始获取 jar 文件路径...");
        let jar_path = Self::get_server_jar_path()?;
        let remote_path = format!("{}{}", TEMP_PATH, FILE_NAME);

        info!("📤 推送 scrcpy-server.jar 到设备");
        info!("  本地路径: {}", jar_path.display());
        info!("  远程路径: {}", remote_path);
        info!("  设备 UDID: {}", udid);
        info!("  ADB 路径: {}", self.adb_path);

        let args = self.build_adb_args(&[
            "-s", udid,
            "push",
            jar_path.to_str().unwrap(),
            &remote_path
        ]);

        info!("📝 执行命令: {} {:?}", self.adb_path, args);

        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 adb push 失败")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        info!("📄 命令输出 stdout: {}", stdout.trim());
        if !stderr.is_empty() {
            info!("📄 命令输出 stderr: {}", stderr.trim());
        }

        if !output.status.success() {
            return Err(anyhow!("推送 scrcpy-server.jar 失败: {}", stderr));
        }

        info!("✅ scrcpy-server.jar 推送成功");
        Ok(())
    }

    /// 在手机上启动 scrcpy-server
    async fn start_server(&self, udid: &str) -> Result<()> {
        // 构建启动命令
        // CLASSPATH=/data/local/tmp/scrcpy-server.jar nohup app_process / com.genymobile.scrcpy.Server 1.19-ws6 web ERROR 8886 false 2>&1 > /dev/null
        let args = format!(
            "{} {} {} {} {}",
            SERVER_VERSION,
            SERVER_TYPE,
            LOG_LEVEL,
            SERVER_PORT,
            SCRCPY_LISTENS_ON_ALL_INTERFACES
        );

        //  使用 & 后台运行，不重定向输出
        // 注意：重定向到 /dev/null 可能导致 adb shell 挂起
        let run_command = format!(
            "CLASSPATH={}{} app_process / {} {} </dev/null >/dev/null 2>&1 &",
            TEMP_PATH,
            FILE_NAME,
            SERVER_PACKAGE,
            args
        );

        info!("🚀 启动 scrcpy-server");
        info!("  命令: {}", run_command);
        info!("  设备 UDID: {}", udid);

        let args = self.build_adb_args(&["-s", udid, "shell", &run_command]);
        info!("📝 执行命令: {} {:?}", self.adb_path, args);

        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行启动命令失败")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        info!("📄 启动命令输出 stdout: {}", stdout.trim());
        if !stderr.is_empty() {
            info!("📄 启动命令输出 stderr: {}", stderr.trim());
        }

        // 注意：由于使用了 & 后台运行，命令会立即返回
        // 检查输出以便调试
        if !output.status.success() {
            info!("⚠️ 启动命令返回非零状态（可能正常）");
        }

        info!("✅ scrcpy-server 启动命令已执行");
        Ok(())
    }

    /// 等待 scrcpy-server 启动完成
    async fn wait_for_server(&self, udid: &str) -> Result<()> {
        const MAX_RETRIES: u32 = 10;
        const RETRY_DELAY_MS: u64 = 500;

        info!("等待 scrcpy-server 启动...");

        for i in 0..MAX_RETRIES {
            tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;

            match self.is_server_running(udid).await {
                Ok(true) => {
                    info!("✅ scrcpy-server 已启动（第 {} 次检查）", i + 1);
                    return Ok(());
                }
                Ok(false) => {
                    info!("等待 scrcpy-server 启动... ({}/{})", i + 1, MAX_RETRIES);
                }
                Err(e) => {
                    info!("检查 scrcpy-server 状态时出错: {}", e);
                }
            }
        }

        Err(anyhow!("scrcpy-server 启动超时（等待 {} 次，每次 {}ms）", MAX_RETRIES, RETRY_DELAY_MS))
    }

    /// 确保 scrcpy-server 在设备上运行
    /// 如果已经运行则直接返回，否则推送并启动
    pub async fn ensure_server_running(&self, udid: &str) -> Result<()> {
        info!("确保 scrcpy-server 在设备 {} 上运行", udid);

        // 1. 检查是否已经运行
        if self.is_server_running(udid).await? {
            info!("scrcpy-server 已在运行");
            return Ok(());
        }

        // 2. 推送 jar 包
        self.push_server(udid).await?;

        // 3. 启动 server
        self.start_server(udid).await?;

        // 4. 等待启动完成
        self.wait_for_server(udid).await?;

        Ok(())
    }

    /// 停止设备上的 scrcpy-server
    pub async fn stop_server(&self, udid: &str) -> Result<()> {
        info!("停止设备 {} 上的 scrcpy-server", udid);

        // 获取所有 app_process 进程
        let args = self.build_adb_args(&["-s", udid, "shell", "ps | grep app_process"]);
        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 ps 命令失败")?;

        // grep 没找到匹配时会返回非 0 状态码,这是正常的
        if !output.status.success() {
            debug!("没有找到 app_process 进程");
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析 ps 输出,提取 PID 并检查 cmdline
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let pid = parts[1];

                // 检查 cmdline 是否包含 scrcpy
                let cmd = format!("cat /proc/{}/cmdline", pid);
                let args = self.build_adb_args(&["-s", udid, "shell", &cmd]);
                let cmdline_output = Command::new(&self.adb_path)
                    .args(&args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                if let Ok(output) = cmdline_output {
                    let cmdline = String::from_utf8_lossy(&output.stdout);
                    // 只杀死 scrcpy.Server,不杀 CleanUp
                    if cmdline.contains(SERVER_PACKAGE) && cmdline.contains("Server") {
                        info!("杀死 scrcpy-server 进程 (PID: {})", pid);
                        let args = self.build_adb_args(&["-s", udid, "shell", "kill", pid]);
                        let _ = Command::new(&self.adb_path)
                            .args(&args)
                            .output()
                            .await;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_server_jar_path() {
        // 这个测试只检查路径构建逻辑，不检查文件是否存在
        // 因为测试环境中可能没有 jar 文件
        let result = ScrcpyServer::get_server_jar_path();
        match result {
            Ok(path) => {
                println!("jar 路径: {}", path.display());
                assert!(path.to_string_lossy().contains("scrcpy-server.jar"));
            }
            Err(e) => {
                println!("预期的错误（jar 不存在）: {}", e);
            }
        }
    }
}
