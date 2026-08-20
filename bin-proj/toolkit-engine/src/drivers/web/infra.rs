// chromedriver 基础设施 - 浏览器进程生命周期 / 会话文件 / Chrome 定位 / 孤儿收割
// （不含 W3C 协议交互本身，那部分在 mod.rs）。
// 会话信息存 $TMPDIR/tke/web/<设备ID>.json，跨 tke 进程复用；失效时自动重建。

use super::{Conn, SessionInfo, WebDriver};
use crate::{Result, TkeError};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

impl WebDriver {
    pub(super) fn session_file(device_id: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tke").join("web");
        let _ = std::fs::create_dir_all(&dir);
        let key: String = device_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        dir.join(format!("{}.json", key))
    }

    pub(super) fn load_session(device_id: &str) -> Option<SessionInfo> {
        let content = std::fs::read_to_string(Self::session_file(device_id)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub(super) fn session_alive(base: &str, session_id: &str) -> bool {
        match ureq::get(&format!("{}/session/{}/url", base, session_id))
            .timeout(Duration::from_millis(1500))
            .call()
        {
            Ok(_) => true,
            // ⚠️ 有原生对话框挂着时,**任何**页面命令都会回 `unexpected alert open`,
            // 包括这条探活。那恰恰证明会话活得好好的——只是被挡住了。
            // 早先一律当"会话已死",于是撞上对话框的下一条命令直接报「无活动浏览器会话」,
            // AI 连把它点掉的机会都没有(实测撞到,P-37)。真死了的会话回的是
            // `invalid session id`,不含 alert 字样,照旧判死。
            Err(ureq::Error::Status(_, resp)) => resp
                .into_string()
                .map(|b| b.contains("unexpected alert"))
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    /// 本设备的 profile 目录前缀（孤儿识别特征）
    fn profile_prefix(device_id: &str) -> PathBuf {
        let key: String = device_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        std::env::temp_dir().join("tke").join("web").join(format!("profile-{}", key))
    }

    /// 收割本设备遗留的孤儿 Chrome（脚本中断/创建失败留下的），并清理旧 profile
    pub(super) fn reap_orphans(device_id: &str) {
        let prefix = Self::profile_prefix(device_id);
        // 按 user-data-dir 特征杀掉遗留 Chrome 进程
        let _ = Command::new("pkill")
            .arg("-f")
            .arg(format!("user-data-dir={}", prefix.to_string_lossy()))
            .output();
        // 清理旧 profile 目录
        if let (Some(parent), Some(stem)) = (prefix.parent(), prefix.file_name()) {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().starts_with(&*stem.to_string_lossy()) {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }

    /// 拉起 chromedriver 并创建新会话
    pub(super) fn start_new_session(device_id: &str) -> Result<Conn> {
        // 先收割上次遗留的孤儿进程/profile
        Self::reap_orphans(device_id);

        // chromedriver 与 tke 同目录，经统一的 ToolManager 定位（须与 Chrome 版本配对，不回退 PATH）
        let chromedriver = crate::ToolManager::resolve("chromedriver")?;
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or_else(|| TkeError::InvalidArgument("无法获取 tke 所在目录".to_string()))?;

        // 找空闲端口
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .map_err(|e| TkeError::DeviceError(format!("无法分配端口: {}", e)))?;

        // 后台拉起 chromedriver（日志落盘便于排查）
        // 清洗环境变量 + 脱离终端进程组：避免继承终端模拟器（如 Ghostty）注入的
        // 环境导致 Chrome 启动崩溃（Mach rendezvous failed / BUS_ADRALN）
        let log_path = std::env::temp_dir()
            .join("tke").join("web")
            .join(format!("chromedriver-{}.log", port));
        let mut cmd = Command::new(&chromedriver);
        cmd.arg(format!("--port={}", port))
            .arg("--verbose")
            .arg(format!("--log-path={}", log_path.to_string_lossy()))
            .env_clear();
        // 只保留必要的环境变量。
        // DISPLAY/WAYLAND_DISPLAY/XAUTHORITY 必须留：Linux **有头**模式下 Chrome 靠它们连图形栈，
        // 清掉会直接起不开（mac/win 不看这些，留着无害；无头模式下有没有都不影响）。
        for key in [
            "PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG",
            "DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY",
        ] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0); // 新进程组，脱离终端会话
        }
        let child = cmd
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| TkeError::DeviceError(format!("启动 chromedriver 失败: {}", e)))?;
        let driver_pid = child.id();

        let base = format!("http://127.0.0.1:{}", port);

        // 等待服务就绪（最多 10s）
        let ready = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(200));
            ureq::get(&format!("{}/status", base))
                .timeout(Duration::from_millis(500))
                .call()
                .is_ok()
        });
        if !ready {
            return Err(TkeError::DeviceError("chromedriver 启动超时".to_string()));
        }

        // 创建会话（优先使用配套的 Chrome for Testing，保证与 chromedriver 版本配对）
        // 注意: Chrome.app 不能放在 ~/Documents 等 TCC 受保护目录下（会卡死在授权），
        // 查找顺序: tke 同目录 → ~/Library/Application Support/tke/
        // 固定窗口尺寸 + 强制缩放因子 1：保证不同机器/显示器（视网膜或否）下
        // 渲染完全一致，截图尺寸与坐标系确定（脚本中的像素坐标可移植）
        // profile 用自有目录（带时间戳=每会话全新状态，同时是孤儿收割的识别特征）
        let profile_dir = format!(
            "{}-{}",
            Self::profile_prefix(device_id).to_string_lossy(),
            chrono::Local::now().format("%H%M%S")
        );
        let mut args: Vec<String> = vec![
            "--window-size=1280,900".to_string(),
            "--force-device-scale-factor=1".to_string(),
            "--disable-infobars".to_string(),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            format!("--user-data-dir={}", profile_dir),
        ];

        // 无头（无头服务器 / docker / CI）：默认 Auto 按环境探测，可用 --headless on/off 强制。
        // 用新版 headless（`=new`）——它跑的是完整浏览器渲染路径，与有头一致；旧版
        // (`--headless` 老实现) 是另一套精简渲染，截图会和有头对不上。
        // **窗口尺寸/缩放因子照旧固定**：脚本里的像素坐标要在有头录、无头回放之间可移植。
        let headless = crate::utils::params::web_headless().resolve();
        // 要开窗口但这台机器根本没有图形界面 → **在这儿拦下并说清楚**。
        // 不拦的话 Chrome 会起来立刻退出,chromedriver 只回一句
        // 「Chrome instance exited. Examine ChromeDriver verbose log」——
        // 人得去翻日志才知道是没有 DISPLAY,而那跟"要开窗口"隔着好几层
        if !headless && !crate::utils::params::desktop_available() {
            return Err(TkeError::DeviceError(
                concat!(
                    "这台机器没有图形界面（没有 DISPLAY / WAYLAND_DISPLAY），开不了有窗口的浏览器。\n",
                    "\u{3000}去掉 --headless=off，用无头跑（渲染结果与有头一致）"
                )
                    .into(),
            ));
        }
        if headless {
            args.push("--headless=new".to_string());
            args.push("--disable-gpu".to_string());
        }

        // 容器 / root 下 Chrome 的沙箱起不来，且 /dev/shm 默认只有 64MB 会让渲染进程崩。
        // 只在真的处于容器或 root 时加——普通桌面环境保留沙箱（安全）。
        if in_container_or_root() {
            args.push("--no-sandbox".to_string());
            args.push("--disable-dev-shm-usage".to_string());
        }

        let mut chrome_options = serde_json::json!({ "args": args });
        if let Some(cft) = Self::find_chrome_binary(&exe_dir) {
            chrome_options["binary"] = serde_json::json!(cft.to_string_lossy());
        }

        let session_req = ureq::post(&format!("{}/session", base))
            .timeout(Duration::from_secs(90))
            .send_json(serde_json::json!({
                "capabilities": {
                    "alwaysMatch": {
                        "browserName": "chrome",
                        // 原生对话框(alert/confirm/prompt)**不要让 WebDriver 自作主张**。
                        // W3C 默认是 "dismiss and notify" —— confirm 会被**自动点成取消**,
                        // 而 tke 这边只看到"点击成功"。实测:点「删除」弹 confirm,结果页面
                        // 显示 CANCELLED,全程没有一句提示。AI 由此得出「删除功能失效」的
                        // 假结论(P-37)。改成 ignore:对话框留在那儿,由 tke 探测、报出来,
                        // 再由调用方显式决定确认还是取消
                        "unhandledPromptBehavior": "ignore",
                        "goog:chromeOptions": chrome_options
                    }
                }
            }));

        let resp: serde_json::Value = match session_req {
            Ok(r) => r.into_json()
                .map_err(|e| TkeError::DeviceError(format!("会话响应解析失败: {}", e)))?,
            Err(e) => {
                // 失败时回收孤儿 chromedriver，并带上具体错误信息
                let _ = Command::new("kill").arg(driver_pid.to_string()).output();
                let detail = match e {
                    ureq::Error::Status(_, resp) => resp
                        .into_json::<serde_json::Value>()
                        .ok()
                        .and_then(|v| v["value"]["message"].as_str().map(|m| {
                            m.lines().next().unwrap_or(m).to_string()
                        }))
                        .unwrap_or_else(|| "未知错误".to_string()),
                    other => other.to_string(),
                };
                // 会话建不起来最常见的原因是**没有 Chrome for Testing**：chromedriver 只能
                // 去找系统 Chrome，版本对不上就是这句干巴巴的 session not created。
                // 光把 chromedriver 的原话抛出来，没人看得出缺的是浏览器本体。
                let hint = if crate::utils::deps::chrome_for_testing_bin().is_none() {
                    format!(
                        "\n  ⚠ 没有找到 Chrome for Testing，chromedriver 只能去找系统 Chrome，版本多半对不上。\n    {}",
                        crate::utils::deps::fix_hint("web")
                    )
                } else {
                    String::new()
                };
                return Err(TkeError::DeviceError(format!(
                    "创建浏览器会话失败: {} (日志: {}){}", detail, log_path.display(), hint
                )));
            }
        };

        let session_id = resp["value"]["sessionId"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError(format!("会话响应缺少 sessionId: {}", resp)))?
            .to_string();

        // 持久化会话信息
        let info = SessionInfo { port, session_id: session_id.clone(), driver_pid, headless };
        let _ = std::fs::write(
            Self::session_file(device_id),
            serde_json::to_string(&info).unwrap_or_default(),
        );

        Ok(Conn { base, session_id })
    }

