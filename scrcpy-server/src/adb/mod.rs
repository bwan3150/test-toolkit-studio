// ADB 工具模块
mod scrcpy_server;

use anyhow::{Result, Context, anyhow};
use tokio::process::Command;
use std::process::Stdio;
use tracing::{debug, info};

pub use scrcpy_server::ScrcpyServer;

/// ADB 工具类
pub struct AdbUtils {
    adb_path: String,
}

impl AdbUtils {
    /// 创建新的 ADB 工具实例
    pub fn new(adb_path: Option<String>) -> Self {
        Self {
            adb_path: adb_path.unwrap_or_else(|| "adb".to_string()),
        }
    }

    /// 获取 ADB 路径
    pub fn get_adb_path(&self) -> &str {
        &self.adb_path
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

    /// 建立 ADB forward 转发
    /// 将本地端口转发到设备上的远程地址
    ///
    /// @param udid - 设备 ID
    /// @param remote - 远程地址，例如 "tcp:8886"
    /// @returns 本地端口号
    pub async fn forward(&self, udid: &str, remote: &str) -> Result<u16> {
        info!("建立 ADB forward: {} -> {}", udid, remote);

        // 1. 检查是否已经存在转发
        let existing_port = self.find_existing_forward(udid, remote).await?;
        if let Some(port) = existing_port {
            info!("使用已存在的 forward，端口: {}", port);
            return Ok(port);
        }

        // 2. 查找可用端口
        let port = self.find_available_port().await?;
        let local = format!("tcp:{}", port);

        // 3. 建立转发
        debug!("执行 adb -s {} forward {} {}", udid, local, remote);
        let args = self.build_adb_args(&["-s", udid, "forward", &local, remote]);
        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 adb forward 命令失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("adb forward 失败: {}", stderr));
        }

        info!("成功建立 forward: {} -> {} (本地端口: {})", udid, remote, port);
        Ok(port)
    }

    /// 查找已存在的 forward
    async fn find_existing_forward(&self, udid: &str, remote: &str) -> Result<Option<u16>> {
        debug!("检查已存在的 forward: {} -> {}", udid, remote);

        let args = self.build_adb_args(&["forward", "--list"]);
        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 adb forward --list 失败")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析输出格式: "serial tcp:port remote"
        // 例如: "emulator-5554 tcp:27183 tcp:8886"
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let serial = parts[0];
                let local = parts[1];
                let remote_part = parts[2];

                if serial == udid && remote_part == remote && local.starts_with("tcp:") {
                    if let Ok(port) = local[4..].parse::<u16>() {
                        debug!("找到已存在的 forward: 端口 {}", port);
                        return Ok(Some(port));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 查找可用端口
    /// 简单实现：从 27183 开始尝试（类似 ws-scrcpy 使用 portfinder）
    async fn find_available_port(&self) -> Result<u16> {
        use tokio::net::TcpListener;

        // 绑定到 0 端口，让系统自动分配
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("无法绑定临时端口")?;

        let port = listener.local_addr()?.port();
        drop(listener); // 释放端口

        debug!("找到可用端口: {}", port);
        Ok(port)
    }

    /// 移除 forward
    pub async fn remove_forward(&self, udid: &str, local_port: u16) -> Result<()> {
        let local = format!("tcp:{}", local_port);
        debug!("移除 forward: {} {}", udid, local);

        let args = self.build_adb_args(&["-s", udid, "forward", "--remove", &local]);
        let output = Command::new(&self.adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("执行 adb forward --remove 失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("移除 forward 失败（可能已不存在）: {}", stderr);
        } else {
            info!("成功移除 forward: {} tcp:{}", udid, local_port);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_available_port() {
        let adb = AdbUtils::new(None);
        let port = adb.find_available_port().await.unwrap();
        assert!(port > 0);
        println!("找到可用端口: {}", port);
    }
}
