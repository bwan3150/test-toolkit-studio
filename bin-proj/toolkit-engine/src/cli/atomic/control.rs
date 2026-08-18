// Control 命令处理器（② 原子方法）
// 对设备执行统一操作，操作名与 .tks 命令对应
// 坐标参数格式: "x,y" 或 "x,y,毫秒"（如 tke control press 100,200,800）

use tke::{Result, Control, ControlAction, DialogAction, JsonOutput, TkeError};
use tke::atomic::control::parse_point;

/// Control 命令枚举（统一操作名）
#[derive(clap::Subcommand)]
pub enum ControlCommands {
    /// 点击: control click x,y
    Click {
        /// 坐标 "x,y"
        point: String,
    },
    /// 按压(长按): control press x,y[,毫秒]
    Press {
        /// 坐标 "x,y" 或 "x,y,毫秒"
        point: String,
        /// 按压时长(毫秒)，默认 1000
        #[arg(short = 't', long)]
        duration: Option<u32>,
    },
    /// 滑动: control swipe x1,y1 x2,y2 [-t 毫秒]
    Swipe {
        /// 起点 "x,y"
        from: String,
        /// 终点 "x,y"
        to: String,
        /// 滑动时长(毫秒)
        #[arg(short = 't', long, default_value = "300")]
        duration: u32,
    },
    /// 拖拽(慢速滑动): control drag x1,y1 x2,y2 [-t 毫秒]
    Drag {
        /// 起点 "x,y"
        from: String,
        /// 终点 "x,y"
        to: String,
        /// 拖拽时长(毫秒)
        #[arg(short = 't', long, default_value = "1500")]
        duration: u32,
    },
    /// 定向滑动: control swipe-dir x,y up 400
    SwipeDir {
        /// 起点 "x,y"
        from: String,
        /// 方向 up/down/left/right
        direction: String,
        /// 滑动距离(像素)
        distance: i32,
        /// 滑动时长(毫秒)
        #[arg(short = 't', long, default_value = "300")]
        duration: u32,
    },
    /// 输入文本: control input "hello" [--at x,y]
    Input {
        /// 输入的文本
        text: String,
        /// 先点击该坐标(输入框)再输入
        #[arg(long)]
        at: Option<String>,
    },
    /// 清空输入框
    Clear,
    /// 隐藏键盘
    HideKeyboard,
    /// 返回键
    Back,
    /// 主页键
    Home,
    /// 启动: control launch <包名> <Activity> (Android) / control launch <URL> (web)
    Launch {
        /// Android 包名 或 网页 URL
        package: String,
        /// Activity 名（web 时省略）
        activity: Option<String>,
    },
    /// 关闭应用: control close <包名>
    Close {
        /// 要关闭的应用包名 / BundleID。**web 可省略**——web 的"关闭"就是销毁浏览器会话
        /// （连同 chromedriver 进程与会话文件一起清掉），包名对它没有意义。
        package: Option<String>,
    },
    /// 按键事件: control key KEYCODE_ENTER
    Key {
        code: String,
    },
    // ── 浏览器独有（只对 -d web；别的设备会如实报错，不编空壳）──
    // 平铺在 control 下、统一 `browser-` 前缀：control 层就是所有原子指令的入口，
    // 单开一组命令的话 tks 解释器和 AI agent 都绕不到这些能力

    /// 回到「首次访问」: 清 cookie/localStorage/sessionStorage/IndexedDB/缓存
    ///
    /// 浏览器会话跨命令复用，登录态会一直带着——测登录/首访引导/权限弹窗前不清，
    /// 你以为在测新用户，其实看到的是老用户视角
    BrowserReset,
    /// 在页面里跑 JS: control browser-eval "localStorage.getItem('token')"
    ///
    /// 用来**观察和造前置状态**。别拿它代替用户操作——直接调函数改状态，
    /// 测的就不是真链路了
    BrowserEval {
        /// JS 代码。不写 return 就当表达式处理
        script: String,
    },
    /// 设视口测响应式: control browser-viewport 390x844
    BrowserViewport {
        /// `宽x高`，如 390x844（iPhone 竖屏）/ 1440x900
        size: String,
    },
    /// 设下载目录(无头 Chrome 默认不落盘): control browser-download --dir ~/dl [--wait 15]
    BrowserDownload {
        /// 下载落到哪个目录
        #[arg(long, value_name = "目录")]
        dir: std::path::PathBuf,
        /// 等到下载完成再返回（秒），并打印文件路径
        #[arg(long, value_name = "秒")]
        wait: Option<u64>,
    },
    /// 处理原生对话框: control browser-dialog accept|dismiss [--text "张三"]
    ///
    /// alert/confirm/prompt 是**浏览器画的**，不在 DOM 里——点不到也采不到，
    /// 只能走这条路。给了 --text 就是往 prompt 里填字并确定
    BrowserDialog {
        /// accept=确定 / dismiss=取消（给了 --text 时忽略）
        #[arg(default_value = "accept")]
        how: String,
        /// prompt 要填的文本（填完自动确定）
        #[arg(long)]
        text: Option<String>,
    },

