// WebDriver - 网页驱动（chromedriver + Chrome for Testing, W3C WebDriver 协议）
//
// 会话持久化：tke 每条指令是短命进程，浏览器必须跨进程存活——
// 会话信息（端口/session_id/pid）存 $TMPDIR/tke/web/<设备ID>.json，
// 每次命令读取复用；会话失效时自动拉起 chromedriver 并新建。
//
// 坐标系：对外统一使用截图像素坐标（与 Android 一致，标注/ocr/img 通道对齐），
// 内部按 devicePixelRatio 换算为 CSS 坐标执行操作。

use crate::{Result, TkeError, DeviceInfo};
use crate::utils::Workarea;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// 持久化的会话信息
#[derive(Debug, Serialize, Deserialize)]
struct SessionInfo {
    port: u16,
    session_id: String,
    driver_pid: u32,
}

/// 活动连接 (chromedriver 地址 + 会话ID)
#[derive(Debug, Clone)]
struct Conn {
    base: String,
    session_id: String,
}

/// 网页驱动（惰性会话：首次操作时建立，关闭后下次操作自动重建）
pub struct WebDriver {
    device_id: String,
    conn: std::sync::Mutex<Option<Conn>>,
}

impl WebDriver {
    pub fn new(device_id: String) -> Result<Self> {
        Ok(Self { device_id, conn: std::sync::Mutex::new(None) })
    }

    /// 取已有连接（进程内 → 持久化会话），不存在则报错
    /// 除"启动/导航"外的所有操作都用这个——避免点击/截图等操作凭空拉起空浏览器
    fn ensure_existing(&self) -> Result<Conn> {
        let mut guard = self.conn.lock().unwrap();

        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }

        // 尝试复用持久化的会话
        if let Some(info) = Self::load_session(&self.device_id) {
            let base = format!("http://127.0.0.1:{}", info.port);
            if Self::session_alive(&base, &info.session_id) {
                let conn = Conn { base, session_id: info.session_id };
                *guard = Some(conn.clone());
                return Ok(conn);
            }
        }

