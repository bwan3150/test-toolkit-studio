// 浏览器专属能力：干净态重置 / 注入 JS / 视口尺寸 / 下载。
//
// 这些都是**只有浏览器有**的东西（移动端没有 cookie 和 localStorage 这一说），
// 所以单独成文件，不去污染 Controller 那套三端统一的动作接口。
//
// 走 CDP 的部分用 chromedriver 的 `/goog/cdp/execute` 扩展端点——W3C 没有对应标准，
// 清缓存和改下载目录只能这么来。拿不到就如实报错，别假装做过了。

use super::WebDriver;
use crate::{Result, TkeError};
use std::path::{Path, PathBuf};
use std::time::Duration;

impl WebDriver {
    /// 发一条 CDP 命令（chromedriver 扩展端点，非 W3C 标准）
    fn cdp(&self, cmd: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        self.post("/goog/cdp/execute", serde_json::json!({ "cmd": cmd, "params": params }))
    }

    /// 回到「首次访问」的状态：清 cookie、localStorage、sessionStorage、IndexedDB、缓存。
    ///
    /// 测登录、首访引导、权限弹窗都要它——**浏览器会话跨命令复用**，不清的话第二次
    /// 检查会带着上一次的登录态开始，看到的是老用户视角，而你以为测的是新用户。
    ///
    /// 返回**实际做成了哪几项**：清不掉的要说出来（比如 `about:blank` 上没有 storage
    /// 可清），否则「已重置」四个字会让人以为回到干净态了，其实没有。
    pub fn reset_state(&self, with_cache: bool) -> Result<Vec<String>> {
        let mut done = Vec::new();

        // cookie：W3C 标准端点
        if self.delete("/cookie").is_ok() {
            done.push("cookie".to_string());
        }

        // storage：要有页面上下文才有得清（about:blank 下访问 localStorage 会抛安全错）
        let js = "try { localStorage.clear(); sessionStorage.clear(); \
                  if (window.indexedDB && indexedDB.databases) { \
                    indexedDB.databases().then(l => l.forEach(d => d.name && indexedDB.deleteDatabase(d.name))); } \
                  return 'ok'; } catch (e) { return 'skip:' + e.name; }";
        match self.execute(js, serde_json::json!([])) {
            Ok(v) if v.as_str() == Some("ok") => done.push("localStorage/sessionStorage/IndexedDB".to_string()),
            _ => {}
        }

        if with_cache && self.cdp("Network.clearBrowserCache", serde_json::json!({})).is_ok() {
            done.push("缓存".to_string());
        }

        if done.is_empty() {
            return Err(TkeError::DeviceError(
                "一项都没清成——多半是当前没有打开任何页面（先 `启动 [URL]`）".into(),
            ));
        }
        Ok(done)
    }

    /// 在页面里执行一段 JS 并返回结果。
    ///
    /// 用来**观察和造前置状态**（读 localStorage、看 window 上的状态、mock 时间），
    /// **不要拿它代替用户操作**——直接调函数改状态测的就不是真链路了，
    /// 那正是这个工具存在的意义所在。
    pub fn eval_js(&self, script: &str) -> Result<serde_json::Value> {
        // 没写 return 的当表达式处理：`eval '1+1'` 该给 2，而不是 null。
        // 人在命令行里手打的多半是表达式，替他补上比让他记规则强
        let has_return = script.contains("return ");
        let wrapped = if has_return {
            script.to_string()
        } else {
            format!("return ({});", script.trim().trim_end_matches(';'))
        };
        self.execute(&wrapped, serde_json::json!([]))
    }

    /// 改**视口**尺寸——测响应式布局用（`390x844` = iPhone 竖屏）。
    ///
    /// 走 CDP 的 `Emulation.setDeviceMetricsOverride` 而不是 `/window/rect`：
    /// 后者改的是**窗口**,窗口里还有标签栏、地址栏、边框,量下来 innerHeight 会短一截
    /// （实测设 390x844 拿到 390x757）。测响应式看的是断点,差几十像素就可能跨过断点、
    /// 测的根本不是那个布局。deviceScaleFactor 传 0 = 沿用设备默认,别顺手把 dpr 也改了
    /// ——截图坐标换算依赖它,改了整套点击坐标都会偏。
    pub fn set_viewport(&self, width: u32, height: u32) -> Result<()> {
        // 窗口比视口小的话内容会被挤,先把窗口撑到够大(失败无所谓,无头下窗口本来就是虚的)
        let _ = self.post(
            "/window/rect",
            serde_json::json!({ "width": width, "height": height + 120, "x": 0, "y": 0 }),
        );
        self.cdp(
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width": width, "height": height,
                "deviceScaleFactor": 0, "mobile": false,
            }),
        )?;
        Ok(())
    }

    /// 把下载落到指定目录。
    ///
    /// **无头 Chrome 默认根本不下载**（或落到一个谁也找不到的临时目录），
    /// 于是「导出 CSV」这类功能没法验——点了按钮，然后呢？文件在哪都不知道。
    pub fn set_download_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(TkeError::IoError)?;
        let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        self.cdp(
            "Page.setDownloadBehavior",
            serde_json::json!({ "behavior": "allow", "downloadPath": abs.to_string_lossy() }),
        )?;
        Ok(())
    }

    /// 等下载落地：轮询到目录里出现**下完了的**文件为止，返回它们。
    ///
    /// 判据是「有文件 **且** 没有 `.crdownload` 半成品」——Chrome 先建
    /// `xxx.crdownload`、完事再改名,只看"目录里有东西了"会拿到下到一半的空壳,
    /// 后面校验内容必然对不上。
    ///
    /// ⚠️ **不能用"有没有新增文件"当判据**：CLI 每条命令都是新进程,记不住上一次的基线。
    /// 实测踩过：文件明明已经下好了,却因为"跟进来时一样、没有新增"报超时。
    /// 要区分新旧,清空目录或换个目录——那是调用方的事,不该由一个记不住状态的进程假装能办。
    pub fn wait_download(&self, dir: &Path, timeout: Duration) -> Result<Vec<PathBuf>> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let now = list_files(dir);
            let partial = now
                .iter()
                .any(|p| p.extension().and_then(|e| e.to_str()) == Some("crdownload"));
            let done: Vec<PathBuf> = now
                .into_iter()
                .filter(|p| p.extension().and_then(|e| e.to_str()) != Some("crdownload"))
                .collect();
            if !done.is_empty() && !partial {
                return Ok(done);
            }
            if std::time::Instant::now() >= deadline {
                return Err(TkeError::DeviceError(if partial {
                    format!("等超时：{} 里还有没下完的 .crdownload", dir.display())
                } else {
                    format!("等超时：{} 里一个文件都没有", dir.display())
                }));
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    /// DELETE 请求（清 cookie 用；mod.rs 那边只有 get/post）
    fn delete(&self, path: &str) -> Result<()> {
        ureq::delete(&self.endpoint(path)?)
            .timeout(Duration::from_secs(30))
            .call()
            .map_err(|e| TkeError::DeviceError(format!("WebDriver 请求失败 {}: {}", path, e)))?;
        Ok(())
    }
}

/// 目录里的文件（不递归；下载不会建子目录）
fn list_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    v.sort();
    v
}
