// WdaDriver - iOS 驱动（WebDriverAgent, W3C WebDriver 系协议）
//
// 基础设施全部由 tke 通过 go-ios（与 tke 同目录的单文件二进制）自动管理:
//   ① go-ios tunnel start --userspace   iOS 17+ 隧道（全设备共用一个守护进程）
//   ② go-ios runwda --udid <udid>       经 testmanagerd 拉起设备上的 WDA（无需 Xcode）
//   ③ go-ios forward <port> 8100        USB 端口转发
// 唯一前置条件: 设备已用 Xcode 装过 WebDriverAgent App（一次性，见 docs/setup-notes.md）。
//
// 会话持久化：与 web 驱动同款——tke 每条指令是短命进程，
// 转发端口/会话/后台进程信息存 $TMPDIR/tke/ios/<udid>.json，每次命令读取复用。
//
// 坐标系：对外统一使用截图像素坐标（与 Android/Web 一致），
// WDA 协议使用逻辑点（point），内部按 scale（视网膜倍率，如 iPhone 12 = 3）换算。

use crate::{Result, TkeError, DeviceInfo};
use crate::utils::Workarea;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// 持久化的转发/会话信息
#[derive(Debug, Serialize, Deserialize)]
struct WdaState {
    /// go-ios forward 本地转发端口
    port: u16,
    forward_pid: u32,
    /// go-ios runwda 进程（由 tke 拉起时记录；外部启动的 WDA 则为 None）
    #[serde(default)]
    runwda_pid: Option<u32>,
    #[serde(default)]
    session_id: Option<String>,
    /// 视网膜倍率（截图像素 / 逻辑点）
    #[serde(default)]
    scale: Option<f64>,
}

/// 活动连接
#[derive(Debug, Clone)]
struct Conn {
    base: String,
    session_id: String,
    scale: f64,
}

/// iOS 驱动（惰性会话：仅 启动 指令可创建，其余操作要求已有会话）
pub struct WdaDriver {
    /// 设备 UDID（已去除 wda: 前缀）
    udid: String,
    conn: std::sync::Mutex<Option<Conn>>,
}

/// WDA 在设备上的监听端口（WebDriverAgent 默认值）
const WDA_DEVICE_PORT: u16 = 8100;

impl WdaDriver {
    pub fn new(device_id: String) -> Result<Self> {
        let udid = device_id.strip_prefix("wda:").unwrap_or(&device_id).to_string();
        Ok(Self { udid, conn: std::sync::Mutex::new(None) })
    }

    // ===== 状态文件 =====

    fn state_file(udid: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tke").join("ios");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("{}.json", udid))
    }

