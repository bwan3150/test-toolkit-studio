// iOS 模拟器驱动（idb）——**跟真机是两条完全不同的路**。
//
// 真机走 go-ios + WebDriverAgent：设备上必须跑一个签名过的 runner，Apple 的硬要求。
// 模拟器不需要——`idb_companion` 直接调 CoreSimulator 的私有框架，
// 装它就是 `brew install idb-companion` 一条命令，免签名、免 Xcode 工程。
//
// 分工（都是外部进程，tke 只做转译）：
//   idb ui describe-all   元素树（AX 树，扁平 JSON）
//   idb ui tap/swipe/text/key/button   操作
//   xcrun simctl io screenshot         截图（simctl 比 idb 直接）
//   xcrun simctl launch/terminate      起/关 App
//
// ⚠️ **坐标单位是「点」**：describe-all 的 frame 和 `ui tap` 的入参都是点，
// 而 tke 对外一律用**截图像素**。差一个 scale，靠 AXApplication 的宽度除截图宽度算出来
// （实测 iPhone 模拟器：截图 1206 ÷ AX 402 = 3.0）。**换算必须在驱动层做完**——
// 让调用方 AI 自己乘 dpr 是把工具的活推给它，也迟早算错。

mod normalize;

use std::path::Path;
use std::process::Command;

use crate::utils::Workarea;
use crate::{DeviceInfo, Result, TkeError};

pub struct IdbDriver {
    udid: String,
    /// 截图像素 ÷ AX 点。首次采集时算出来缓存，省得每步都算
    scale: std::sync::Mutex<Option<f64>>,
}

impl IdbDriver {
    pub fn new(device_id: String) -> Result<Self> {
        let udid = device_id.strip_prefix("sim:").unwrap_or(&device_id).to_string();
        Ok(Self { udid, scale: std::sync::Mutex::new(None) })
    }

    // ===== 外部命令 =====

