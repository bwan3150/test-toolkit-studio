// WdaDriver - iOS 驱动（WebDriverAgent, W3C WebDriver 系协议）
// 本文件只负责 WDA HTTP 协议对接：会话管理 + 请求 + 操作 + 信息。
// 支撑职责拆到同目录子模块：
//   infra      go-ios 进程/隧道/端口转发（基础设施自动拉起，见 infra.rs）
//   normalize  XCUI 元素树 → uiautomator 风格 XML（纯转换，见 normalize.rs）
//
// 会话持久化：与 web 驱动同款——tke 每条指令是短命进程，
// 转发端口/会话/后台进程信息存 $TMPDIR/tke/ios/<udid>.json，每次命令读取复用。
//
// 坐标系：对外统一使用截图像素坐标（与 Android/Web 一致），
// WDA 协议使用逻辑点（point），内部按 scale（视网膜倍率，如 iPhone 12 = 3）换算。

mod infra;
mod normalize;

use crate::{Result, TkeError, DeviceInfo};
use crate::utils::Workarea;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// 持久化的转发/会话信息（字段对 infra 子模块可见）
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
    /// 设备 UDID（已去除 wda: / sim: 前缀）
    udid: String,
    /// 模拟器（`-d sim:<udid>`）——不走 go-ios，直连 127.0.0.1:8100
    simulator: bool,
    conn: std::sync::Mutex<Option<Conn>>,
}

impl WdaDriver {
    pub fn new(device_id: String) -> Result<Self> {
        // `sim:<udid>` = 模拟器。它和真机是**两条完全不同的接入路**：
        // 真机要 go-ios 建隧道 + USB 端口转发；模拟器与主机共享网络，WDA 就在
        // 127.0.0.1:8100 上，一步都不用绕。WDA 协议本身（点击/采集/截图）两边一模一样。
        let simulator = device_id.starts_with("sim:");
        let udid = device_id
            .strip_prefix("sim:")
            .or_else(|| device_id.strip_prefix("wda:"))
            .unwrap_or(&device_id)
            .to_string();
        // 前缀后面空着（`-d sim:`，多半是 shell 变量没展开）——**在这儿拦下**。
        // 不拦的话会一路传到 simctl，回一句光秃秃的 `Invalid device:`，
        // 没人看得出是自己的 $UDID 是空的（实测撞过）
        if udid.trim().is_empty() {
            return Err(TkeError::InvalidArgument(format!(
                "设备 ID 是空的（`-d {}` 后面什么都没有）——是不是 shell 变量没展开？\n\
                 {}用 `tke device list` 查一个填进去",
                device_id,
                "\u{3000}"
            )));
        }
        Ok(Self { udid, simulator, conn: std::sync::Mutex::new(None) })
    }

    /// 这是模拟器吗（决定走不走 go-ios）
    pub(super) fn is_simulator(&self) -> bool {
        self.simulator
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

        // 没有会话就**附着当前前台 App** 建一个。
        // 「App 已经开着，我只想看看这一页」是最常见的诉求，不该逼人先 `启动 [BundleID]`
        // ——那会重启 App，把要看的现场毁掉。
        drop(guard);
        if let Ok(conn) = self.attach_foreground(&base, state) {
            return Ok(conn);
        }

        Err(TkeError::DeviceError(
            "无活动 WDA 会话，请先执行 启动 [BundleID] 或 control launch <BundleID>".to_string(),
        ))
    }

