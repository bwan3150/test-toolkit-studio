// Server模块 - 负责管理手机上的 tke-autoserver
// 职责：版本检查、注入 autoserver、启动/停止视频流

use crate::{Result, TkeError, AdbManager};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const DEFAULT_PORT: u16 = 8765;

/// AutoServer 管理器
pub struct AutoServer {
    device_id: Option<String>,
    adb_manager: AdbManager,
    port: u16,
}

impl AutoServer {
    /// 创建新的 AutoServer 实例
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let adb_manager = AdbManager::new()?;
        adb_manager.verify_adb()?;

        Ok(Self {
            device_id,
            adb_manager,
            port: DEFAULT_PORT,
        })
    }

    /// 设置端口（可选，默认 8765）
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// 获取当前使用的端口
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 获取 tke 可执行文件所在目录
    fn get_tke_dir() -> Result<PathBuf> {
        let exe_path = std::env::current_exe()
            .map_err(|e| TkeError::IoError(e))?;

        exe_path.parent()
            .ok_or_else(|| TkeError::InvalidArgument("无法获取可执行文件目录".to_string()))
            .map(|p| p.to_path_buf())
    }

    /// 获取 tke 版本号
    fn get_tke_version() -> &'static str {
        env!("BUILD_VERSION")
    }

    /// 获取手机上的 autoserver 版本号
    fn get_device_autoserver_version(&self) -> Result<String> {
        let output = self.run_adb_command_output(&[
            "shell",
            "CLASSPATH=/data/local/tmp/tke-autoserver",
            "app_process",
            "/",
            "app.TestToolkit.TKE.AutoServer.Server",
            "version"
        ])?;

        // 输出格式: "Toolkit Engine AutoServer vX.X.X-beta"
        if let Some(version) = output.trim().split("v").nth(1) {
            Ok(version.to_string())
        } else {
            Err(TkeError::InvalidArgument("无法解析 autoserver 版本号".to_string()))
        }
    }

    /// 检查是否需要注入 autoserver
    fn should_inject_autoserver(&self) -> Result<bool> {
        // 检查文件是否存在
        let check_output = self.run_adb_command_output(&[
            "shell",
            "test",
            "-f",
            "/data/local/tmp/tke-autoserver",
            "&&",
            "echo",
            "exists"
        ]);

        if check_output.is_err() || !check_output.unwrap().contains("exists") {
            // 文件不存在，需要注入
            return Ok(true);
        }

        // 文件存在，检查版本
        match self.get_device_autoserver_version() {
            Ok(device_version) => {
                let tke_version = Self::get_tke_version();
                Ok(device_version != tke_version)
            }
            Err(_) => {
                // 获取版本失败，需要重新注入
                Ok(true)
            }
        }
    }

    /// 注入 autoserver 到手机
    fn inject_autoserver(&self) -> Result<()> {
        let tke_dir = Self::get_tke_dir()?;
        let autoserver_path = tke_dir.join("tke-autoserver");

        if !autoserver_path.exists() {
            return Err(TkeError::InvalidArgument(
                format!("找不到 tke-autoserver: {:?}", autoserver_path)
            ));
        }

        // 推送到设备
        self.run_adb_command(&[
            "push",
            autoserver_path.to_str()
                .ok_or_else(|| TkeError::InvalidArgument("无效的路径".to_string()))?,
            "/data/local/tmp/tke-autoserver"
        ])?;

        Ok(())
    }

    /// 设置端口转发
    fn setup_port_forward(&self) -> Result<()> {
        let forward_spec = format!("tcp:{}", self.port);
        self.run_adb_command(&["forward", &forward_spec, &forward_spec])?;
        Ok(())
    }

    /// 启动 autoserver screenshot-server
    pub fn start(&self) -> Result<()> {
        // 检查是否需要注入
        if self.should_inject_autoserver()? {
            self.inject_autoserver()?;
        }

        // 设置端口转发
        self.setup_port_forward()?;

        // 使用 sh -c '命令 &' && sleep 的方式启动后台进程
        // 这样 shell 会在后台启动进程后立即退出，而后台进程继续运行
        let cmd_str = "sh -c 'CLASSPATH=/data/local/tmp/tke-autoserver app_process / app.TestToolkit.TKE.AutoServer.Server screenshot-server >/dev/null 2>&1 &' && sleep 0.1";

        self.run_adb_command(&[
            "shell",
            cmd_str
        ])?;

        // 等待服务器启动
        std::thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// 停止 autoserver
    pub fn stop(&self) -> Result<()> {
        // 使用 pkill -9 强制杀死所有相关进程（包括父进程和子进程）
        // 注意：进程名为 app_process，需要通过命令行参数 screenshot-server 来匹配
        let _ = self.run_adb_command(&[
            "shell",
            "pkill",
            "-9",
            "-f",
            "screenshot-server"
        ]);

        // 移除端口转发
        let forward_spec = format!("tcp:{}", self.port);
        let _ = self.run_adb_command(&[
            "forward",
            "--remove",
            &forward_spec
        ]);

        Ok(())
    }

    /// 检查 screenshot-server 是否在运行
    pub fn is_running(&self) -> bool {
        // 通过 pgrep 检查进程是否存在
        // 注意：进程名为 app_process，需要通过命令行参数 screenshot-server 来匹配
        let output = self.run_adb_command_output(&[
            "shell",
            "pgrep",
            "-f",
            "screenshot-server"
        ]);

        // 如果命令成功且有输出，说明进程存在
        match output {
            Ok(out) => !out.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// 执行ADB命令
    fn run_adb_command(&self, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new(self.adb_manager.adb_path());

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

        Ok(())
    }

    /// 执行ADB命令并获取输出
    fn run_adb_command_output(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(self.adb_manager.adb_path());

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