    fn load_state(udid: &str) -> Option<WdaState> {
        let content = std::fs::read_to_string(Self::state_file(udid)).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_state(udid: &str, state: &WdaState) {
        let _ = std::fs::write(
            Self::state_file(udid),
            serde_json::to_string(state).unwrap_or_default(),
        );
    }

    // ===== 基础设施（go-ios: 转发/隧道/runwda） =====

    /// WDA /status 是否可达
    fn wda_ready(base: &str) -> bool {
        ureq::get(&format!("{}/status", base))
            .timeout(Duration::from_millis(1500))
            .call()
            .is_ok()
    }

    /// 确保 USB 转发就绪且 WDA 可达，返回 (base_url, 已有状态)
    /// WDA 不可达时自动经 go-ios（隧道 + runwda）拉起
    fn ensure_forward(&self) -> Result<(String, WdaState)> {
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

    fn log_dir() -> PathBuf {
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

    /// 查找 go-ios: tke 同目录 → PATH
    fn find_goios() -> Result<PathBuf> {
        if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
            let local = exe_dir.join("go-ios");
            if local.exists() {
                return Ok(local);
            }
        }
        which::which("go-ios").map_err(|_| {
            TkeError::InvalidArgument(
                "go-ios 可执行文件缺失或不完整：请将其放在与 tke 相同的目录下\
                 （下载: https://github.com/danielpaulus/go-ios/releases）".to_string(),
            )
        })
    }

    // ===== 会话管理 =====

    /// 会话是否存活
    fn session_alive(base: &str, session_id: &str) -> bool {
        ureq::get(&format!("{}/session/{}/wda/activeAppInfo", base, session_id))
            .timeout(Duration::from_secs(5))
            .call()
            .is_ok()
    }

    /// 取已有会话，不存在则报错（除 启动 外的所有操作都用这个）
    fn ensure_existing(&self) -> Result<Conn> {
        let mut guard = self.conn.lock().unwrap();
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }

        let (base, mut state) = self.ensure_forward()?;
        if let Some(sid) = state.session_id.clone() {
            if Self::session_alive(&base, &sid) {
                let scale = match state.scale {
                    Some(s) => s,
                    None => {
                        let s = Self::fetch_scale(&base, &sid)?;
                        state.scale = Some(s);
                        Self::save_state(&self.udid, &state);
                        s
                    }
                };
                let conn = Conn { base, session_id: sid, scale };
                *guard = Some(conn.clone());
                return Ok(conn);
            }
        }

        Err(TkeError::DeviceError(
            "无活动 WDA 会话，请先执行 启动 [BundleID] 或 control launch <BundleID>".to_string(),
        ))
    }

    /// 取活动会话，不存在则新建（仅供 启动 使用）
    /// bundle_id: 新建会话时随会话拉起的 App
    fn ensure_create(&self, bundle_id: &str) -> Result<Conn> {
        if let Ok(conn) = self.ensure_existing() {
            return Ok(conn);
        }

        let (base, mut state) = self.ensure_forward()?;

        // 创建会话（带 bundleId 即拉起 App；App 启动可能较慢）
        let resp: serde_json::Value = ureq::post(&format!("{}/session", base))
            .timeout(Duration::from_secs(60))
            .send_json(serde_json::json!({
                "capabilities": { "alwaysMatch": { "bundleId": bundle_id } }
            }))
            .map_err(|e| TkeError::DeviceError(format!("创建 WDA 会话失败: {}", e)))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("WDA 会话响应解析失败: {}", e)))?;

        let session_id = resp["sessionId"]
            .as_str()
            .or_else(|| resp["value"]["sessionId"].as_str())
            .ok_or_else(|| TkeError::DeviceError(format!("WDA 会话响应缺少 sessionId: {}", resp)))?
            .to_string();

        let scale = Self::fetch_scale(&base, &session_id)?;
        state.session_id = Some(session_id.clone());
        state.scale = Some(scale);
        Self::save_state(&self.udid, &state);