    /// 取活动会话，不存在则新建（仅供 启动 使用）
    /// bundle_id: 新建会话时随会话拉起的 App
    /// 附着到**当前前台 App** 建会话（不带 bundleId，WDA 会挂到活动 App 上）。
    ///
    /// ⚠️ 附上之后要**确认附到了谁**：模拟器上第一次拉起 WDA runner 时，
    /// 它自己会被带到前台、把用户的 App 挤到后台——这时附着成功、`/status` 也正常，
    /// 但采到的是 WDA 那个空白测试界面。不检查的话就是一份「页面上什么都没有」的
    /// 假结论（跟这两天那一族一模一样）。
    fn attach_foreground(&self, base: &str, mut state: WdaState) -> Result<Conn> {
        let resp: serde_json::Value = ureq::post(&format!("{}/session", base))
            .timeout(Duration::from_secs(30))
            .send_json(serde_json::json!({ "capabilities": { "alwaysMatch": {} } }))
            .map_err(|e| TkeError::DeviceError(format!("附着当前 App 失败: {}", e)))?
            .into_json()
            .map_err(|e| TkeError::DeviceError(format!("WDA 会话响应解析失败: {}", e)))?;
        let session_id = resp["sessionId"]
            .as_str()
            .or_else(|| resp["value"]["sessionId"].as_str())
            .ok_or_else(|| TkeError::DeviceError("附着后没拿到 sessionId".into()))?
            .to_string();

        // 附到谁身上了
        let active = ureq::get(&format!("{}/wda/activeAppInfo", base))
            .timeout(Duration::from_secs(10))
            .call()
            .ok()
            .and_then(|r| r.into_json::<serde_json::Value>().ok())
            .and_then(|v| v["value"]["bundleId"].as_str().map(String::from))
            .unwrap_or_default();
        // **附到"明显不是被测对象"的东西上就报错**。第一次拉起 WDA runner 会把
        // 用户的 App 挤走,落点有两种:WDA 自己、或者桌面(SpringBoard)——
        // 两种都附得上、`/status` 也正常,采到的却是一屏跟被测功能毫无关系的东西。
        // 只判 WDA 不够(实测第二种更常见:采到一堆 Fitness/通讯录/文件 图标,
        // 然后"找不到那个按钮"超时 20 秒)。
        let bystander = match active.as_str() {
            b if b.starts_with("com.facebook.WebDriverAgentRunner") => Some("WebDriverAgent 自己"),
            "com.apple.springboard" | "com.apple.SpringBoard" => Some("桌面（主屏幕）"),
            _ => None,
        };
        if let Some(what) = bystander {
            // `\u{3000}` 是全角空格（对齐用）。写成常量而不是直接嵌进 format! ——
            // 它的花括号会跟占位符打架，而直接写那个字符又会在字符串续行后触发编译警告
            const IDT: &str = "\u{3000}";
            return Err(TkeError::DeviceError(format!(
                "现在前台是{what}，不是你要测的 App。\n\
                 {IDT}第一次把 WebDriverAgent 拉起来时会挤掉前台 App——\
                 用 `启动 [\"你的BundleID\"]` 把它拉回来。\n\
                 {IDT}之后 WDA 一直跑着，不会再挤了\
                 （包名用 `xcrun simctl listapps <UDID>` 找）"
            )));
        }

        let scale = Self::fetch_scale(base, &session_id)?;
        state.session_id = Some(session_id.clone());
        state.scale = Some(scale);
        Self::save_state(&self.udid, &state);
        let conn = Conn { base: base.to_string(), session_id, scale };
        *self.conn.lock().unwrap() = Some(conn.clone());
        Ok(conn)
    }

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

