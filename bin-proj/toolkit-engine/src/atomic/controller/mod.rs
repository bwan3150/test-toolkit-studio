// Controller - 设备驱动分发层
// 按设备 ID 选择驱动，上层（原子方法/解释器）API 完全一致：
//   -d <android序列号>  → AdbDriver  (Android, adb)
//   -d web[:会话名]     → WebDriver  (网页, chromedriver + Chrome for Testing)
//   未来: -d wda:xxx    → iOS (WebDriverAgent)

mod adb;
mod web;

pub use adb::AdbDriver;
pub use web::WebDriver;

use crate::{Result, DeviceInfo};
use crate::utils::Workarea;

/// 设备驱动（统一入口）
pub struct Controller {
    driver: Driver,
}

enum Driver {
    Adb(AdbDriver),
    Web(WebDriver),
}

/// 判断设备 ID 是否为网页驱动（web / web:xxx）
fn is_web_device(device_id: &Option<String>) -> bool {
    device_id
        .as_deref()
        .map(|d| d == "web" || d.starts_with("web:"))
        .unwrap_or(false)
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        let driver = if is_web_device(&device_id) {
            Driver::Web(WebDriver::new(device_id.unwrap())?)
        } else {
            Driver::Adb(AdbDriver::new(device_id)?)
        };
        Ok(Self { driver })
    }

    // ===== 页面状态采集 =====

    pub async fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_ui_state(workarea).await,
            Driver::Web(d) => d.capture_ui_state(workarea),
        }
    }

    pub async fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.capture_xml_only(workarea).await,
            Driver::Web(d) => d.capture_xml_only(workarea),
        }
    }

    // ===== 操作 =====

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.tap(x, y),
            Driver::Web(d) => d.tap(x, y),
        }
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.swipe(x1, y1, x2, y2, duration_ms),
            Driver::Web(d) => d.swipe(x1, y1, x2, y2, duration_ms),
        }
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.press(x, y, duration_ms),
            Driver::Web(d) => d.press(x, y, duration_ms),
        }
    }

    pub fn input_text(&self, text: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.input_text(text),
            Driver::Web(d) => d.input_text(text),
        }
    }

    pub fn key_event(&self, key_code: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.key_event(key_code),
            Driver::Web(d) => d.key_event(key_code),
        }
    }

    pub fn back(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.back(),
            Driver::Web(d) => d.back(),
        }
    }

    pub fn home(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.home(),
            Driver::Web(d) => d.home(),
        }
    }

    /// 启动: Android = 包名+Activity；Web = URL（activity 忽略）
    pub fn launch_app(&self, package: &str, activity: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.launch_app(package, activity),
            Driver::Web(d) => d.navigate(package),
        }
    }

    /// 关闭: Android = force-stop 包；Web = 销毁浏览器会话
    pub fn stop_app(&self, package: &str) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.stop_app(package),
            Driver::Web(d) => d.close_session(),
        }
    }

    pub fn clear_input(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.clear_input(),
            Driver::Web(d) => d.clear_input(),
        }
    }

    pub fn hide_keyboard(&self) -> Result<()> {
        match &self.driver {
            Driver::Adb(d) => d.hide_keyboard(),
            Driver::Web(_) => Ok(()), // 网页无软键盘
        }
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        match &self.driver {
            Driver::Adb(d) => d.get_device_info(),
            Driver::Web(d) => d.get_device_info(),
        }
    }
}
