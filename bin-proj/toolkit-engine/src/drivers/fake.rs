// 【Fake 驱动】测试专用（仿 Maestro 的 FakeDriver）：设备 id 以 `fake:` 开头即启用。
//
// 让驱动循环 / 感知 / 执行链路可以**无设备、无网络**地在 CI 里测：
//   · 页面 = 脚本化的 uiautomator XML 序列（install 装配）；capture 把当前页写进工作区，
//     fetcher/感知层照常解析——测试走的是真实解析路径，不是 mock；
//   · 动作 = 事件记录（tap/swipe/input…原样记下），测试跑完 assert 事件序列；
//   · 页面推进：tap/switch 前进一页（到最后一页停住）、back 退一页、launch 回到第 0 页，
//     其余动作只记录不推进——测试按此设计页面脚本。
//
// 状态放进程级注册表（Controller 每次调用都新建实例，状态必须在实例外）；
// 测试用不同的 fake:<名字> 隔离互不干扰，结束 remove() 清理。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::utils::Workarea;
use crate::{DeviceInfo, Result, TkeError};

/// 一台 fake 设备的状态：页面脚本 + 当前页 + 事件记录
#[derive(Default)]
struct FakeState {
    /// 页面序列（uiautomator XML 全文）
    pages: Vec<String>,
    /// 当前页下标
    current: usize,
    /// 动作事件记录（人类可读，按发生顺序）
    events: Vec<String>,
}

static REGISTRY: OnceLock<Mutex<HashMap<String, FakeState>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, FakeState>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 测试装配：为 `fake:<名字>` 设备安装页面脚本（覆盖旧状态）。
pub fn install(device: &str, pages: Vec<String>) {
    let mut reg = registry().lock().expect("fake registry 锁中毒");
    reg.insert(device.to_string(), FakeState { pages, current: 0, events: Vec::new() });
}

/// 取某 fake 设备的事件记录（测试断言用）。
pub fn events(device: &str) -> Vec<String> {
    registry()
        .lock()
        .expect("fake registry 锁中毒")
        .get(device)
        .map(|s| s.events.clone())
        .unwrap_or_default()
}

/// 移除某 fake 设备（测试收尾）。
pub fn remove(device: &str) {
    registry().lock().expect("fake registry 锁中毒").remove(device);
}

/// 便捷：把若干 `<node …/>` 行包成一页 uiautomator XML。
pub fn page(nodes: &[&str]) -> String {
    format!(
        "<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n{}\n</hierarchy>\n",
        nodes.join("\n")
    )
}

/// 便捷：一个带文字/边界的可点击节点。
pub fn node(text: &str, x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    format!(
        "  <node index=\"0\" text=\"{}\" class=\"android.widget.Button\" clickable=\"true\" bounds=\"[{},{}][{},{}]\"/>",
        text, x1, y1, x2, y2
    )
}

/// fake 驱动实例（无自有状态，一切经注册表）。
pub struct FakeDriver {
    id: String,
}

impl FakeDriver {
    /// 假设备没有"环境"要起——但**要成功返回**，否则脚本里的 `启动环境` 在
    /// 无设备测试层里会挂掉，而那正是这套测试层存在的意义
    pub fn boot(&self, _headed: Option<bool>) -> Result<()> {
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// 「我是谁」——测试用的假设备
    pub fn describe(&self) -> String {
        "假设备（测试用）".to_string()
    }

    pub fn new(id: String) -> Self {
        Self { id }
    }

    fn with<R>(&self, f: impl FnOnce(&mut FakeState) -> R) -> Result<R> {
        let mut reg = registry().lock().map_err(|_| TkeError::DeviceError("fake registry 锁中毒".into()))?;
        let st = reg
            .get_mut(&self.id)
            .ok_or_else(|| TkeError::DeviceError(format!("fake 设备 {} 未装配（先调 drivers::fake::install）", self.id)))?;
        Ok(f(st))
    }

    /// 记录事件；`step`：+1 前进 / -1 后退 / 0 原地 / RESET 回到第 0 页
    fn record(&self, ev: String, step: i32) -> Result<()> {
        self.with(|s| {
            s.events.push(ev);
            let n = s.pages.len().saturating_sub(1);
            s.current = match step {
                i32::MIN => 0, // reset（launch）
                1 => (s.current + 1).min(n),
                -1 => s.current.saturating_sub(1),
                _ => s.current,
            };
        })
    }

    // ===== 采集 =====

    /// 把当前页 XML + 一张纯黑截图写进工作区——感知层照常读文件解析（真实路径）。
    pub fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        self.capture_xml_only(workarea)?;
        image::RgbaImage::new(720, 1280)
            .save(workarea.screenshot_path())
            .map_err(|e| TkeError::DeviceError(format!("fake 截图写入失败：{}", e)))?;
        Ok(())
    }

    pub fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        let xml = self.with(|s| s.pages.get(s.current).cloned())?
            .ok_or_else(|| TkeError::DeviceError(format!("fake 设备 {} 页面脚本为空", self.id)))?;
        std::fs::write(workarea.ui_tree_path(), xml).map_err(TkeError::IoError)?;
        Ok(())
    }

    // ===== 操作（记录事件 + 按语义推进页面） =====

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        self.record(format!("tap {},{}", x, y), 1)
    }

    pub fn hover(&self, x: i32, y: i32) -> Result<()> {
        self.record(format!("hover {},{}", x, y), 0)
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        self.record(format!("swipe {},{} -> {},{} ({}ms)", x1, y1, x2, y2, duration_ms), 0)
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        self.record(format!("press {},{} ({}ms)", x, y, duration_ms), 0)
    }

    pub fn input_text(&self, text: &str) -> Result<()> {
        self.record(format!("input \"{}\"", text), 0)
    }

    pub fn key_event(&self, code: &str) -> Result<()> {
        self.record(format!("key {}", code), 0)
    }

    pub fn back(&self) -> Result<()> {
        self.record("back".into(), -1)
    }

    pub fn home(&self) -> Result<()> {
        self.record("home".into(), 0)
    }

    pub fn switch(&self, target: &str) -> Result<()> {
        self.record(format!("switch {}", target), 1)
    }

    pub fn launch_app(&self, package: &str, activity: &str) -> Result<()> {
        self.record(format!("launch {} {}", package, activity), i32::MIN)
    }

    pub fn stop_app(&self, package: &str) -> Result<()> {
        self.record(format!("close {}", package), 0)
    }

    pub fn clear_input(&self) -> Result<()> {
        self.record("clear".into(), 0)
    }

    pub fn hide_keyboard(&self) -> Result<()> {
        self.record("hide_keyboard".into(), 0)
    }

    // ===== 信息 =====

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        Ok(DeviceInfo {
            id: self.id.clone(),
            model: Some("fake".into()),
            manufacturer: Some("tke-test".into()),
            android_version: None,
            screen_width: 720,
            screen_height: 1280,
            hardware: None,
            battery: None,
            network: None,
        })
    }
}