        let conn = Conn { base, session_id, scale };
        *self.conn.lock().unwrap() = Some(conn.clone());
        Ok(conn)
    }

    /// 视网膜倍率（截图像素 / 逻辑点），WDA /wda/screen 直接给出
    fn fetch_scale(base: &str, session_id: &str) -> Result<f64> {
        let resp: serde_json::Value = ureq::get(&format!("{}/session/{}/wda/screen", base, session_id))
            .timeout(Duration::from_secs(10))
            .call()
            .map_err(|e| TkeError::DeviceError(format!("获取屏幕倍率失败: {}", e)))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("屏幕倍率响应解析失败: {}", e)))?;
        resp["value"]["scale"]
            .as_f64()
            .filter(|s| *s > 0.0)
            .ok_or_else(|| TkeError::DeviceError("屏幕倍率响应无 scale 字段".to_string()))
    }

    /// 销毁会话：结束 App + 删除会话；保留转发/隧道/WDA 进程（轻量，下次直接复用）
    pub fn close_session(&self) -> Result<()> {
        if let Some(conn) = self.conn.lock().unwrap().take() {
            let _ = ureq::delete(&format!("{}/session/{}", conn.base, conn.session_id))
                .timeout(Duration::from_secs(15))
                .call();
        } else if let Some(state) = Self::load_state(&self.udid) {
            if let Some(sid) = &state.session_id {
                let base = format!("http://127.0.0.1:{}", state.port);
                let _ = ureq::delete(&format!("{}/session/{}", base, sid))
                    .timeout(Duration::from_secs(15))
                    .call();
            }
        }
        // 状态文件仅清掉会话，端口转发保留
        if let Some(mut state) = Self::load_state(&self.udid) {
            state.session_id = None;
            Self::save_state(&self.udid, &state);
        }
        Ok(())
    }

    // ===== 协议基础 =====

    fn get(&self, path: &str) -> Result<serde_json::Value> {
        let conn = self.ensure_existing()?;
        ureq::get(&format!("{}/session/{}{}", conn.base, conn.session_id, path))
            .timeout(Duration::from_secs(30))
            .call()
            .map_err(|e| TkeError::DeviceError(format!("WDA 请求失败 {}: {}", path, Self::err_detail(e))))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("WDA 响应解析失败: {}", e)))
    }

    fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let conn = self.ensure_existing()?;
        ureq::post(&format!("{}/session/{}{}", conn.base, conn.session_id, path))
            .timeout(Duration::from_secs(60))
            .send_json(body)
            .map_err(|e| TkeError::DeviceError(format!("WDA {}: {}", path, Self::err_detail(e))))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("WDA 响应解析失败: {}", e)))
    }

    /// 提取 WDA 返回的具体错误信息（只取首行，后面是冗长堆栈）
    fn err_detail(e: ureq::Error) -> String {
        match e {
            ureq::Error::Status(code, resp) => resp
                .into_json::<serde_json::Value>()
                .ok()
                .and_then(|v| {
                    v["value"]["message"]
                        .as_str()
                        .map(|m| m.lines().next().unwrap_or(m).to_string())
                })
                .unwrap_or_else(|| format!("status {}", code)),
            other => other.to_string(),
        }
    }

    /// 屏幕逻辑尺寸（点）
    fn window_size(&self) -> Result<(f64, f64)> {
        let resp = self.get("/window/size")?;
        let w = resp["value"]["width"].as_f64();
        let h = resp["value"]["height"].as_f64();
        match (w, h) {
            (Some(w), Some(h)) => Ok((w, h)),
            _ => Err(TkeError::DeviceError("窗口尺寸响应无数据".to_string())),
        }
    }

    // ===== 页面状态采集 =====

    pub fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        self.capture_screenshot(workarea)?;
        self.capture_xcui_xml(workarea)?;
        Ok(())
    }

    pub fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        self.capture_xcui_xml(workarea)
    }

    fn capture_screenshot(&self, workarea: &Workarea) -> Result<()> {
        // /screenshot 也可无会话调用，但统一要求会话在先（与操作语义一致）
        let conn = self.ensure_existing()?;
        let resp: serde_json::Value = ureq::get(&format!("{}/screenshot", conn.base))
            .timeout(Duration::from_secs(30))
            .call()
            .map_err(|e| TkeError::DeviceError(format!("截图失败: {}", Self::err_detail(e))))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("截图响应解析失败: {}", e)))?;
        let b64 = resp["value"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError("截图响应无数据".to_string()))?;
        let png = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| TkeError::DeviceError(format!("截图解码失败: {}", e)))?;
        std::fs::write(workarea.screenshot_path(), png).map_err(TkeError::IoError)?;
        Ok(())
    }

    /// 抓取 XCUI 元素树并归一化为 uiautomator 风格 XML：
    /// iOS 元素与 App/Web 元素进入同一套解析/识别/标注体系
    /// (class=XCUIElementType*, resource-id=name, content-desc=label,
    ///  text=value|label, bounds=截图像素坐标)
    fn capture_xcui_xml(&self, workarea: &Workarea) -> Result<()> {
        let conn = self.ensure_existing()?;
        let resp = self.get("/source")?;
        let source = resp["value"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError("UI 结构响应无数据".to_string()))?;

        let xml = normalize_xcui_xml(source, conn.scale)?;
        std::fs::write(workarea.ui_tree_path(), xml).map_err(TkeError::IoError)?;
        Ok(())
    }

    // ===== 操作（入参为截图像素坐标） =====

    /// 启动 App（BundleID）；iOS 上 activity 概念不存在，忽略
    pub fn launch_app(&self, bundle_id: &str) -> Result<()> {
        // 已有会话 → 会话内拉起；无会话 → 创建会话随之拉起（唯一的会话创建入口）
        let had_session = self.ensure_existing().is_ok();
        self.ensure_create(bundle_id)?;
        if had_session {
            self.post("/wda/apps/launch", serde_json::json!({ "bundleId": bundle_id }))?;
        }
        Ok(())
    }

    /// 关闭 App（BundleID）；空串 = 销毁整个会话
    /// 无 WDA 会话时退回 go-ios kill（脚本开头 关闭 保证冷启动的场景）
    pub fn stop_app(&self, bundle_id: &str) -> Result<()> {
        if bundle_id.is_empty() {
            return self.close_session();
        }
        if self.ensure_existing().is_ok() {
            self.post("/wda/apps/terminate", serde_json::json!({ "bundleId": bundle_id }))?;
            return Ok(());
        }
        // 无会话: go-ios kill 一次性命令直接结束 App（App 本就没跑也算成功）
        let goios = Self::find_goios()?;
        let output = Command::new(&goios)
            .args(["kill", bundle_id, "--udid", &self.udid])
            .current_dir(Self::log_dir())
            .output()
            .map_err(|e| TkeError::DeviceError(format!("go-ios kill 执行失败: {}", e)))?;
        let msg = String::from_utf8_lossy(&output.stderr);
        // "process not found" = App 本就没在跑, 关闭目的已达成
        if msg.contains("\"killed\"") || msg.contains("process not found") {
            Ok(())
        } else if output.status.success() {
            Ok(())
        } else {
            Err(TkeError::DeviceError(format!(
                "关闭 App 失败: {}",
                msg.lines().last().unwrap_or("未知错误")
            )))
        }
    }

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        let conn = self.ensure_existing()?;
        let (px, py) = (x as f64 / conn.scale, y as f64 / conn.scale);
        self.post("/wda/tap", serde_json::json!({ "x": px, "y": py }))?;
        Ok(())
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        let conn = self.ensure_existing()?;
        let (px, py) = (x as f64 / conn.scale, y as f64 / conn.scale);
        self.post("/wda/touchAndHold", serde_json::json!({
            "x": px, "y": py,
            "duration": duration_ms as f64 / 1000.0
        }))?;
        Ok(())
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        let conn = self.ensure_existing()?;
        let s = conn.scale;
        self.post("/wda/dragfromtoforduration", serde_json::json!({
            "fromX": x1 as f64 / s, "fromY": y1 as f64 / s,
            "toX": x2 as f64 / s, "toY": y2 as f64 / s,
            // duration 是拖拽前按住的时长，滑动手势用短按住即可
            "duration": (duration_ms as f64 / 1000.0).clamp(0.1, 2.0)
        }))?;
        Ok(())
    }

    /// 输入文本到当前聚焦元素
    pub fn input_text(&self, text: &str) -> Result<()> {
        self.post("/wda/keys", serde_json::json!({ "value": [text] }))?;
        Ok(())
    }

    /// 清空当前聚焦输入框（WDA 无全局 clear，退格兜底）
    pub fn clear_input(&self) -> Result<()> {
        let backspaces: Vec<String> = std::iter::repeat("\u{8}".to_string()).take(50).collect();
        self.post("/wda/keys", serde_json::json!({ "value": backspaces }))?;
        Ok(())
    }

    /// 按键：回车映射为换行输入，BACK 走返回手势，其余忽略
    pub fn key_event(&self, key_code: &str) -> Result<()> {
        match key_code {
            "KEYCODE_ENTER" | "ENTER" => self.input_text("\n"),
            "KEYCODE_BACK" => self.back(),
            _ => Ok(()),
        }
    }

    /// 返回 = 屏幕左缘右滑手势（iOS 系统级返回）
    pub fn back(&self) -> Result<()> {
        let (w, h) = self.window_size()?;
        self.post("/wda/dragfromtoforduration", serde_json::json!({
            "fromX": 1.0, "fromY": h / 2.0,
            "toX": w * 0.6, "toY": h / 2.0,
            "duration": 0.1
        }))?;
        Ok(())
    }

    /// 主页 = 回到主屏幕
    pub fn home(&self) -> Result<()> {
        let conn = self.ensure_existing()?;
        ureq::post(&format!("{}/wda/homescreen", conn.base))
            .timeout(Duration::from_secs(15))
            .send_json(serde_json::json!({}))
            .map_err(|e| TkeError::DeviceError(format!("回主屏失败: {}", Self::err_detail(e))))?;
        Ok(())
    }

    pub fn hide_keyboard(&self) -> Result<()> {
        // 键盘没弹出时 WDA 会报错，忽略
        let _ = self.post("/wda/keyboard/dismiss", serde_json::json!({}));
        Ok(())
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        let conn = self.ensure_existing()?;
        let (w, h) = self.window_size().unwrap_or((0.0, 0.0));

        // /status 里有 iOS 版本和设备类型
        let status: Option<serde_json::Value> = ureq::get(&format!("{}/status", conn.base))
            .timeout(Duration::from_secs(5))
            .call()
            .ok()
            .and_then(|r| r.into_json().ok());
        let os_version = status
            .as_ref()
            .and_then(|s| s["value"]["os"]["version"].as_str())
            .map(|v| format!("iOS {}", v));

        Ok(DeviceInfo {
            id: self.udid.clone(),
            model: Some("iPhone (WDA)".to_string()),
            manufacturer: Some("Apple".to_string()),
            android_version: os_version,
            screen_width: (w * conn.scale) as u32,
            screen_height: (h * conn.scale) as u32,
            hardware: None,
            battery: None,
            network: None,
        })
    }
}

