// go-ios 基础设施 - iOS 自动化的进程/隧道/端口转发管理（不含 WDA HTTP 协议本身）
// 全部经 go-ios（与 tke 同目录的单文件二进制）自动拉起并跨 tke 进程复用:
//   ① go-ios tunnel start --userspace   iOS 17+ 隧道（全设备共用一个守护进程）
//   ② go-ios runwda --udid <udid>       经 testmanagerd 拉起设备上的 WDA（无需 Xcode）
//   ③ go-ios forward <port> 8100        USB 端口转发
// 唯一前置条件: 设备已用 Xcode 装过 WebDriverAgent App（一次性，见 docs/setup-notes.md）。

use super::{WdaDriver, WdaState};
use crate::{Result, TkeError};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// WDA 在设备上的监听端口（WebDriverAgent 默认值）
const WDA_DEVICE_PORT: u16 = 8100;

impl WdaDriver {
    /// WDA /status 是否可达
    fn wda_ready(base: &str) -> bool {
        ureq::get(&format!("{}/status", base))
            .timeout(Duration::from_millis(1500))
            .call()
            .is_ok()
    }

    /// 确保 USB 转发就绪且 WDA 可达，返回 (base_url, 已有状态)
    /// WDA 不可达时自动经 go-ios（隧道 + runwda）拉起
    pub(super) fn ensure_forward(&self) -> Result<(String, WdaState)> {
        // ── 模拟器：没有 USB，也就没有隧道和转发这回事 ──
        // 模拟器与主机共享网络，跑在里面的 WDA 直接就在 127.0.0.1:8100 上。
        // 拉起 WDA 这一步**不归 tke 管**（go-ios 只能对真机做，模拟器没有 lockdown）——
        // 所以这里只连、连不上就如实说清楚该怎么把它跑起来。
        if self.is_simulator() {
            let base = format!("http://127.0.0.1:{}", WDA_DEVICE_PORT);
            if Self::wda_ready(&base) {
                let state = Self::load_state(&self.udid).unwrap_or(WdaState {
                    port: WDA_DEVICE_PORT,
                    forward_pid: 0,   // 模拟器没有转发进程
                    runwda_pid: None, // 也没有 runwda
                    session_id: None,
                    scale: None,
                });
                return Ok((base, WdaState { port: WDA_DEVICE_PORT, ..state }));
            }
            return Err(TkeError::DeviceError(format!(
                "模拟器 {} 上的 WebDriverAgent 没在跑（{} 连不上）。\n\
                 先把 WDA 跑进模拟器再回来——模拟器不需要签名，装一次就行：\n\
                 　xcodebuild -project WebDriverAgent.xcodeproj -scheme WebDriverAgentRunner \\\n\
                 　  -destination 'id={}' test-without-building\n\
                 （tke 暂不自动拉起模拟器上的 WDA：go-ios 只对真机有效，模拟器没有 lockdown 通道）",
                self.udid, base, self.udid
            )));
        }

        // 复用已有转发
        if let Some(state) = Self::load_state(&self.udid) {
            let base = format!("http://127.0.0.1:{}", state.port);
            if Self::wda_ready(&base) {
                return Ok((base, state));
            }
        }

        let goios = Self::find_goios()?;

        // 收割本设备遗留的转发进程，再新开
        let _ = Command::new("pkill")
            .arg("-f")
            .arg(format!("go-ios forward.*{}", self.udid))
            .output();

        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .map_err(|e| TkeError::DeviceError(format!("无法分配端口: {}", e)))?;

        let forward_pid = Self::spawn_daemon(
            &goios,
            &[&port.to_string(), &WDA_DEVICE_PORT.to_string(), "--udid", &self.udid],
            "forward",
            &format!("forward-{}.log", self.udid),
        )?;

        let base = format!("http://127.0.0.1:{}", port);

        // 转发建立很快; WDA 没跑则自动拉起
        let mut runwda_pid = None;
        let ready = (0..6).any(|_| {
            std::thread::sleep(Duration::from_millis(300));
            Self::wda_ready(&base)
        });
        if !ready {
            runwda_pid = Some(self.start_wda(&goios, &base, forward_pid)?);
        }

        let state = WdaState { port, forward_pid, runwda_pid, session_id: None, scale: None };
        Self::save_state(&self.udid, &state);
        Ok((base, state))
    }

