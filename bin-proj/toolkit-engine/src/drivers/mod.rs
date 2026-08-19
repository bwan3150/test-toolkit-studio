// Drivers 模块 - 设备/协议对接层（最底层）+ Controller 分发（管理层）
// 每个子模块只负责对接一种设备的协议，互不依赖；Controller 按设备 ID 选择驱动，
// 向上层（原子方法/解释器）暴露完全一致的 API：
//   -d <android序列号>      → adb::AdbDriver  (Android, adb)
//   -d web[:会话名]         → web::WebDriver  (网页, chromedriver + Chrome for Testing)
//   -d <iOS UDID>/wda:xxx  → wda::WdaDriver  (iOS, WebDriverAgent)

mod adb;
pub mod fake;
mod wda;
mod web;

pub use adb::AdbDriver;
pub use fake::FakeDriver;
pub use wda::WdaDriver;
pub use web::WebDriver;

use crate::{Result, DeviceInfo, TkeError};
use crate::utils::Workarea;

/// 一个标签页/窗口的信息（web）
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub active: bool,
}

/// 把标签页列表格式化成一段人/AI 都能读的文字（>1 个才有意义；≤1 返回空串）
pub fn format_tabs(tabs: &[TabInfo]) -> String {
    if tabs.len() <= 1 {
        return String::new();
    }
    let items: Vec<String> = tabs
        .iter()
        .map(|t| {
            let mark = if t.active { " ◀你现在就在这页" } else { "" };
            let title = if t.title.trim().is_empty() { t.url.as_str() } else { t.title.as_str() };
            let title: String = title.chars().take(40).collect();
            format!("[{}] {}{}", t.index, title, mark)
        })
        .collect();
    format!(
        "【浏览器共 {} 个标签页】{}\n（标「◀你现在就在这页」的那个标签就是你当前所处的页面，上面那些页面元素就是它的内容。switch [序号] 用于切到**别的**标签页；switch <URL> 用新标签打开网址。）",
        tabs.len(),
        items.join("  ")
    )
}

/// 设备驱动（统一入口）
pub struct Controller {
    driver: Driver,
}

enum Driver {
    Adb(AdbDriver),
    Web(WebDriver),
    Wda(WdaDriver),
    /// 测试专用：`fake:` 前缀设备（脚本化页面 + 事件记录，见 drivers/fake.rs）
    Fake(FakeDriver),
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        // 测试专用 fake 设备：优先于平台推断（fake: 前缀不属于任何真实平台）
        if let Some(id) = device_id.as_deref() {
            if id.starts_with("fake:") {
                return Ok(Self { driver: Driver::Fake(FakeDriver::new(id.to_string())) });
            }
        }
        // 宿主机能力门禁：注定跑不通的组合（如 Windows 上测 iOS）在这里就拦下并说清原因，
        // 别让人撞进 go-ios 的底层报错里猜。这是所有设备操作的必经之路——
        // control / run / steps / harness 全都从这儿走，一处覆盖。
        let platform = crate::Platform::from_device(device_id.as_deref());
        crate::utils::capability::check(platform)?;

