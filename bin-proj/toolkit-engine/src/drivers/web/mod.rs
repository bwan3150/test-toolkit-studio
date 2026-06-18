// WebDriver - 网页驱动（chromedriver + Chrome for Testing, W3C WebDriver 协议）
// 本文件只负责 W3C 协议对接：会话管理 + 请求 + 操作 + 信息。
// 支撑职责拆到同目录子模块：
//   infra      chromedriver 进程生命周期 / 会话文件 / Chrome 定位 / 孤儿收割（见 infra.rs）
//   normalize  DOM 可见元素 → uiautomator 风格 XML（纯转换，见 normalize.rs）
//
// 会话持久化：tke 每条指令是短命进程，浏览器必须跨进程存活——
// 会话信息（端口/session_id/pid）存 $TMPDIR/tke/web/<设备ID>.json，
// 每次命令读取复用；会话失效时自动拉起 chromedriver 并新建。
//
// 坐标系：对外统一使用截图像素坐标（与 Android 一致，标注/ocr/img 通道对齐），
// 内部按 devicePixelRatio 换算为 CSS 坐标执行操作。

mod infra;
mod normalize;

use crate::{Result, TkeError, DeviceInfo};
use crate::utils::Workarea;
use base64::Engine;
use serde::{Deserialize, Serialize};
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

    /// 列出所有标签页（含标题/URL/是否当前）。会逐个切换读取标题再切回当前，
    /// 故有一定开销——只在 fetch/采集时调用。无会话/出错返回空。
    pub fn list_tabs(&self) -> Vec<crate::drivers::TabInfo> {
        let handles: Vec<String> = match self.get("/window/handles") {
            Ok(r) => r["value"]
                .as_array()
                .map(|a| a.iter().filter_map(|h| h.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            Err(_) => return Vec::new(),
        };
        if handles.is_empty() {
            return Vec::new();
        }
        let current = self.get("/window").ok().and_then(|r| r["value"].as_str().map(String::from));
        let mut tabs = Vec::new();
        for (i, h) in handles.iter().enumerate() {
            let _ = self.post("/window", serde_json::json!({ "handle": h }));
            let title = self.get("/title").ok().and_then(|r| r["value"].as_str().map(String::from)).unwrap_or_default();
            let url = self.get("/url").ok().and_then(|r| r["value"].as_str().map(String::from)).unwrap_or_default();
            tabs.push(crate::drivers::TabInfo {
                index: i,
                title,
                url,
                active: current.as_deref() == Some(h.as_str()),
            });
        }
        // 切回原当前标签，避免列举副作用
        if let Some(c) = &current {
            let _ = self.post("/window", serde_json::json!({ "handle": c }));
        }
        tabs
    }

    /// 切换到第 index 个标签页
    pub fn switch_tab(&self, index: usize) -> Result<()> {
        let resp = self.get("/window/handles")?;
        let handles = resp["value"].as_array().cloned().unwrap_or_default();
        let h = handles
            .get(index)
            .and_then(|h| h.as_str())
            .ok_or_else(|| TkeError::DeviceError(format!("标签页序号越界: {}（共 {} 个）", index, handles.len())))?;
        self.post("/window", serde_json::json!({ "handle": h }))?;
        Ok(())
    }

    /// 新开一个标签页并导航到 url
    pub fn open_tab(&self, url: &str) -> Result<()> {
        let resp = self.post("/window/new", serde_json::json!({ "type": "tab" }))?;
        if let Some(h) = resp["value"]["handle"].as_str() {
            self.post("/window", serde_json::json!({ "handle": h }))?;
        }
        self.navigate(url)
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
        // 注入脚本提取可见元素，再归一化为 uiautomator 风格 XML（逻辑见 normalize 子模块）
        let elements = self.execute(normalize::DOM_WALK_JS, serde_json::json!([]))?;
        let xml = normalize::dom_elements_to_xml(&elements);
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
        let (mut cx, mut cy) = ((x as f64 / dpr) as i64, (y as f64 / dpr) as i64);
        // 目标若在视口外（回放时滚动位置与录制略有出入），先纵向滚动使其居中，
        // 再点击居中位置——避免 WebDriver "move target out of bounds"。
        if let Ok(dims) = self.execute("return [window.innerWidth, window.innerHeight];", serde_json::json!([])) {
            let iw = dims["value"][0].as_i64().unwrap_or(0);
            let ih = dims["value"][1].as_i64().unwrap_or(0);
            if ih > 0 && (cy < 0 || cy >= ih) {
                let dy = cy - ih / 2;
                let _ = self.execute(&format!("window.scrollBy(0, {}); return null;", dy), serde_json::json!([]));
                cy = ih / 2;
            }
            if iw > 0 {
                cx = cx.clamp(0, iw - 1);
            }
        }
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