        Err(TkeError::DeviceError(
            "无活动浏览器会话，请先执行 启动 [URL] 或 control launch <URL>".to_string(),
        ))
    }

    /// 取活动连接，不存在则新建会话（仅供 导航/启动 使用）
    fn ensure_create(&self) -> Result<Conn> {
        if let Ok(conn) = self.ensure_existing() {
            return Ok(conn);
        }

        let conn = Self::start_new_session(&self.device_id)?;
        *self.conn.lock().unwrap() = Some(conn.clone());
        Ok(conn)
    }

    // ===== 会话管理 =====

    fn session_file(device_id: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tke").join("web");
        let _ = std::fs::create_dir_all(&dir);
        let key: String = device_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        dir.join(format!("{}.json", key))
    }

    fn load_session(device_id: &str) -> Option<SessionInfo> {
        let content = std::fs::read_to_string(Self::session_file(device_id)).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn session_alive(base: &str, session_id: &str) -> bool {
        ureq::get(&format!("{}/session/{}/url", base, session_id))
            .timeout(Duration::from_millis(1500))
            .call()
            .is_ok()
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
    fn reap_orphans(device_id: &str) {
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
    fn start_new_session(device_id: &str) -> Result<Conn> {
        // 先收割上次遗留的孤儿进程/profile
        Self::reap_orphans(device_id);

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or_else(|| TkeError::InvalidArgument("无法获取 tke 所在目录".to_string()))?;

        // chromedriver 必须与 tke 同目录
        let chromedriver = exe_dir.join("chromedriver");
        if !chromedriver.exists() {
            return Err(TkeError::InvalidArgument(
                "chromedriver 可执行文件缺失或不完整：请将其放在与 tke 相同的目录下".to_string(),
            ));
        }

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
        // 只保留必要的环境变量
        for key in ["PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG"] {
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
        let mut chrome_options = serde_json::json!({
            "args": [
                "--window-size=1280,900",
                "--force-device-scale-factor=1",
                "--disable-infobars",
                "--no-first-run",
                "--no-default-browser-check",
                format!("--user-data-dir={}", profile_dir)
            ]
        });
        if let Some(cft) = Self::find_chrome_binary(&exe_dir) {
            chrome_options["binary"] = serde_json::json!(cft.to_string_lossy());
        }

        let session_req = ureq::post(&format!("{}/session", base))
            .timeout(Duration::from_secs(90))
            .send_json(serde_json::json!({
                "capabilities": {
                    "alwaysMatch": {
                        "browserName": "chrome",
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
                return Err(TkeError::DeviceError(format!(
                    "创建浏览器会话失败: {} (日志: {})", detail, log_path.display()
                )));
            }
        };

        let session_id = resp["value"]["sessionId"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError(format!("会话响应缺少 sessionId: {}", resp)))?
            .to_string();

        // 持久化会话信息
        let info = SessionInfo { port, session_id: session_id.clone(), driver_pid };
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
        const MAC_APP_PATH: &str =
            "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing";

        let mut candidates = vec![exe_dir.join(MAC_APP_PATH)];
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("Library/Application Support/tke").join(MAC_APP_PATH));
        }

        candidates.into_iter().find(|p| p.exists())
    }

    /// 销毁会话：关闭浏览器 + 结束 chromedriver + 删除会话文件
    /// 进程内连接同时清空，后续操作（如脚本中 关闭→启动）会自动重建新会话
    pub fn close_session(&self) -> Result<()> {
        if let Some(conn) = self.conn.lock().unwrap().take() {
            let _ = ureq::delete(&format!("{}/session/{}", conn.base, conn.session_id))
                .timeout(Duration::from_secs(10))
                .call();
        }

        if let Some(info) = Self::load_session(&self.device_id) {
            // 结束 chromedriver 进程
            let _ = Command::new("kill").arg(info.driver_pid.to_string()).output();
        }
        let _ = std::fs::remove_file(Self::session_file(&self.device_id));
        // 兜底收割（处理 DELETE /session 未生效的情况）
        Self::reap_orphans(&self.device_id);
        Ok(())
    }

    // ===== W3C 协议基础 =====

    fn endpoint(&self, path: &str) -> Result<String> {
        let conn = self.ensure_existing()?;
        Ok(format!("{}/session/{}{}", conn.base, conn.session_id, path))
    }

    fn get(&self, path: &str) -> Result<serde_json::Value> {
        ureq::get(&self.endpoint(path)?)
            .timeout(Duration::from_secs(30))
            .call()
            .map_err(|e| TkeError::DeviceError(format!("WebDriver 请求失败 {}: {}", path, e)))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("WebDriver 响应解析失败: {}", e)))
    }

    fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        match ureq::post(&self.endpoint(path)?)
            .timeout(Duration::from_secs(60))
            .send_json(body)
        {
            Ok(resp) => resp
                .into_json()
                .map_err(|e| TkeError::DeviceError(format!("WebDriver 响应解析失败: {}", e))),
            // 协议错误（4xx/5xx）：提取 chromedriver 返回的具体错误信息
            Err(ureq::Error::Status(code, resp)) => {
                let detail = resp
                    .into_json::<serde_json::Value>()
                    .ok()
                    .and_then(|v| v["value"]["message"].as_str().map(|s| {
                        // 只取首行（后面是冗长的 stacktrace）
                        s.lines().next().unwrap_or(s).to_string()
                    }))
                    .unwrap_or_else(|| format!("status {}", code));
                Err(TkeError::DeviceError(format!("WebDriver {}: {}", path, detail)))
            }
            Err(e) => Err(TkeError::DeviceError(format!("WebDriver 请求失败 {}: {}", path, e))),
        }
    }

    /// 执行同步 JS，返回结果
    fn execute(&self, script: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let resp = self.post("/execute/sync", serde_json::json!({
            "script": script,
            "args": args,
        }))?;
        Ok(resp["value"].clone())
    }

    /// 当前 devicePixelRatio（截图像素 ↔ CSS 坐标换算）
    /// 带重试；失败直接报错（静默回退 1.0 会导致坐标错位、操作打偏）
    fn device_pixel_ratio(&self) -> Result<f64> {
        let mut last_err = None;
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(300));
            }
            match self.execute("return window.devicePixelRatio || 1;", serde_json::json!([])) {
                Ok(v) => {
                    if let Some(dpr) = v.as_f64() {
                        return Ok(dpr);
                    }
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            TkeError::DeviceError("无法获取页面 devicePixelRatio".to_string())
        }))
    }

    // ===== 页面状态采集 =====

    /// 采集截图 + 归一化 DOM（写入工作区）
    pub fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        self.capture_screenshot(workarea)?;
        self.capture_dom_xml(workarea)?;
        Ok(())
    }

    pub fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        self.capture_dom_xml(workarea)
    }

    fn capture_screenshot(&self, workarea: &Workarea) -> Result<()> {
        let resp = self.get("/screenshot")?;
        let b64 = resp["value"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError("截图响应无数据".to_string()))?;
        let png = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| TkeError::DeviceError(format!("截图解码失败: {}", e)))?;
        std::fs::write(workarea.screenshot_path(), png).map_err(TkeError::IoError)?;
        Ok(())
    }

    /// 提取可见元素并归一化为 uiautomator 风格 XML：
    /// 网页元素与 App 元素进入同一套解析/识别/标注体系
    /// （resource-id=DOM id, content-desc=aria-label, bounds=截图像素坐标）
    fn capture_dom_xml(&self, workarea: &Workarea) -> Result<()> {
        // 一次 JS 调用提取全部可见元素
        let script = r#"
const dpr = window.devicePixelRatio || 1;
const out = [];
const walk = (el) => {
  for (const child of el.children) {
    const r = child.getBoundingClientRect();
    const style = getComputedStyle(child);
    const visible = r.width > 0 && r.height > 0 &&
      style.visibility !== 'hidden' && style.display !== 'none' &&
      r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth;
    if (visible) {
      // 仅取直接文本（不含子元素文本），避免父容器吞掉所有文字
      let ownText = '';
      for (const n of child.childNodes) {
        if (n.nodeType === 3) ownText += n.textContent;
      }
      ownText = ownText.trim().slice(0, 120);
      // 输入框取 placeholder/value 兜底
      if (!ownText && (child.tagName === 'INPUT' || child.tagName === 'TEXTAREA')) {
        ownText = (child.value || child.placeholder || '').slice(0, 120);
      }
      const clickable = ['A','BUTTON','SELECT'].includes(child.tagName) ||
        ['INPUT','TEXTAREA'].includes(child.tagName) ||
        child.onclick != null || style.cursor === 'pointer' ||
        child.getAttribute('role') === 'button';
      out.push({
        tag: child.tagName.toLowerCase(),
        id: child.id || '',
        aria: child.getAttribute('aria-label') || '',
        text: ownText,
        clickable: clickable,
        x1: Math.round(r.left * dpr), y1: Math.round(r.top * dpr),
        x2: Math.round(r.right * dpr), y2: Math.round(r.bottom * dpr),
      });
    }
    walk(child);
  }
};
walk(document.body);
return out;
"#;
        let elements = self.execute(script, serde_json::json!([]))?;
        let empty = vec![];
        let list = elements.as_array().unwrap_or(&empty);

        // 归一化为 uiautomator 风格 XML
        let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
        for e in list {
            xml.push_str(&format!(
                "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
                xml_escape(e["tag"].as_str().unwrap_or("")),
                xml_escape(e["id"].as_str().unwrap_or("")),
                xml_escape(e["aria"].as_str().unwrap_or("")),
                xml_escape(e["text"].as_str().unwrap_or("")),
                e["clickable"].as_bool().unwrap_or(false),
                e["x1"].as_i64().unwrap_or(0),
                e["y1"].as_i64().unwrap_or(0),
                e["x2"].as_i64().unwrap_or(0),
                e["y2"].as_i64().unwrap_or(0),
            ));
        }
        xml.push_str("</hierarchy>\n");

        std::fs::write(workarea.ui_tree_path(), xml).map_err(TkeError::IoError)?;
        Ok(())
    }

    // ===== 操作（入参为截图像素坐标） =====

    pub fn navigate(&self, url: &str) -> Result<()> {
        // 启动/导航是唯一允许创建会话的入口
        self.ensure_create()?;

        // 无协议前缀自动补 https://
        let url = if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("about:") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };
        self.post("/url", serde_json::json!({ "url": url }))?;
        Ok(())
    }

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        let dpr = self.device_pixel_ratio()?;
        let (cx, cy) = ((x as f64 / dpr) as i64, (y as f64 / dpr) as i64);
        self.pointer_actions(serde_json::json!([
            { "type": "pointerMove", "duration": 0, "x": cx, "y": cy },
            { "type": "pointerDown", "button": 0 },
            { "type": "pointerUp", "button": 0 }
        ]))
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        let dpr = self.device_pixel_ratio()?;
        let (cx, cy) = ((x as f64 / dpr) as i64, (y as f64 / dpr) as i64);
        self.pointer_actions(serde_json::json!([
            { "type": "pointerMove", "duration": 0, "x": cx, "y": cy },
            { "type": "pointerDown", "button": 0 },
            { "type": "pause", "duration": duration_ms },
            { "type": "pointerUp", "button": 0 }
        ]))
    }

    /// 滑动：网页语义 = 在起点处滚轮滚动（移动端拖拽手势在桌面浏览器中是滚轮）
    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, _duration_ms: u32) -> Result<()> {
        let dpr = self.device_pixel_ratio()?;
        let (cx, cy) = ((x1 as f64 / dpr) as i64, (y1 as f64 / dpr) as i64);
        let dx = ((x1 - x2) as f64 / dpr) as i64;
        let dy = ((y1 - y2) as f64 / dpr) as i64;
        self.post("/actions", serde_json::json!({
            "actions": [{
                "type": "wheel",
                "id": "wheel1",
                "actions": [
                    { "type": "scroll", "x": cx, "y": cy, "deltaX": dx, "deltaY": dy, "duration": 200 }
                ]
            }]
        }))?;
        Ok(())
    }

    fn pointer_actions(&self, actions: serde_json::Value) -> Result<()> {
        self.post("/actions", serde_json::json!({
            "actions": [{
                "type": "pointer",
                "id": "mouse1",
                "parameters": { "pointerType": "mouse" },
                "actions": actions
            }]
        }))?;
        Ok(())
    }

    /// 输入文本到当前聚焦元素（原生 setter + input 事件，兼容 React/Vue 受控组件）
    pub fn input_text(&self, text: &str) -> Result<()> {
        let script = r#"
const el = document.activeElement;
const text = arguments[0];
if (!el) return false;
if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
  const proto = el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
  setter.call(el, (el.value || '') + text);
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  return true;
}
if (el.isContentEditable) {
  el.textContent += text;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  return true;
}
return false;
"#;
        let ok = self.execute(script, serde_json::json!([text]))?;
        if ok.as_bool() == Some(true) {
            Ok(())
        } else {
            Err(TkeError::DeviceError(
                "输入失败：当前没有聚焦的输入框（请先点击输入框）".to_string(),
            ))
        }
    }

    /// 清空当前聚焦输入框
    pub fn clear_input(&self) -> Result<()> {
        let script = r#"
const el = document.activeElement;
if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) {
  const proto = el.tagName === 'INPUT' ? window.HTMLInputElement.prototype : window.HTMLTextAreaElement.prototype;
  Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, '');
  el.dispatchEvent(new Event('input', { bubbles: true }));
  return true;
}
return false;
"#;
        self.execute(script, serde_json::json!([]))?;
        Ok(())
    }

    /// 按键：回车/Tab/Esc 常用键映射，其余忽略
    pub fn key_event(&self, key_code: &str) -> Result<()> {
        let key = match key_code {
            "KEYCODE_ENTER" | "ENTER" => "\u{E007}",
            "KEYCODE_TAB" | "TAB" => "\u{E004}",
            "KEYCODE_ESCAPE" | "ESC" => "\u{E00C}",
            "KEYCODE_DEL" | "BACKSPACE" => "\u{E003}",
            "KEYCODE_BACK" => return self.back(),
            _ => return Ok(()),
        };
        self.post("/actions", serde_json::json!({
            "actions": [{
                "type": "key",
                "id": "kb1",
                "actions": [
                    { "type": "keyDown", "value": key },
                    { "type": "keyUp", "value": key }
                ]
            }]
        }))?;
        Ok(())
    }

    /// 返回 = 浏览器后退
    pub fn back(&self) -> Result<()> {
        self.post("/back", serde_json::json!({}))?;
        Ok(())
    }

    /// 主页 = 空白页
    pub fn home(&self) -> Result<()> {
        self.navigate("about:blank")
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        let (w, h) = self
            .execute(
                "const d=window.devicePixelRatio||1; return [Math.round(innerWidth*d), Math.round(innerHeight*d)];",
                serde_json::json!([]),
            )
            .ok()
            .and_then(|v| {
                let a = v.as_array()?;
                Some((a[0].as_u64()? as u32, a[1].as_u64()? as u32))
            })
            .unwrap_or((0, 0));

        Ok(DeviceInfo {
            id: self.device_id.clone(),
            model: Some("Chrome for Testing".to_string()),
            manufacturer: Some("web".to_string()),
            android_version: None,
            screen_width: w,
            screen_height: h,
            hardware: None,
            battery: None,
            network: None,
        })
    }
}

/// XML 属性转义
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}
