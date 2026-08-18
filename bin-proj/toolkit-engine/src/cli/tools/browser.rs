// 【browser】浏览器专属能力：干净态重置 / 注入 JS / 视口尺寸 / 下载。
//
// 为什么单开一组而不是塞进 control：control 是**三端统一**的动作接口
// （点击/输入/滑动，安卓 iOS 网页都得有）。cookie、localStorage、下载目录
// 只有浏览器有，硬塞进去会逼着另外两端实现一堆"不支持"的空壳。

use std::path::PathBuf;
use std::time::Duration;

use std::sync::Arc;

use tke::{Controller, JsonOutput, Params, Result};

#[derive(clap::Subcommand)]
pub enum BrowserCommands {
    /// 回到「首次访问」状态：清 cookie / localStorage / sessionStorage / IndexedDB
    ///
    /// 浏览器会话跨命令复用——不清的话下次检查会带着上次的登录态开始，
    /// 你以为在测新用户，其实看到的是老用户视角
    ///
    /// (缓存也一起清——「首次访问」本来就包含这个，多一个开关只是多一件要记的事;
    ///  另外全局已经有个 --cache 指缓存目录，同名参数会直接撞车)
    Reset,
    /// 在页面里跑一段 JS 并打印结果：`browser eval "localStorage.getItem('token')"`
    ///
    /// 用来**观察和造前置状态**。别拿它代替用户操作——直接调函数改状态，
    /// 测的就不是真链路了
    Eval {
        /// JS 代码。不写 return 就当表达式处理
        script: String,
    },
    /// 改视口尺寸测响应式：`browser viewport 390x844`
    Viewport {
        /// `宽x高`，如 390x844（iPhone 竖屏）/ 1440x900
        size: String,
    },
    /// 指定下载目录并可等下载完成——无头 Chrome 默认根本不落盘
    Download {
        /// 下载落到哪个目录
        #[arg(long, value_name = "目录")]
        dir: PathBuf,
        /// 设好目录后等新文件下完（秒）。不加就只设置目录、立即返回
        #[arg(long, value_name = "秒")]
        wait: Option<u64>,
    },
}

pub fn handle(cmd: BrowserCommands, params: Arc<Params>) -> Result<()> {
    let device = params.device.clone();
    // 这组命令**只对浏览器成立**。不拦的话 `-d <安卓序列号> browser reset` 会跑去
    // 连安卓驱动，报一句风马牛不相及的错
    match device.as_deref() {
        Some("web") => {}
        _ => JsonOutput::error("browser 子命令只对浏览器有效，请加 -d web"),
    }
    let controller = Controller::new(device)?;

    match cmd {
        BrowserCommands::Reset => match controller.web_reset(true) {
            Ok(done) => {
                println!("已清：{}", done.join(" · "));
                Ok(())
            }
            Err(e) => JsonOutput::error(e.to_string()),
        },
        BrowserCommands::Eval { script } => match controller.web_eval(&script) {
            Ok(v) => {
                // 字符串直接打原文，别带引号——这行多半要被人或脚本接着用
                match v.as_str() {
                    Some(s) => println!("{}", s),
                    None => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                }
                Ok(())
            }
            Err(e) => JsonOutput::error(e.to_string()),
        },
        BrowserCommands::Viewport { size } => {
            let (w, h) = match parse_size(&size) {
                Some(v) => v,
                None => JsonOutput::error(format!("尺寸要写成 宽x高，如 390x844（收到 {}）", size)),
            };
            match controller.web_viewport(w, h) {
                Ok(()) => {
                    println!("视口 {}x{}", w, h);
                    Ok(())
                }
                Err(e) => JsonOutput::error(e.to_string()),
            }
        }
        BrowserCommands::Download { dir, wait } => {
            if let Err(e) = controller.web_download_dir(&dir) {
                JsonOutput::error(e.to_string());
            }
            println!("下载落点 {}", dir.display());
            if let Some(secs) = wait {
                match controller.web_wait_download(&dir, Duration::from_secs(secs)) {
                    Ok(files) => {
                        for f in files {
                            println!("{}", f.display());
                        }
                    }
                    Err(e) => JsonOutput::error(e.to_string()),
                }
            }
            Ok(())
        }
    }
}

/// `390x844` → (390, 844)。大小写 X 都认——人打字不会在意这个
fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.trim().split_once(['x', 'X', '*'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn parses_sizes_people_actually_type() {
        assert_eq!(parse_size("390x844"), Some((390, 844)));
        assert_eq!(parse_size(" 1440X900 "), Some((1440, 900)));
        assert_eq!(parse_size("1280*720"), Some((1280, 720)));
        assert_eq!(parse_size("390"), None);
        assert_eq!(parse_size("axb"), None);
    }
}