    /// 切换: control switch <标签序号|URL> (web) / control switch <包名> (App)
    Switch {
        /// web=标签序号 或 http(s) URL（新标签打开）；移动端=要切到前台的 App 包名
        target: String,
    },
}

/// 把 CLI 参数转换为 ControlAction
fn to_action(cmd: ControlCommands, device: &str) -> Result<ControlAction> {
    Ok(match cmd {
        ControlCommands::Click { point } => {
            let (p, _) = parse_point(&point)?;
            ControlAction::Click { point: p }
        }
        ControlCommands::Press { point, duration } => {
            let (p, inline_ms) = parse_point(&point)?;
            ControlAction::Press {
                point: p,
                duration_ms: duration.or(inline_ms).unwrap_or(1000),
            }
        }
        ControlCommands::Swipe { from, to, duration } => {
            let (f, _) = parse_point(&from)?;
            let (t, inline_ms) = parse_point(&to)?;
            ControlAction::Swipe { from: f, to: t, duration_ms: inline_ms.unwrap_or(duration) }
        }
        ControlCommands::Drag { from, to, duration } => {
            let (f, _) = parse_point(&from)?;
            let (t, inline_ms) = parse_point(&to)?;
            ControlAction::Swipe { from: f, to: t, duration_ms: inline_ms.unwrap_or(duration) }
        }
        ControlCommands::SwipeDir { from, direction, distance, duration } => {
            let (f, _) = parse_point(&from)?;
            ControlAction::SwipeDir { from: f, direction, distance, duration_ms: duration }
        }
        ControlCommands::Input { text, at } => {
            let point = match at {
                Some(s) => Some(parse_point(&s)?.0),
                None => None,
            };
            ControlAction::Input { text, point }
        }
        ControlCommands::Clear => ControlAction::Clear,
        ControlCommands::HideKeyboard => ControlAction::HideKeyboard,
        ControlCommands::Back => ControlAction::Back,
        ControlCommands::Home => ControlAction::Home,
        ControlCommands::Launch { package, activity } => ControlAction::Launch {
            package,
            activity: activity.unwrap_or_default(),
        },
        ControlCommands::Close { package } => ControlAction::Close {
            // web: 省略包名 = 销毁会话（Controller 的 web 分支本就忽略这个参数）
            // 移动端: 没有包名无从下手,明确报错而不是拿空串去 force-stop
            package: package.unwrap_or_else(|| {
                if matches!(tke::Platform::from_device(Some(device)), tke::Platform::Web) {
                    String::new()
                } else {
                    JsonOutput::error(
                        "control close 需要包名: control close <包名/BundleID>（web 可省略）",
                    )
                }
            }),
        },
        ControlCommands::Key { code } => ControlAction::Key { code },
        ControlCommands::BrowserReset => ControlAction::BrowserReset,
        ControlCommands::BrowserEval { script } => ControlAction::BrowserEval { script },
        ControlCommands::BrowserViewport { size } => {
            let (width, height) = parse_size(&size).ok_or_else(|| {
                TkeError::InvalidArgument(format!("尺寸要写成 宽x高，如 390x844（收到 {}）", size))
            })?;
            ControlAction::BrowserViewport { width, height }
        }
        ControlCommands::BrowserDownload { dir, wait } => {
            ControlAction::BrowserDownload { dir, wait_secs: wait }
        }
        ControlCommands::BrowserDialog { how, text } => ControlAction::Dialog {
            action: match (text, how.as_str()) {
                (Some(t), _) => DialogAction::Input(t),
                (None, "dismiss" | "cancel" | "取消") => DialogAction::Dismiss,
                (None, "accept" | "ok" | "确定") => DialogAction::Accept,
                (None, other) => {
                    return Err(TkeError::InvalidArgument(format!(
                        "对话框动作只能是 accept / dismiss（收到 {}），填 prompt 用 --text",
                        other
                    )))
                }
            },
        },
        ControlCommands::Switch { target } => ControlAction::Switch { target },
    })
}

/// 处理 Control 命令（必须指定 -d/--device）
pub async fn handle(cmd: ControlCommands, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("control 必须指定设备: -d/--device <设备ID>"));

    let action = to_action(cmd, &device).unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    let control = Control::new(device)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match control.execute(action).await {
        Ok(detail) => {
            let mut json = serde_json::json!({ "success": true });
            if let (Some(base), serde_json::Value::Object(extra)) = (json.as_object_mut(), detail) {
                for (k, v) in extra {
                    base.insert(k, v);
                }
            }
            JsonOutput::success(json);
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}

/// `390x844` → (390, 844)。大小写 X 和 `*` 都认——人打字不会在意这个
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