        let driver = match platform {
            crate::Platform::Web => Driver::Web(WebDriver::new(device_id.unwrap())?),
            // 真机与模拟器**同一套 WDA 协议代码**，只有"怎么连上"那一步不同
            // （真机 go-ios 建隧道；模拟器共享网络、直连 8100）。见 WdaDriver::new
            crate::Platform::Ios => Driver::Wda(WdaDriver::new(device_id.unwrap())?),
            crate::Platform::Android => Driver::Adb(AdbDriver::new(device_id)?),
        };
        Ok(Self { driver })
    }

    /// 这个设备有没有**软键盘**（点输入框后要等它弹出来）。
    /// 只有真实移动端有；web 是硬件键盘、fake 没有键盘——它们不该为此白等（见 control.rs 的 Input）。
    pub fn has_soft_keyboard(&self) -> bool {
        // **模拟器不在此列，是有意的**：它默认用电脑的硬件键盘，软键盘根本不弹；
        // 而且 `idb ui text` 是直接注入文本，本来也不依赖键盘弹没弹。
        // 白等一次软键盘就是白等几百毫秒 × 每个输入框（P-27 的教训）
        matches!(self.driver, Driver::Adb(_) | Driver::Wda(_))
    }

    // ===== 页面状态采集 =====

    pub async fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_ui_state(workarea).await,
            Driver::Web(d) => d.capture_ui_state(workarea),
            Driver::Wda(d) => d.capture_ui_state(workarea),
            Driver::Fake(d) => d.capture_ui_state(workarea),
        }
    }

    pub async fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_xml_only(workarea).await,
            Driver::Web(d) => d.capture_xml_only(workarea),
            Driver::Wda(d) => d.capture_xml_only(workarea),
            Driver::Fake(d) => d.capture_xml_only(workarea),
        }
    }

    // ===== 操作 =====

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.tap(x, y),
            Driver::Web(d) => d.tap(x, y),
            Driver::Wda(d) => d.tap(x, y),
            Driver::Fake(d) => d.tap(x, y),
        }
    }

    /// 悬停（web 独有）：鼠标移到坐标触发 hover，展开悬停下拉/菜单。移动端驱动会返回不支持。
    pub fn hover(&self, x: i32, y: i32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.hover(x, y),
            Driver::Web(d) => d.hover(x, y),
            Driver::Wda(d) => d.hover(x, y),
            Driver::Fake(d) => d.hover(x, y),
        }
    }

    /// 选中 `<select>` 的某一项（只有 web 有原生下拉；移动端的"下拉"多是普通列表，点就行）
    pub fn select_option(&self, x: i32, y: i32, label: &str) -> Result<String> {
        match &self.driver {
            Driver::Web(d) => d.select_option(x, y, label),
            _ => Err(crate::TkeError::InvalidArgument(
                "「选择」是 web 独有指令：移动端的下拉通常是普通列表，直接用「点击」".into(),
            )),
        }
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Web(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Wda(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Fake(d) => d.swipe(x1, y1, x2, y2, duration_ms),
        }
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.press(x, y, duration_ms),
            Driver::Web(d) => d.press(x, y, duration_ms),
            Driver::Wda(d) => d.press(x, y, duration_ms),
            Driver::Fake(d) => d.press(x, y, duration_ms),
        }
    }

    /// 返回值 = **写入的是不是密码框**（供上层给证据打码，见 utils::redact）
    pub fn input_text(&self, text: &str) -> Result<bool> {
        match &self.driver {
            // 只有 web 说得准（它看得到焦点元素的 type）；移动端靠页面结构里的
            // password 属性判断（见 TargetResolver::hits_password）
            Driver::Adb(d) => d.input_text(text).map(|_| false),
            Driver::Web(d) => d.input_text(text),
            Driver::Wda(d) => d.input_text(text).map(|_| false),
            Driver::Fake(d) => d.input_text(text).map(|_| false),
        }
    }

    pub fn key_event(&self, key_code: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.key_event(key_code),
            Driver::Web(d) => d.key_event(key_code),
            Driver::Wda(d) => d.key_event(key_code),
            Driver::Fake(d) => d.key_event(key_code),
        }
    }

    pub fn back(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.back(),
            Driver::Web(d) => d.back(),
            Driver::Wda(d) => d.back(),
            Driver::Fake(d) => d.back(),
        }
    }

    pub fn home(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.home(),
            Driver::Web(d) => d.home(),
            Driver::Wda(d) => d.home(),
            Driver::Fake(d) => d.home(),
        }
    }

    // ===== 标签页/App 切换 =====

    /// 列出标签页（仅 web 有；其它平台返回空）
    pub fn list_tabs(&self) -> Vec<TabInfo> {
        match &self.driver {
            Driver::Web(d) => d.list_tabs(),
            _ => Vec::new(),
        }
    }

    /// 切换：web=目标标签序号 或 用新标签打开 URL；移动端=把目标 App 包名切到前台。
    pub fn switch(&self, target: &str) -> Result<()> {
        let t = target.trim();
        match &self.driver {
            Driver::Web(d) => {
                if let Ok(idx) = t.parse::<usize>() {
                    d.switch_tab(idx)
                } else if t.starts_with("http://") || t.starts_with("https://") {
                    d.open_tab(t)
                } else {
                    Err(TkeError::InvalidArgument(format!(
                        "web switch 目标应为标签序号或 http(s) URL，收到: {}",
                        t
                    )))
                }
            }
            // 移动端：切到目标 App = 启动其包名（当前 App 自动退到后台）
            Driver::Adb(d) => d.launch_app(t, ""),
            Driver::Wda(d) => d.launch_app(t),
            Driver::Fake(d) => d.switch(t),
        }
    }

    /// 启动: Android = 包名+Activity；Web = URL；iOS = BundleID（activity 忽略）
    pub fn launch_app(&self, package: &str, activity: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.launch_app(package, activity),
            Driver::Web(d) => d.navigate(package),
            Driver::Wda(d) => d.launch_app(package),
            Driver::Fake(d) => d.launch_app(package, activity),
        }
    }

    /// 关闭: Android = force-stop 包；Web = 销毁浏览器会话；iOS = 结束 App（空串销毁会话）
    pub fn stop_app(&self, package: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.stop_app(package),
            Driver::Web(d) => d.close_session(),
            Driver::Wda(d) => d.stop_app(package),
            Driver::Fake(d) => d.stop_app(package),
        }
    }

    pub fn clear_input(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.clear_input(),
            Driver::Web(d) => d.clear_input(),
            Driver::Wda(d) => d.clear_input(),
            Driver::Fake(d) => d.clear_input(),
        }
    }

    pub fn hide_keyboard(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.hide_keyboard(),
            Driver::Web(_) => Ok(()), // 网页无软键盘
            Driver::Wda(d) => d.hide_keyboard(),
            Driver::Fake(d) => d.hide_keyboard(),
        }
    }

    /// 浏览器日志里的错误（console.error / 未捕获异常 / 加载失败的请求）。
    /// **只有 web 有**；读一次就清空,所以每步收一次
    pub fn console_errors(&self) -> Vec<String> {
        match &self.driver {
            Driver::Web(d) => d.console_errors(),
            _ => vec![],
        }
    }

    // ===== 浏览器专属（cookie/storage/下载/视口）=====
    // 这些**只有浏览器有**，移动端没有对应概念，所以不做成三端统一动作、
    // 也不给别的驱动编空壳实现——调错设备就如实说"只对浏览器有效"

    pub fn web_reset(&self, with_cache: bool) -> Result<Vec<String>> {
        match &self.driver {
            Driver::Web(d) => d.reset_state(with_cache),
            _ => Err(TkeError::InvalidArgument("只对浏览器有效（-d web）".into())),
        }
    }

    pub fn web_eval(&self, script: &str) -> Result<serde_json::Value> {
        match &self.driver {
            Driver::Web(d) => d.eval_js(script),
            _ => Err(TkeError::InvalidArgument("只对浏览器有效（-d web）".into())),
        }
    }

    pub fn web_viewport(&self, w: u32, h: u32) -> Result<()> {
        match &self.driver {
            Driver::Web(d) => d.set_viewport(w, h),
            _ => Err(TkeError::InvalidArgument("只对浏览器有效（-d web）".into())),
        }
    }

    pub fn web_download_dir(&self, dir: &std::path::Path) -> Result<()> {
        match &self.driver {
            Driver::Web(d) => d.set_download_dir(dir),
            _ => Err(TkeError::InvalidArgument("只对浏览器有效（-d web）".into())),
        }
    }

    pub fn web_wait_download(
        &self,
        dir: &std::path::Path,
        timeout: std::time::Duration,
    ) -> Result<Vec<std::path::PathBuf>> {
        match &self.driver {
            Driver::Web(d) => d.wait_download(dir, timeout),
            _ => Err(TkeError::InvalidArgument("只对浏览器有效（-d web）".into())),
        }
    }

    // ===== 原生对话框（alert / confirm / prompt）=====
    //
    // **只有 web 有**：这三种是浏览器画的,不在 DOM 里,fetch 一个字都采不到。
    // 移动端的系统弹窗是真元素,照常点就行,不走这条路。

    /// 当前有没有未处理的对话框；有则返回它的文字。**每步操作后都该问一句**——
    /// 它不出现在任何页面结构里,不主动探测就等于不存在(P-37)
    pub fn dialog_text(&self) -> Option<String> {
        match &self.driver {
            Driver::Web(d) => d.dialog_text(),
            _ => None,
        }
    }

    /// 点「确定」（confirm 的确认、alert 的知道了）
    pub fn dialog_accept(&self) -> Result<()> {
        match &self.driver {
            Driver::Web(d) => d.dialog_accept(),
            _ => Err(TkeError::InvalidArgument("只有浏览器有原生对话框".into())),
        }
    }

    /// 点「取消」
    pub fn dialog_dismiss(&self) -> Result<()> {
        match &self.driver {
            Driver::Web(d) => d.dialog_dismiss(),
            _ => Err(TkeError::InvalidArgument("只有浏览器有原生对话框".into())),
        }
    }

    /// 往 prompt 里填字并确定
    pub fn dialog_input(&self, text: &str) -> Result<()> {
        match &self.driver {
            Driver::Web(d) => d.dialog_input(text),
            _ => Err(TkeError::InvalidArgument("只有浏览器有原生对话框".into())),
        }
    }

    /// 「我是谁」——给人看的一句话（`iPhone 17 Pro · iOS 26.0（模拟器）` /
    /// `Pixel 7（安卓 14）` / `Chrome（无头）`）。
    ///
    /// 报告和设备列表显示这个而不是设备 ID：**那串 UUID/序列号对人没有意义**，
    /// 而"这次跑在哪台机器上"恰恰是读报告的人最先要知道的。查不到就退回设备 ID
    /// ——查询本身失败不该让整件事失败。
    pub fn describe(&self) -> String {
        match &self.driver {
            Driver::Adb(d) => d.describe(),
            Driver::Web(d) => d.describe(),
            Driver::Wda(d) => d.describe(),
            Driver::Fake(d) => d.describe(),
        }
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        match &self.driver {
            Driver::Adb(d) => d.get_device_info(),
            Driver::Web(d) => d.get_device_info(),
            Driver::Wda(d) => d.get_device_info(),
            Driver::Fake(d) => d.get_device_info(),
        }
    }
}
