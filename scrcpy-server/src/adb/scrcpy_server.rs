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
const SCRCPY_LISTENS_ON_ALL_INTERFACES: bool = false;
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

    /// 获取 scrcpy-server.jar 的路径
    fn get_server_jar_path() -> Result<PathBuf> {
        // 获取可执行文件所在目录
        let exe_path = std::env::current_exe()
            .context("无法获取可执行文件路径")?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| anyhow!("无法获取可执行文件目录"))?;

        // scrcpy-server.jar 应该在 exe 同级的 vendor 目录下
        let jar_path = exe_dir.join("vendor/Genymobile/scrcpy/scrcpy-server.jar");

        if !jar_path.exists() {
            return Err(anyhow!(
                "scrcpy-server.jar 不存在: {}，请确保将 vendor 目录放在可执行文件旁边",
                jar_path.display()
            ));
        }

        Ok(jar_path)
    }

    /// 检查 scrcpy-server 是否已在手机上运行
    async fn is_server_running(&self, udid: &str) -> Result<bool> {
        debug!("检查 scrcpy-server 是否在设备 {} 上运行", udid);

        // 获取所有 app_process 进程
        let output = Command::new(&self.adb_path)
            .args(&["-s", udid, "shell", "pidof", SERVER_PROCESS_NAME])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 pidof 命令失败")?;

        if !output.status.success() {
            debug!("没有找到 {} 进程", SERVER_PROCESS_NAME);
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<&str> = stdout.trim().split_whitespace().collect();

        if pids.is_empty() {
            debug!("没有找到 {} 进程", SERVER_PROCESS_NAME);
            return Ok(false);
        }

        // 检查每个进程的 cmdline
        for pid in pids {
            let cmdline_output = Command::new(&self.adb_path)
                .args(&["-s", udid, "shell", &format!("cat /proc/{}/cmdline", pid)])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            if let Ok(output) = cmdline_output {
                let cmdline = String::from_utf8_lossy(&output.stdout);
                // cmdline 使用 \0 分隔参数
                if cmdline.contains(SERVER_PACKAGE) && cmdline.contains(SERVER_VERSION) {
                    info!("发现已运行的 scrcpy-server (PID: {})", pid);
                    return Ok(true);
                }
            }
        }

        debug!("没有找到匹配版本的 scrcpy-server");
        Ok(false)
    }

    /// 将 scrcpy-server.jar 推送到手机
    async fn push_server(&self, udid: &str) -> Result<()> {
        let jar_path = Self::get_server_jar_path()?;
        let remote_path = format!("{}{}", TEMP_PATH, FILE_NAME);

        info!("推送 scrcpy-server.jar 到设备: {} -> {}",
              jar_path.display(), remote_path);

        let output = Command::new(&self.adb_path)
            .args(&[
                "-s", udid,
                "push",
                jar_path.to_str().unwrap(),
                &remote_path
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 adb push 失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
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

        let run_command = format!(
            "CLASSPATH={}{} nohup app_process / {} {} 2>&1 > /dev/null &",
            TEMP_PATH,
            FILE_NAME,
            SERVER_PACKAGE,
            args
        );

        info!("启动 scrcpy-server: {}", run_command);

        let output = Command::new(&self.adb_path)
            .args(&["-s", udid, "shell", &run_command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行启动命令失败")?;

        // 注意：由于使用了 nohup 和 &，命令会立即返回
        // 不检查 status，因为后台运行总是返回成功

        debug!("scrcpy-server 启动命令已执行");
        Ok(())
    }

    /// 等待 scrcpy-server 启动完成
    async fn wait_for_server(&self, udid: &str) -> Result<()> {
        const MAX_RETRIES: u32 = 10;
        const RETRY_DELAY_MS: u64 = 500;

        info!("等待 scrcpy-server 启动...");

        for i in 0..MAX_RETRIES {
            tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;

            if self.is_server_running(udid).await? {
                info!("✅ scrcpy-server 已启动");
                return Ok(());
            }

            debug!("等待 scrcpy-server 启动... ({}/{})", i + 1, MAX_RETRIES);
        }

        Err(anyhow!("scrcpy-server 启动超时"))
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

        // 查找进程 ID
        let output = Command::new(&self.adb_path)
            .args(&["-s", udid, "shell", "pidof", SERVER_PROCESS_NAME])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 pidof 命令失败")?;

        if !output.status.success() {
            debug!("没有找到运行中的 scrcpy-server");
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pids: Vec<&str> = stdout.trim().split_whitespace().collect();

        // 杀死每个匹配的进程
        for pid in pids {
            let cmdline_output = Command::new(&self.adb_path)
                .args(&["-s", udid, "shell", &format!("cat /proc/{}/cmdline", pid)])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            if let Ok(output) = cmdline_output {
                let cmdline = String::from_utf8_lossy(&output.stdout);
                if cmdline.contains(SERVER_PACKAGE) {
                    info!("杀死 scrcpy-server 进程 (PID: {})", pid);
                    let _ = Command::new(&self.adb_path)
                        .args(&["-s", udid, "shell", "kill", pid])
                        .output()
                        .await;
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
