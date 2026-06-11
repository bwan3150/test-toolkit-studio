// Control 命令处理器（② 原子方法）
// 对设备执行统一操作，操作名与 .tks 命令对应
// 坐标参数格式: "x,y" 或 "x,y,毫秒"（如 tke control press 100,200,800）

use tke::{Result, Control, ControlAction, JsonOutput};
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
    /// 启动应用: control launch <包名> <Activity>
    Launch {
        package: String,
        activity: String,
    },
    /// 关闭应用: control close <包名>
    Close {
        package: String,
    },
    /// 按键事件: control key KEYCODE_ENTER
    Key {
        code: String,
    },
}

/// 把 CLI 参数转换为 ControlAction
fn to_action(cmd: ControlCommands) -> Result<ControlAction> {
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
        ControlCommands::Launch { package, activity } => ControlAction::Launch { package, activity },
        ControlCommands::Close { package } => ControlAction::Close { package },
        ControlCommands::Key { code } => ControlAction::Key { code },
    })
}

/// 处理 Control 命令（必须指定 -d/--device）
pub async fn handle(cmd: ControlCommands, device_id: Option<String>) -> Result<()> {
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("control 必须指定设备: -d/--device <设备ID>"));

    let action = to_action(cmd).unwrap_or_else(|e| JsonOutput::error(e.to_string()));

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
