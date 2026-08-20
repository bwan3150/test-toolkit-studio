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

/// WDA 在设备上的监听端口（WebDriverAgent 默认值）。
/// **真机专用**：USB 转发的对端固定是它。模拟器不走这个常量，见 `sim_port`（Q-13）
const WDA_DEVICE_PORT: u16 = 8100;

/// 模拟器里那个预编译 runner 的 bundle id
const WDA_SIM_BUNDLE: &str = "com.facebook.WebDriverAgentRunner.xctrunner";

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
            return self.ensure_simulator_wda();
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
    /// 模拟器的 WDA：**每台一个端口**（Q-13）。
    ///
    /// 模拟器与主机共享网络，WebDriverAgent 默认全都监听 8100——并行跑两台就会互相抢。
    /// 抢输的那台起不来还算好的，更糟的是**端口通、命令却发到了另一台设备上**：
    /// 每一步都报成功，动的是别人（P-35 那一族的老毛病）。
    /// 所以每台按自己的 UDID 定端口、记进自己的状态文件，启动时用 `SIMCTL_CHILD_USE_PORT`
    /// 告诉 runner 监听哪儿（`simctl` 把 `SIMCTL_CHILD_*` 转成被启动进程的环境变量）。
    ///
    /// 结果按 UDID 缓存在进程内：`ensure_forward` 一步里会被调好几次
    /// （launch_app → ensure_create → ensure_existing → …），归属校验要跑两个子进程，
    /// 每次都验就是白等
    fn ensure_simulator_wda(&self) -> Result<(String, WdaState)> {
        use std::sync::{Mutex, OnceLock};
        static CACHE: OnceLock<Mutex<std::collections::HashMap<String, (String, WdaState)>>> =
            OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        if let Some(hit) = cache.lock().ok().and_then(|c| c.get(&self.udid).cloned()) {
            return Ok(hit);
        }

        let prev = Self::load_state(&self.udid);
        // 复用：状态文件记着端口、端口通、**并且那个端口确实是这台的**。
        // 少了最后一条就会误连——两台模拟器都在跑时，8100 上应答的可能是任何一台。
        // 8100 一律不复用：那是 WDA 的出厂默认，**任何一台**没带 USE_PORT 起来的都在那儿，
        // 旧状态文件里记的也是它——认它等于认了一个公共端口
        if let Some(state) = prev.as_ref().filter(|s| s.port != 0 && s.port != WDA_DEVICE_PORT) {
            let base = format!("http://127.0.0.1:{}", state.port);
            if Self::wda_ready(&base) && self.owns_port(state.port) {
                let hit = (base, state.clone());
                if let Ok(mut c) = cache.lock() {
                    c.insert(self.udid.clone(), hit.clone());
                }
                return Ok(hit);
            }
        }

        // 端口**只认 UDID 算出来的那个**，不从状态文件继承。
        // 继承过一次就出事了（用户实测）：旧版留下的状态文件里两台都写着 8100，
        // 于是两台又都挑了 8100——「沿用上次」听着稳，实际是把历史包袱一路带下去。
        // 算出来的被别人占了就往后挪，挪到空的为止
        let want = Self::sim_port(&self.udid);
        let port = (want..want.saturating_add(20))
            .find(|p| Self::port_free(*p))
            .unwrap_or_else(Self::free_port);

        self.launch_wda_on_simulator(port)?;

        let state = WdaState {
            port,
            forward_pid: 0,   // 模拟器没有转发进程
            runwda_pid: None, // 也没有 runwda
            session_id: None,
            scale: None,
        };
        Self::save_state(&self.udid, &state);
        let hit = (format!("http://127.0.0.1:{}", port), state);
        if let Ok(mut c) = cache.lock() {
            c.insert(self.udid.clone(), hit.clone());
        }
        Ok(hit)
    }

    /// 这个端口上监听的，是不是**这台模拟器**里的 WDA。
    ///
    /// 靠得住的原因：模拟器里的进程就是 macOS 上的进程，`simctl spawn <udid> launchctl list`
    /// 报的 PID 与 `lsof` 看到的监听 PID 是同一个数。
    /// 两边有任何一边问不出来（没装 lsof、launchctl 输出变了）就**放行**——
    /// 那时退回"端口通就算数"，跟改这一版之前一样，不会更差
    fn owns_port(&self, port: u16) -> bool {
        let (Some(mine), Some(listening)) = (self.sim_wda_pid(), Self::listener_pid(port)) else {
            return true;
        };
        mine == listening
    }

    /// 这台模拟器里 WDA runner 的进程号（没跑 → None）。
    ///
    /// ⚠️ **不能用 `launchctl list <bundle-id>`**：iOS 里 App 进程在 launchd 里的 label
    /// 是 `UIKitApplication:com.facebook.…[0x…][rb-legacy]` 这种带前缀和随机后缀的东西，
    /// 拿 bundle id 精确查永远查不到——用户实测就卡在这儿：两边 PID 都报 `?`，
    /// 归属校验一路放行，于是第二台**直接复用了第一台的 WDA**（这正是要防的误连）。
    /// 所以列全表再按 bundle id 找子串
    fn sim_wda_pid(&self) -> Option<u32> {
        let out = Command::new("xcrun")
            .args(["simctl", "spawn", &self.udid, "launchctl", "list"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        // 表格式：`PID\tStatus\tLabel`，没在跑的 PID 那列是 `-`
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find(|l| l.contains(WDA_SIM_BUNDLE))
            .and_then(|l| l.split_whitespace().next()?.parse().ok())
    }

    /// 谁在监听这个端口
    fn listener_pid(port: u16) -> Option<u32> {
        let out = Command::new("lsof")
            .args(["-nP", &format!("-iTCP:{}", port), "-sTCP:LISTEN", "-t"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout).split_whitespace().next()?.parse().ok()
    }

    fn port_free(port: u16) -> bool {
        std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap_or(WDA_DEVICE_PORT)
    }

    /// 由 UDID 定的端口：`8100 + hash % 100`。
    /// **要的是稳定**——同一台模拟器每次都拿同一个端口，人对日志、抓包、`lsof` 时不用猜；
    /// 换成每次随机分配也能跑，但排查起来就没有一个固定的锚点了
    fn sim_port(udid: &str) -> u16 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        udid.hash(&mut h);
        // 从 8101 起，**有意跳过 8100**：那是 WDA 的出厂默认端口，
        // 任何一台没带 USE_PORT 起来的 WDA 都在那儿（外部起的、旧版起的），
        // 分到它就等于自愿跟所有人共用一个端口
        WDA_DEVICE_PORT + 1 + (h.finish() % 99) as u16
    }

    /// 把 WebDriverAgent 拉进**模拟器**：装（幂等）→ 起在指定端口 → 等就绪。
    ///
    /// 这条路比真机干净得多：**不用签名、不用 xcodebuild、连 .xctestrun 都不用**
    /// ——实测预编译的 `WebDriverAgentRunner-Runner.app` 用 `simctl launch` 直接就起得来
    /// （XCTest bundle 一般要 xcodebuild 带一堆环境变量，模拟器上不需要）。
    fn launch_wda_on_simulator(&self, port: u16) -> Result<()> {
        // **一次运行只试一次**。ensure_forward 在一步里会被调好几次
        // （launch_app → ensure_create → ensure_existing → …），不拦的话那行提示
        // 打四遍，失败时还要一遍遍等满 15 秒超时（实测:一步里打了 4 次）
        static TRIED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if TRIED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return Err(TkeError::DeviceError(
                "这次运行已经试过把 WebDriverAgent 拉起来了，没成功——不再重试".into(),
            ));
        }

        let app = Self::find_wda_app().ok_or_else(|| {
            TkeError::DeviceError(
                "模拟器要用 WebDriverAgent，但本机没有它：\n\
                 \u{3000}装它：tke doctor --fix --profile ios\n\
                 （自己编译的话，把 WebDriverAgentRunner-Runner.app 的路径设进 TKE_WDA_APP）"
                    .into(),
            )
        })?;

        // 注：这一下会把当前前台 App 挤走（simctl launch 必然带到前台，没有后台启动
        // 选项）。**不打提示**——正常流程里 `启动 [BundleID]` 紧接着就把 App 拉回来了，
        // 那行字对用户没有任何可做的事；真出问题时 attach_foreground 会报得很清楚。
        // install 幂等：装过也不报错，省得先查一遍
        let _ = Command::new("xcrun")
            .args(["simctl", "install", &self.udid])
            .arg(&app)
            .output();
        // `--terminate-running-process`：已经在跑的那个用的是**旧端口**，不杀掉的话
        // launch 只是把它带到前台，USE_PORT 根本不生效——端口就还是撞的
        let out = Command::new("xcrun")
            .args([
                "simctl",
                "launch",
                "--terminate-running-process",
                &self.udid,
                WDA_SIM_BUNDLE,
            ])
            .env("SIMCTL_CHILD_USE_PORT", port.to_string())
            .output()
            .map_err(|e| TkeError::DeviceError(format!("拉起模拟器上的 WDA 失败: {}", e)))?;
        if !out.status.success() {
            return Err(TkeError::DeviceError(format!(
                "拉起模拟器上的 WDA 失败: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }

        // 等就绪。**别只等一两秒**：冷启动那次要跑一遍 XCTest 初始化
        let base = format!("http://127.0.0.1:{}", port);
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(500));
            if Self::wda_ready(&base) {
                return Ok(());
            }
        }
        // 指定端口不通、而**出厂默认的 8100 通**，几乎只有一个解释：
        // 这个 runner 不认 USE_PORT，仍旧监听在 8100 上。这时候不说清楚，
        // 人会一路去查模拟器锁屏、防火墙、WDA 版本——查的全是别的地方
        if port != WDA_DEVICE_PORT
            && Self::wda_ready(&format!("http://127.0.0.1:{}", WDA_DEVICE_PORT))
        {
            return Err(TkeError::DeviceError(format!(
                "指定的端口 {} 不通，但默认的 {} 通了——这个 WebDriverAgent runner \
                 不认 SIMCTL_CHILD_USE_PORT，还监听在出厂默认端口上。\
                 多台模拟器并行会撞；单台跑不受影响。请把这条报回来（换个 runner 版本才能解）",
                port, WDA_DEVICE_PORT
            )));
        }
        Err(TkeError::DeviceError(format!(
            "WDA 起来了但 {} 一直不通（15 秒）。模拟器锁屏了？或者上一个 WDA 还占着这个端口",
            base
        )))
    }

    /// 找 `WebDriverAgentRunner-Runner.app`。
    /// `TKE_WDA_APP` 优先——自己编译的、或想试别的版本时用它顶掉
    fn find_wda_app() -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("TKE_WDA_APP") {
            let p = PathBuf::from(p);
            if p.exists() {
                return Some(p);
            }
        }
        let home = std::env::var_os("HOME")?;
        let p = PathBuf::from(home)
            .join(".tke")
            .join("wda")
            .join("WebDriverAgentRunner-Runner.app");
        p.exists().then_some(p)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 同一台每次都拿同一个端口——这是能用 `lsof` 对上号的前提（Q-13）
    #[test]
    fn sim_port_is_stable_per_udid() {
        let a = WdaDriver::sim_port("A1B2C3D4-0000-1111-2222-333344445555");
        assert_eq!(a, WdaDriver::sim_port("A1B2C3D4-0000-1111-2222-333344445555"));
    }

    /// 不同模拟器要落在不同端口上——它们共享主机网络，撞了就是命令发错设备
    #[test]
    fn sim_port_differs_across_udids() {
        let udids: Vec<String> = (0..20).map(|i| format!("UDID-{:04}-SIM", i)).collect();
        let ports: std::collections::HashSet<u16> =
            udids.iter().map(|u| WdaDriver::sim_port(u)).collect();
        // 100 个槽里放 20 台，撞几个是数学上的必然（生日问题），但不能挤成一堆
        assert!(ports.len() >= 15, "20 台只分到 {} 个端口，太挤了", ports.len());
    }

    /// 端口必须落在 8101..8200，**且永远不是 8100**——
    /// 那是 WDA 的出厂默认，分到它就等于跟所有外部起的 WDA 共用一个口
    #[test]
    fn sim_port_stays_in_range_and_skips_default() {
        for i in 0..300 {
            let p = WdaDriver::sim_port(&format!("sim-{}", i));
            assert!((8101..8200).contains(&p), "端口 {} 越界", p);
        }
    }
}
