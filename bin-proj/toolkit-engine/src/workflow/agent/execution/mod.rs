// 【执行】把一个 AgentAction 落地：设备执行 + 元素落库 + 生成可回放的 .tks 行
//   device   执行原语（ControlAction → 设备）
//   library  按坐标落库
//   script   .tks 文本工具 + 写出
// apply() 负责按动作类型编排这三者，返回 (.tks 行, 执行详情)。

pub mod device;
pub mod library;
pub mod script;

use std::path::Path;

use crate::{ControlAction, Point, Result, TkeError, UIElement};

use super::tools::AgentAction;
use super::transcript::Transcript;
use device::exec;
use library::save_element;
use script::{direction_cn, escape_text};

/// 执行一个设备动作：返回 (.tks 行, 执行详情)
///
/// 注意：仅处理"设备动作"；控制流动作（要图/反问/finish）由 runner 主循环拦截，不进此处。
pub async fn apply(
    device: &str,
    element_path: &Path,
    action: &AgentAction,
    elements: &[UIElement],
    shot_path: &Path,
    tx: &mut Transcript,
    round: usize,
) -> Result<(String, String)> {
    match action {
        AgentAction::Click { element_id, name, desc } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            save_element(device, element_path, name, desc, c.x, c.y, tx, round).await;
            let detail = exec(device, ControlAction::Click { point: c }).await?;
            Ok((format!("点击 [{{{}}}]", name), detail))
        }
        AgentAction::Input { element_id, name, desc, text } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            save_element(device, element_path, name, desc, c.x, c.y, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Input { text: text.clone(), point: Some(c) },
            )
            .await?;
            Ok((format!("输入 [{{{}}}, \"{}\"]", name, escape_text(text)), detail))
        }
        AgentAction::LongPress { element_id, name, desc, duration_ms } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            save_element(device, element_path, name, desc, c.x, c.y, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Press { point: c, duration_ms: *duration_ms as u32 },
            )
            .await?;
            Ok((format!("按压 [{{{}}}, {}]", name, duration_ms), detail))
        }
        AgentAction::Clear { element_id, name, desc } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            save_element(device, element_path, name, desc, c.x, c.y, tx, round).await;
            // 先点击聚焦，再清空（ControlAction::Clear 自身不带坐标）
            let _ = exec(device, ControlAction::Click { point: c }).await?;
            let detail = exec(device, ControlAction::Clear).await?;
            Ok((format!("清理 [{{{}}}]", name), detail))
        }
        AgentAction::SwipeDir { direction, distance } => {
            let (w, h) = image::image_dimensions(shot_path).unwrap_or((1080, 1920));
            let (cx, cy) = (w as i32 / 2, h as i32 / 2);
            let dist = distance.unwrap_or((h as i32) * 5 / 10);
            let detail = exec(
                device,
                ControlAction::SwipeDir {
                    from: Point::new(cx, cy),
                    direction: direction.clone(),
                    distance: dist,
                    duration_ms: 300,
                },
            )
            .await?;
            let dir_cn = direction_cn(direction);
            Ok((format!("定向滑动 [{{{}, {}}}, {}, {}]", cx, cy, dir_cn, dist), detail))
        }
        AgentAction::Launch { target, activity } => {
            let detail = exec(
                device,
                ControlAction::Launch {
                    package: target.clone(),
                    activity: activity.clone().unwrap_or_default(),
                },
            )
            .await?;
            let line = match activity {
                Some(act) => format!("启动 [{}, {}]", target, act),
                None => format!("启动 [{}]", target),
            };
            Ok((line, detail))
        }
        AgentAction::Close { target } => {
            let detail = exec(device, ControlAction::Close { package: target.clone() }).await?;
            Ok((format!("关闭 [{}]", target), detail))
        }
        AgentAction::Back => {
            let detail = exec(device, ControlAction::Back).await?;
            Ok(("返回".to_string(), detail))
        }
        AgentAction::HideKeyboard => {
            let detail = exec(device, ControlAction::HideKeyboard).await?;
            Ok(("隐藏键盘".to_string(), detail))
        }
        AgentAction::Wait { ms, element } => {
            if let Some(ms) = ms {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok((format!("等待 [{}]", ms), format!("等待 {}ms", ms)))
            } else if let Some(elem) = element {
                // v1：仅记录可回放的 .tks 行，实时不阻塞（下一轮采集会反映新状态）
                Ok((format!("等待 [{{{}}}]", elem), format!("记录等待元素出现: {}", elem)))
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                Ok(("等待 [1000]".to_string(), "等待 1000ms".to_string()))
            }
        }
        // 控制流动作不在此处理（由主循环拦截）
        AgentAction::RequestScreenshot { .. }
        | AgentAction::AskUser { .. }
        | AgentAction::Finish { .. } => {
            Err(TkeError::ScriptExecuteError("控制流动作不应进入执行器".to_string()))
        }
    }
}

/// 取元素列表中第 id 个元素
fn lookup(elements: &[UIElement], id: usize) -> Result<&UIElement> {
    elements.get(id).ok_or_else(|| {
        TkeError::ScriptExecuteError(format!("无效的 element_id={}（共 {} 个元素）", id, elements.len()))
    })
}
