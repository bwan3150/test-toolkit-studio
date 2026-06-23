// Drivers 模块 - 设备/协议对接层（最底层）+ Controller 分发（管理层）
// 每个子模块只负责对接一种设备的协议，互不依赖；Controller 按设备 ID 选择驱动，
// 向上层（原子方法/解释器）暴露完全一致的 API：
//   -d <android序列号>      → adb::AdbDriver  (Android, adb)
//   -d web[:会话名]         → web::WebDriver  (网页, chromedriver + Chrome for Testing)
//   -d <iOS UDID>/wda:xxx  → wda::WdaDriver  (iOS, WebDriverAgent)

mod adb;
mod wda;
mod web;

pub use adb::AdbDriver;
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
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let driver = match crate::Platform::from_device(device_id.as_deref()) {
            crate::Platform::Web => Driver::Web(WebDriver::new(device_id.unwrap())?),
            crate::Platform::Ios => Driver::Wda(WdaDriver::new(device_id.unwrap())?),
            crate::Platform::Android => Driver::Adb(AdbDriver::new(device_id)?),
        };
        Ok(Self { driver })
    }

    // ===== 页面状态采集 =====

    pub async fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_ui_state(workarea).await,
            Driver::Web(d) => d.capture_ui_state(workarea),
            Driver::Wda(d) => d.capture_ui_state(workarea),
        }
    }

    pub async fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_xml_only(workarea).await,
            Driver::Web(d) => d.capture_xml_only(workarea),
            Driver::Wda(d) => d.capture_xml_only(workarea),
        }
    }

    // ===== 操作 =====

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.tap(x, y),
            Driver::Web(d) => d.tap(x, y),
            Driver::Wda(d) => d.tap(x, y),
        }
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Web(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Wda(d) => d.swipe(x1, y1, x2, y2, duration_ms),
        }
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.press(x, y, duration_ms),
            Driver::Web(d) => d.press(x, y, duration_ms),
            Driver::Wda(d) => d.press(x, y, duration_ms),
        }
    }

    pub fn input_text(&self, text: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.input_text(text),
            Driver::Web(d) => d.input_text(text),
            Driver::Wda(d) => d.input_text(text),
        }
    }

    pub fn key_event(&self, key_code: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.key_event(key_code),
            Driver::Web(d) => d.key_event(key_code),
            Driver::Wda(d) => d.key_event(key_code),
        }
    }

    pub fn back(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.back(),
            Driver::Web(d) => d.back(),
            Driver::Wda(d) => d.back(),
        }
    }

    pub fn home(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.home(),
            Driver::Web(d) => d.home(),
            Driver::Wda(d) => d.home(),
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
        }
    }

    /// 启动: Android = 包名+Activity；Web = URL；iOS = BundleID（activity 忽略）
    pub fn launch_app(&self, package: &str, activity: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.launch_app(package, activity),
            Driver::Web(d) => d.navigate(package),
            Driver::Wda(d) => d.launch_app(package),
        }
    }

    /// 关闭: Android = force-stop 包；Web = 销毁浏览器会话；iOS = 结束 App（空串销毁会话）
    pub fn stop_app(&self, package: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.stop_app(package),
            Driver::Web(d) => d.close_session(),
            Driver::Wda(d) => d.stop_app(package),
        }
    }

    pub fn clear_input(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.clear_input(),
            Driver::Web(d) => d.clear_input(),
            Driver::Wda(d) => d.clear_input(),
        }
    }

    pub fn hide_keyboard(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.hide_keyboard(),
            Driver::Web(_) => Ok(()), // 网页无软键盘
            Driver::Wda(d) => d.hide_keyboard(),
        }
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        match &self.driver {
            Driver::Adb(d) => d.get_device_info(),
            Driver::Web(d) => d.get_device_info(),
            Driver::Wda(d) => d.get_device_info(),
        }
    }
}