    /// 经 go-ios 拉起设备上的 WDA：确保隧道守护进程 → runwda → 等待就绪
    /// 返回 runwda 进程 pid
    fn start_wda(&self, goios: &std::path::Path, base: &str, forward_pid: u32) -> Result<u32> {
        // ① 隧道守护进程（iOS 17+ 必需; 全设备共用, 已在跑则跳过）
        let tunnel_running = Command::new("pgrep")
            .arg("-f")
            .arg("go-ios tunnel start")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !tunnel_running {
            Self::spawn_daemon(goios, &["start", "--userspace"], "tunnel", "tunnel.log")?;
            std::thread::sleep(Duration::from_secs(3)); // 等隧道协商
        }

        // ② 收割旧 runwda 后拉起（同设备只能有一个 XCTest 会话）
        let _ = Command::new("pkill")
            .arg("-f")
            .arg(format!("go-ios runwda.*{}", self.udid))
            .output();
        let runwda_pid = Self::spawn_daemon(
            goios,
            &["--udid", &self.udid],
            "runwda",
            &format!("runwda-{}.log", self.udid),
        )?;

        // ③ 等 WDA 就绪（设备上 App 启动 + 服务监听，首次可能较慢）
        let ready = (0..40).any(|_| {
            std::thread::sleep(Duration::from_millis(1500));
            Self::wda_ready(base)
        });
        if !ready {
            let log = Self::log_dir().join(format!("runwda-{}.log", self.udid));
            let _ = Command::new("kill").arg(runwda_pid.to_string()).output();
            let _ = Command::new("kill").arg(forward_pid.to_string()).output();
            return Err(TkeError::DeviceError(format!(
                "自动启动 WebDriverAgent 失败：请确认设备 {} 已连接并解锁、\
                 已用 Xcode 安装过 WebDriverAgent（一次性，见 docs/setup-notes.md）。\
                 日志: {}",
                self.udid,
                log.display()
            )));
        }
        Ok(runwda_pid)
    }

    /// 状态文件 / go-ios 日志所在目录（$TMPDIR/tke/ios）
    pub(super) fn log_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("tke").join("ios");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// 拉起 go-ios 后台守护进程（脱离终端进程组，跨 tke 进程存活，日志落盘）
    fn spawn_daemon(goios: &std::path::Path, args: &[&str], subcmd: &str, log_name: &str) -> Result<u32> {
        let log = std::fs::File::create(Self::log_dir().join(log_name))
            .map_err(TkeError::IoError)?;
        let mut cmd = Command::new(goios);
        // go-ios 会把配对身份文件（selfIdentity.plist）写到 cwd，
        // 固定到状态目录，避免污染用户的项目目录
        cmd.arg(subcmd).args(args).current_dir(Self::log_dir());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0); // 脱离终端进程组，跨 tke 进程存活
        }
        let child = cmd
            .stdout(std::process::Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| TkeError::DeviceError(format!("启动 go-ios {} 失败: {}", subcmd, e)))?;
        Ok(child.id())
    }

    /// 查找 go-ios: 经统一的 ToolManager 在 tke 同目录定位，找不到再回退系统 PATH
    pub(super) fn find_goios() -> Result<PathBuf> {
        if let Ok(p) = crate::ToolManager::resolve("go-ios") {
            return Ok(p);
        }
        which::which("go-ios").map_err(|_| {
            TkeError::InvalidArgument(
                "go-ios 可执行文件缺失或不完整：请将其放在与 tke 相同的目录下\
                 （下载: https://github.com/danielpaulus/go-ios/releases）".to_string(),
            )
        })
    }
}