    /// 查找 Chrome for Testing 二进制
    /// 顺序: tke 同目录 → ~/Library/Application Support/tke/；找不到则交给
    /// chromedriver 用系统 Chrome（存在版本不配对风险）
    fn find_chrome_binary(exe_dir: &std::path::Path) -> Option<PathBuf> {
        // 路径定义在 utils::deps::CHROME_REL —— 与 `tke fix` 的检测**共用同一份**。
        // 各写一套的话会出这种怪事：fix 说装好了，驱动却找不到。
        // 搜索根：tke 同目录（生产打包）→ 用户数据目录（开发机/CI 推荐，避开 macOS TCC
        // 保护目录，见 setup-notes「Chrome for Testing 的三个坑」）。
        // 找不到则交给 chromedriver 用系统 Chrome（存在版本不配对风险，但不提前拦）。
        let mut roots = vec![exe_dir.to_path_buf()];
        if let Some(d) = crate::utils::deps::chrome_data_dir() {
            roots.push(d);
        }
        roots
            .iter()
            .flat_map(|root| crate::utils::deps::CHROME_REL.iter().map(move |rel| root.join(rel)))
            .find(|p| p.exists())
    }
}

/// 是否处于容器内或以 root 运行——Chrome 沙箱在这两种情况下起不来。
/// docker: `/.dockerenv`；podman: `/run/.containerenv`。
fn in_container_or_root() -> bool {
    #[cfg(unix)]
    {
        // Safety: getuid 无副作用、永远成功
        let is_root = unsafe { libc_getuid() } == 0;
        is_root
            || std::path::Path::new("/.dockerenv").exists()
            || std::path::Path::new("/run/.containerenv").exists()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

// 取当前 uid。本项目没有 libc 依赖，只为这一个调用引入不划算，直接声明该符号。
#[cfg(unix)]
extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}