/// XCUI 元素树 → uiautomator 风格扁平 XML（仅可见元素，bounds 换算为像素）
fn normalize_xcui_xml(source: &str, scale: f64) -> Result<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let mut typ = String::new();
                let mut name = String::new();
                let mut label = String::new();
                let mut value = String::new();
                let mut visible = false;
                let mut accessible = false;
                let (mut x, mut y, mut w, mut h) = (0i64, 0i64, 0i64, 0i64);

                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    match key.as_str() {
                        "type" => typ = val,
                        "name" => name = val,
                        "label" => label = val,
                        "value" => value = val,
                        "visible" => visible = val == "true",
                        "accessible" => accessible = val == "true",
                        "x" => x = val.parse().unwrap_or(0),
                        "y" => y = val.parse().unwrap_or(0),
                        "width" => w = val.parse().unwrap_or(0),
                        "height" => h = val.parse().unwrap_or(0),
                        _ => {}
                    }
                }

                // 只保留可见且有面积的元素；跳过 Application/Window 等纯容器
                let is_container = matches!(
                    typ.as_str(),
                    "XCUIElementTypeApplication" | "XCUIElementTypeWindow" | "XCUIElementTypeOther"
                ) && name.is_empty() && label.is_empty() && value.is_empty();
                if !visible || w <= 0 || h <= 0 || is_container {
                    buf.clear();
                    continue;
                }

                // 可交互类型（uiautomator clickable 语义）
                let clickable = matches!(
                    typ.as_str(),
                    "XCUIElementTypeButton" | "XCUIElementTypeCell" | "XCUIElementTypeLink"
                        | "XCUIElementTypeSwitch" | "XCUIElementTypeTextField"
                        | "XCUIElementTypeSecureTextField" | "XCUIElementTypeSearchField"
                        | "XCUIElementTypeTabBar" | "XCUIElementTypeSegmentedControl"
                ) || accessible;

                let text = if !value.is_empty() { value.as_str() } else { label.as_str() };
                xml.push_str(&format!(
                    "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
                    xml_escape(&typ),
                    xml_escape(&name),
                    xml_escape(&label),
                    xml_escape(text),
                    clickable,
                    (x as f64 * scale) as i64,
                    (y as f64 * scale) as i64,
                    ((x + w) as f64 * scale) as i64,
                    ((y + h) as f64 * scale) as i64,
                ));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(TkeError::DeviceError(format!("XCUI XML 解析失败: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    xml.push_str("</hierarchy>\n");
    Ok(xml)
}

/// XML 属性转义
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}
