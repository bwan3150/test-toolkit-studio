// control - 对设备执行统一操作
// 操作名与 .tks 脚本命令一一对应（click/press/swipe/swipe-dir/input/clear/...），
// 坐标来自 recognize 的定位结果或直接给定，参数格式如: tke control press 100,200,800

use crate::{Result, TkeError, Controller, Point};

/// 统一操作类型（与 TksCommand 对应）
#[derive(Debug, Clone)]
pub enum ControlAction {
    /// 点击 click x,y
    Click { point: Point },
    /// 按压 press x,y[,duration_ms]
    Press { point: Point, duration_ms: u32 },
    /// 滑动 swipe x1,y1 x2,y2 [duration_ms]
    Swipe { from: Point, to: Point, duration_ms: u32 },
    /// 定向滑动 swipe-dir x,y <up|down|left|right> distance [duration_ms]
    SwipeDir { from: Point, direction: String, distance: i32, duration_ms: u32 },
    /// 输入 input "text" （先点击 x,y 可选）
    Input { text: String, point: Option<Point> },
    /// 清空输入框
    Clear,
    /// 隐藏键盘
    HideKeyboard,
    /// 返回键
    Back,
    /// 主页键
    Home,
    /// 启动应用 launch package activity
    Launch { package: String, activity: String },
    /// 关闭应用 close package
    Close { package: String },
    /// 按键事件 key KEYCODE_XXX
    Key { code: String },
    /// 切换：web=目标标签序号 或 用新标签打开 URL；移动端=把目标 App 包名切到前台
    Switch { target: String },
    /// 悬停 hover x,y（web 独有：鼠标移到坐标触发 hover，展开悬停下拉/菜单，不按下）
    Hover { point: Point },
}

/// control 原子方法
pub struct Control {
    controller: Controller,
}

impl Control {
    pub fn new(device_id: String) -> Result<Self> {
        let controller = Controller::new(Some(device_id))?;
        Ok(Self { controller })
    }

    /// 执行操作，返回结果描述（用于 JSON 输出）
    pub async fn execute(&self, action: ControlAction) -> Result<serde_json::Value> {
        execute_action(&self.controller, action).await
    }

    /// 列出标签页（仅 web；其它平台为空）
    pub fn list_tabs(&self) -> Vec<crate::TabInfo> {
        self.controller.list_tabs()
    }
}

/// 统一设备操作执行器——唯一的「ControlAction → 设备」映射。
/// `tke control` / tks 解释器(command_executor) / AI agent 都经此执行，保证操作语义单一来源。
/// 仅做坐标级原子操作；工作流控制（等待/断言/采集/启动后刷新等）由调用方负责。
pub async fn execute_action(controller: &Controller, action: ControlAction) -> Result<serde_json::Value> {
    match action {
        ControlAction::Click { point } => {
            controller.tap(point.x, point.y)?;
            Ok(serde_json::json!({ "action": "click", "x": point.x, "y": point.y }))
        }
        ControlAction::Press { point, duration_ms } => {
            controller.press(point.x, point.y, duration_ms)?;
            Ok(serde_json::json!({
                "action": "press", "x": point.x, "y": point.y, "duration_ms": duration_ms
            }))
        }
        ControlAction::Swipe { from, to, duration_ms } => {
            controller.swipe(from.x, from.y, to.x, to.y, duration_ms)?;
            Ok(serde_json::json!({
                "action": "swipe",
                "from": { "x": from.x, "y": from.y },
                "to": { "x": to.x, "y": to.y },
                "duration_ms": duration_ms
            }))
        }
        ControlAction::SwipeDir { from, direction, distance, duration_ms } => {
            let to = match direction.as_str() {
                "up" => Point::new(from.x, from.y - distance),
                "down" => Point::new(from.x, from.y + distance),
                "left" => Point::new(from.x - distance, from.y),
                "right" => Point::new(from.x + distance, from.y),
                _ => {
                    return Err(TkeError::InvalidArgument(format!(
                        "无效的方向: {} (支持 up/down/left/right)", direction
                    )))
                }
            };
            controller.swipe(from.x, from.y, to.x, to.y, duration_ms)?;
            Ok(serde_json::json!({
                "action": "swipe-dir", "direction": direction, "distance": distance,
                "from": { "x": from.x, "y": from.y },
                "to": { "x": to.x, "y": to.y },
                "duration_ms": duration_ms
            }))
        }
        ControlAction::Input { text, point } => {
            if let Some(p) = point {
                controller.tap(p.x, p.y)?;
                // 等**软键盘**弹出来——只有真实移动端需要。web 没有软键盘，而且 tap 本身
                // 已经等到页面就绪了，再睡 500ms 是纯白等（实测占「输入」这一步的 ~38%，
                // 与 P-27 同族：一个平台的必需品被无条件套到所有平台上）。
                if controller.has_soft_keyboard() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
            controller.input_text(&text)?;
            Ok(serde_json::json!({ "action": "input", "text": text }))
        }
        ControlAction::Clear => {
            controller.clear_input()?;
            Ok(serde_json::json!({ "action": "clear" }))
        }
        ControlAction::HideKeyboard => {
            controller.hide_keyboard()?;
            Ok(serde_json::json!({ "action": "hide-keyboard" }))
        }
        ControlAction::Back => {
            controller.back()?;
            Ok(serde_json::json!({ "action": "back" }))
        }
        ControlAction::Home => {
            controller.home()?;
            Ok(serde_json::json!({ "action": "home" }))
        }
        ControlAction::Launch { package, activity } => {
            controller.launch_app(&package, &activity)?;
            Ok(serde_json::json!({ "action": "launch", "package": package, "activity": activity }))
        }
        ControlAction::Close { package } => {
            controller.stop_app(&package)?;
            Ok(serde_json::json!({ "action": "close", "package": package }))
        }
        ControlAction::Key { code } => {
            controller.key_event(&code)?;
            Ok(serde_json::json!({ "action": "key", "code": code }))
        }
        ControlAction::Switch { target } => {
            controller.switch(&target)?;
            Ok(serde_json::json!({ "action": "switch", "target": target }))
        }
        ControlAction::Hover { point } => {
            controller.hover(point.x, point.y)?;
            Ok(serde_json::json!({ "action": "hover", "x": point.x, "y": point.y }))
        }
    }
}

/// 解析 "x,y" / "x,y,ms" 形式的坐标参数
pub fn parse_point(s: &str) -> Result<(Point, Option<u32>)> {
    let nums: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    if nums.len() < 2 || nums.len() > 3 {
        return Err(TkeError::InvalidArgument(format!(
            "坐标格式无效: '{}' (期望 x,y 或 x,y,毫秒)", s
        )));
    }
    let x = nums[0].parse::<i32>()
        .map_err(|_| TkeError::InvalidArgument(format!("无效的 X 坐标: {}", nums[0])))?;
    let y = nums[1].parse::<i32>()
        .map_err(|_| TkeError::InvalidArgument(format!("无效的 Y 坐标: {}", nums[1])))?;
    let duration = if nums.len() == 3 {
        Some(nums[2].parse::<u32>()
            .map_err(|_| TkeError::InvalidArgument(format!("无效的毫秒数: {}", nums[2])))?)
    } else {
        None
    };
    Ok((Point::new(x, y), duration))
}