    /// 跑一条 `idb`。找不到就把**怎么装**说清楚——这类"缺依赖"的报错最容易让人卡住
    fn idb(&self, args: &[&str]) -> Result<String> {
        let bin = crate::ToolManager::resolve("idb")
            .ok()
            .or_else(|| which::which("idb").ok())
            .ok_or_else(|| {
                TkeError::DeviceError(
                    "没装 idb（iOS 模拟器要靠它操作）：\n\
                     　brew tap facebook/fb && brew trust facebook/fb\n\
                     　brew install idb-companion && pip3 install fb-idb"
                        .into(),
                )
            })?;
        let out = Command::new(&bin)
            .args(args)
            .arg("--udid")
            .arg(&self.udid)
            .output()
            .map_err(|e| TkeError::DeviceError(format!("执行 idb 失败: {}", e)))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            // idb 的报错里常混着一大段 Python traceback，只挑最后一句有用的
            let msg = err
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty() && !l.starts_with("  "))
                .unwrap_or("未知错误");
            return Err(TkeError::DeviceError(format!("idb {}: {}", args.join(" "), msg)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// 跑一条 `xcrun simctl`
    fn simctl(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("xcrun")
            .arg("simctl")
            .args(args)
            .arg(&self.udid)
            .output()
            .map_err(|e| TkeError::DeviceError(format!("执行 simctl 失败: {}", e)))?;
        if !out.status.success() {
            return Err(TkeError::DeviceError(format!(
                "simctl {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// simctl 的参数顺序是 `simctl <子命令> <udid> [其余]`，跟上面那个反过来
    fn simctl_dev_first(&self, sub: &str, rest: &[&str]) -> Result<String> {
        let out = Command::new("xcrun")
            .arg("simctl")
            .arg(sub)
            .arg(&self.udid)
            .args(rest)
            .output()
            .map_err(|e| TkeError::DeviceError(format!("执行 simctl 失败: {}", e)))?;
        if !out.status.success() {
            return Err(TkeError::DeviceError(format!(
                "simctl {}: {}",
                sub,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    // ===== 坐标 =====

    /// 截图像素 → AX 点。**每个操作入口都要过这一道**
    fn to_points(&self, x: i32, y: i32) -> Result<(f64, f64)> {
        let s = self.scale()?;
        Ok((x as f64 / s, y as f64 / s))
    }

    /// 拿 scale（缓存）。算不出来时**报错而不是假设 2 或 3**——
    /// 猜错的话每一次点击都偏，而且偏得很"合理"，最难查
    fn scale(&self) -> Result<f64> {
        if let Some(s) = *self.scale.lock().unwrap() {
            return Ok(s);
        }
        let tmp = std::env::temp_dir().join(format!("tke-idb-scale-{}.png", self.udid));
        self.simctl_dev_first("io", &["screenshot", &tmp.to_string_lossy()])?;
        let width = png_width(&tmp).ok_or_else(|| {
            TkeError::DeviceError("读不出截图宽度，算不了坐标缩放".into())
        })?;
        let _ = std::fs::remove_file(&tmp);
        let json = self.idb(&["ui", "describe-all"])?;
        let s = normalize::scale_from(&json, width).ok_or_else(|| {
            TkeError::DeviceError(
                "元素树里没有 Application 那条，算不出坐标缩放（App 起来了吗）".into(),
            )
        })?;
        *self.scale.lock().unwrap() = Some(s);
        Ok(s)
    }

    // ===== 采集 =====

    pub fn capture_ui_state(&self, workarea: &Workarea) -> Result<()> {
        self.capture_screenshot(workarea)?;
        self.capture_ax(workarea)
    }

    pub fn capture_xml_only(&self, workarea: &Workarea) -> Result<()> {
        self.capture_ax(workarea)
    }

    fn capture_screenshot(&self, workarea: &Workarea) -> Result<()> {
        let path = workarea.screenshot_path();
        self.simctl_dev_first("io", &["screenshot", &path.to_string_lossy()])?;
        Ok(())
    }

    fn capture_ax(&self, workarea: &Workarea) -> Result<()> {
        let json = self.idb(&["ui", "describe-all"])?;
        // AX 原文先存一份：normalize 会筛元素、换坐标，对得上原文才知道某个控件
        // 是被筛掉了还是压根没采到（web/adb/wda 同理）
        let _ = std::fs::write(workarea.raw_page_path("json"), &json);
        let xml = normalize::normalize_ax_json(&json, self.scale()?)?;
        std::fs::write(workarea.ui_tree_path(), xml).map_err(TkeError::IoError)?;
        Ok(())
    }

    // ===== 操作（入参一律是截图像素） =====

    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        let (px, py) = self.to_points(x, y)?;
        self.idb(&["ui", "tap", &fmt(px), &fmt(py)])?;
        Ok(())
    }

    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        let (px, py) = self.to_points(x, y)?;
        let secs = format!("{:.2}", duration_ms as f64 / 1000.0);
        self.idb(&["ui", "tap", "--duration", &secs, &fmt(px), &fmt(py)])?;
        Ok(())
    }

    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        let (a, b) = self.to_points(x1, y1)?;
        let (c, d) = self.to_points(x2, y2)?;
        let secs = format!("{:.2}", duration_ms as f64 / 1000.0);
        self.idb(&["ui", "swipe", "--duration", &secs, &fmt(a), &fmt(b), &fmt(c), &fmt(d)])?;
        Ok(())
    }

    pub fn input_text(&self, text: &str) -> Result<()> {
        self.idb(&["ui", "text", text])?;
        Ok(())
    }

    pub fn clear_input(&self) -> Result<()> {
        // HID keycode 42 = Backspace。没有"全选删除"这种整体操作，只能连按——
        // 与 WDA 那边一个做法（那儿是发 50 个退格）
        let backspaces: Vec<String> = std::iter::repeat("42".to_string()).take(50).collect();
        let mut args = vec!["ui", "key-sequence"];
        args.extend(backspaces.iter().map(|s| s.as_str()));
        self.idb(&args)?;
        Ok(())
    }

    /// HID keycode（USB HID Usage Table）——跟安卓的 KEYCODE_* 是两套东西，这里做映射
    pub fn key_event(&self, key_code: &str) -> Result<()> {
        let code = match key_code.to_uppercase().as_str() {
            "KEYCODE_ENTER" | "ENTER" => "40",
            "KEYCODE_ESCAPE" | "ESC" | "ESCAPE" => "41",
            "KEYCODE_DEL" | "BACKSPACE" => "42",
            "KEYCODE_TAB" | "TAB" => "43",
            "KEYCODE_BACK" => return self.back(),
            "KEYCODE_HOME" | "HOME" => return self.home(),
            // 认不出就**报错**，别静默吞掉（P-40：`_ => Ok(())` 会让人以为按下去了）
            other => {
                return Err(TkeError::InvalidArgument(format!(
                    "模拟器上没有这个按键：{}（支持 ENTER / ESC / BACKSPACE / TAB / HOME）",
                    other
                )))
            }
        };
        self.idb(&["ui", "key", code])?;
        Ok(())
    }

    /// iOS 没有系统返回键——用**左边缘右滑**模拟，跟真机(WDA)一个路子
    pub fn back(&self) -> Result<()> {
        let s = self.scale()?;
        let json = self.idb(&["ui", "describe-all"])?;
        // 高度取 Application 那条；拿不到就用一个保守的中间值
        let h = serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| {
                v.as_array()?
                    .iter()
                    .find(|e| e["type"].as_str() == Some("Application"))?["frame"]["height"]
                    .as_f64()
            })
            .unwrap_or(800.0);
        let y = h / 2.0;
        self.idb(&["ui", "swipe", "--duration", "0.30", "2", &fmt(y), "200", &fmt(y)])?;
        let _ = s; // scale 只是用来确认 App 起着
        Ok(())
    }

    pub fn home(&self) -> Result<()> {
        self.idb(&["ui", "button", "HOME"])?;
        Ok(())
    }

    /// 模拟器用的是电脑键盘，没有软键盘要收——**空操作，但不算失败**
    pub fn hide_keyboard(&self) -> Result<()> {
        Ok(())
    }

    /// 触摸屏没有 hover 这回事（同 Android / 真机 iOS）
    pub fn hover(&self, _x: i32, _y: i32) -> Result<()> {
        Err(TkeError::InvalidArgument(
            "iOS 模拟器不支持悬停（hover 为 web 独有）".to_string(),
        ))
    }

    pub fn launch_app(&self, bundle_id: &str) -> Result<()> {
        self.simctl_dev_first("launch", &[bundle_id])?;
        // App 换了，屏幕尺寸不会变，但 scale 缓存留着没坏处
        Ok(())
    }

    pub fn stop_app(&self, bundle_id: &str) -> Result<()> {
        if bundle_id.is_empty() {
            return Ok(()); // 模拟器没有"会话"可销毁
        }
        self.simctl_dev_first("terminate", &[bundle_id])?;
        Ok(())
    }

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        let s = self.scale().unwrap_or(1.0);
        let json = self.idb(&["ui", "describe-all"]).unwrap_or_default();
        let (w, h) = serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| {
                let app = v
                    .as_array()?
                    .iter()
                    .find(|e| e["type"].as_str() == Some("Application"))?["frame"]
                    .clone();
                Some((app["width"].as_f64()?, app["height"].as_f64()?))
            })
            .unwrap_or((0.0, 0.0));
        // 机型/系统版本从 simctl 那边问（AX 树里没有这些）
        let name = self
            .simctl_list_name()
            .unwrap_or_else(|| "iOS 模拟器".to_string());
        Ok(DeviceInfo {
            id: format!("sim:{}", self.udid),
            model: Some(name),
            manufacturer: Some("Apple".to_string()),
            android_version: None,
            screen_width: (w * s) as u32,
            screen_height: (h * s) as u32,
            hardware: None,
            battery: None,
            network: None,
        })
    }

    /// 从 `simctl list` 里找这台的机型名（"iPhone 17 Pro"）
    fn simctl_list_name(&self) -> Option<String> {
        let out = Command::new("xcrun")
            .args(["simctl", "list", "devices", "--json"])
            .output()
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        for (rt, list) in v["devices"].as_object()? {
            for d in list.as_array()? {
                if d["udid"].as_str() == Some(&self.udid) {
                    let ver = rt.rsplit('.').next().unwrap_or("").replacen('-', " ", 1).replace('-', ".");
                    return Some(format!("{}（{}）", d["name"].as_str().unwrap_or("iPhone"), ver));
                }
            }
        }
        None
    }
}

/// idb 的坐标参数不接受科学计数法，也不需要小数点后一长串
fn fmt(v: f64) -> String {
    format!("{:.0}", v)
}

/// 从 PNG 头里读宽度（IHDR 的前 4 字节），不为了这个引一个图像库
fn png_width(path: &Path) -> Option<u32> {
    let bytes = std::fs::read(path).ok()?;
    // 8 字节签名 + 4 长度 + 4 类型("IHDR") = 16，宽度在 16..20
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some(u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]))
}