    /// 抓取 XCUI 元素树并归一化为 uiautomator 风格 XML（归一化逻辑见 normalize 子模块）
    fn capture_xcui_xml(&self, workarea: &Workarea) -> Result<()> {
        let conn = self.ensure_existing()?;
        let resp = self.get("/source")?;
        let source = resp["value"]
            .as_str()
            .ok_or_else(|| TkeError::DeviceError("UI 结构响应无数据".to_string()))?;

        // WDA 给的 XCUI 原文先存一份：normalize 会做坐标换算与筛选，
        // 对得上原文才知道某个控件是被筛掉了还是压根没采到（web/adb 同理）
        let _ = std::fs::write(workarea.raw_page_path("xml"), source);

        let xml = normalize::normalize_xcui_xml(source, conn.scale)?;
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

    /// 悬停：移动端无"鼠标悬停"概念，不支持（hover 为 web 独有）
    pub fn hover(&self, _x: i32, _y: i32) -> Result<()> {
        Err(TkeError::InvalidArgument("iOS 不支持悬停(hover 为 web 独有)".to_string()))
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
            // ⚠️ 认不出的键**必须报错**。这里原先是 `_ => Ok(())`——`按键 ["TAB"]`
            // 在 iOS 上什么都不做,却报成功。这种"成功了但没发生"最难查:人会以为
            // 焦点已经移走了,接着往下写,错在后面几步才暴露出来(INV-9)
            other => Err(TkeError::InvalidArgument(format!(
                "iOS 上没有这个按键：{}（WDA 只认 ENTER 和 BACK；其它键请直接点目标元素）",
                other
            ))),
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

    /// 启动运行环境。**模拟器才有这回事**——真机开着就是开着，插上就能用。
    ///
    /// `headed` 对模拟器意味着"要不要把 Simulator 窗口显示出来"：
    /// 不显示也能测（WDA 走 HTTP，不依赖窗口），但人看不到画面。默认显示。
    pub fn boot(&self, headed: Option<bool>) -> Result<()> {
        if !self.simulator {
            return Err(TkeError::InvalidArgument(
                "真机不需要 boot —— 插上、解锁就能用。要装/拉起 WebDriverAgent 见 tke doctor".into(),
            ));
        }
        // 已经在跑就什么都不做（幂等）——boot 写在脚本第一行，每次跑都会执行
        let out = std::process::Command::new("xcrun")
            .args(["simctl", "boot", &self.udid])
            .output()
            .map_err(|e| TkeError::DeviceError(format!("simctl boot 失败: {}", e)))?;
        let err = String::from_utf8_lossy(&out.stderr);
        // "Unable to boot device in current state: Booted" = 已经开着，不是错误
        if !out.status.success() && !err.contains("current state: Booted") {
            return Err(TkeError::DeviceError(format!("启动模拟器失败: {}", err.trim())));
        }
        if headed.unwrap_or(true) {
            let _ = std::process::Command::new("open").args(["-a", "Simulator"]).output();
        }
        // 等到真的可用为止。**别只等 boot 命令返回**：它返回时系统还在起，
        // 紧接着 simctl install 会失败
        let _ = std::process::Command::new("xcrun")
            .args(["simctl", "bootstatus", &self.udid, "-b"])
            .output();
        Ok(())
    }

    /// 关掉运行环境（模拟器关机）。真机没有这回事
    pub fn shutdown(&self) -> Result<()> {
        if !self.simulator {
            return Err(TkeError::InvalidArgument(
                "真机没法从这儿关机 —— 要结束 WDA 会话用 `关闭`（空参）".into(),
            ));
        }
        // 关机前先把会话文件清掉：下次 boot 起来的是新系统，旧 session_id 必然失效，
        // 留着只会让下一条命令先撞一次"会话已死"再重建
        let _ = std::fs::remove_file(Self::state_file(&self.udid));
        *self.conn.lock().unwrap() = None;
        let out = std::process::Command::new("xcrun")
            .args(["simctl", "shutdown", &self.udid])
            .output()
            .map_err(|e| TkeError::DeviceError(format!("simctl shutdown 失败: {}", e)))?;
        let err = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() && !err.contains("current state: Shutdown") {
            return Err(TkeError::DeviceError(format!("关闭模拟器失败: {}", err.trim())));
        }
        Ok(())
    }

    /// 「我是谁」——给人看的一句话（`iPhone 17 Pro（模拟器）`）。
    ///
    /// 报告和设备列表里显示这个，而不是那串 UUID：**UUID 对人没有任何意义**，
    /// 而"这次跑在哪台机器上"恰恰是读报告的人最先要知道的。
    pub fn describe(&self) -> String {
        if self.simulator {
            let name = self.simctl_name().unwrap_or_else(|| "iOS 模拟器".into());
            return format!("{}（模拟器）", name);
        }
        // 真机：WDA 的 /status 里有机型与系统版本
        let info = self
            .ensure_existing()
            .ok()
            .and_then(|c| {
                ureq::get(&format!("{}/status", c.base))
                    .timeout(Duration::from_secs(5))
                    .call()
                    .ok()?
                    .into_json::<serde_json::Value>()
                    .ok()
            })
            .and_then(|v| {
                let os = v["value"]["os"].clone();
                let name = os["name"].as_str().unwrap_or("iOS").to_string();
                let ver = os["version"].as_str().unwrap_or_default().to_string();
                Some(if ver.is_empty() { name } else { format!("{} {}", name, ver) })
            })
            .unwrap_or_else(|| "iOS".into());
        format!("{}（真机）", info)
    }

    /// 模拟器的设备信息：机型/系统版本走 `simctl list`，屏幕尺寸靠截一张图量。
    /// **全程不碰 WDA**
    fn simulator_info(&self) -> Result<DeviceInfo> {
        let (w, h) = self.simulator_screen().unwrap_or((0, 0));
        Ok(DeviceInfo {
            id: format!("sim:{}", self.udid),
            model: Some(self.simctl_name().unwrap_or_else(|| "iOS 模拟器".into())),
            manufacturer: Some("Apple".to_string()),
            android_version: None,
            screen_width: w,
            screen_height: h,
            hardware: None,
            battery: None,
            network: None,
        })
    }

    /// 截一张图量屏幕像素。模拟器没启动时截不到——那时给 0，
    /// 但机型信息照常给（**没启动也该看得到这台是什么**）
    fn simulator_screen(&self) -> Option<(u32, u32)> {
        let tmp = std::env::temp_dir().join(format!("tke-siminfo-{}.png", self.udid));
        let out = std::process::Command::new("xcrun")
            .args(["simctl", "io", &self.udid, "screenshot", &tmp.to_string_lossy()])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        // PNG 的 IHDR：8 字节签名 + 4 长度 + 4 类型，宽高在 16..24
        let bytes = std::fs::read(&tmp).ok()?;
        let _ = std::fs::remove_file(&tmp);
        if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
            return None;
        }
        Some((
            u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        ))
    }

    /// 从 `simctl list` 里查这台模拟器的机型名（"iPhone 17 Pro"）
    fn simctl_name(&self) -> Option<String> {
        let out = std::process::Command::new("xcrun")
            .args(["simctl", "list", "devices", "--json"])
            .output()
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        for (rt, list) in v["devices"].as_object()? {
            for d in list.as_array()? {
                if d["udid"].as_str() == Some(&self.udid) {
                    let ver = rt.rsplit('.').next().unwrap_or("").replacen('-', " ", 1).replace('-', ".");
                    let name = d["name"].as_str().unwrap_or("iPhone");
                    return Some(if ver.is_empty() { name.into() } else { format!("{} · {}", name, ver) });
                }
            }
        }
        None
    }

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        // 模拟器：**别为了看一眼机型就把 WDA 拉起来**。机型和系统版本 simctl 直接有，
        // 屏幕尺寸截一张图就知道（截图不需要 WDA）——早先这里一律走 WDA，于是
        // `tke device info -d sim:…` 回一句「无活动 WDA 会话，请先执行 启动 [BundleID]」，
        // 而用户只是想看看这台是什么
        if self.simulator {
            return self.simulator_info();
        }
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
