// 【执行】把一个 AgentAction 落地：设备执行 + 元素落库 + 生成可回放的 .tks 行
//   device   执行原语（ControlAction → 设备，经统一 execute_action）
//   library  按坐标落库
//   script   .tks 写出
// apply() 编排三者，.tks 行经 Phase 2 序列化器生成（构造 TksStep → step_to_source），
// 保证产出与 parser 同构、可被 run 回放，不再手拼字符串。

pub mod device;
pub mod library;
pub mod script;

use std::path::Path;

use crate::workflow::step_to_source;
use crate::{ControlAction, LocatorStrategy, Point, Result, TkeError, TksCommand, TksParam, TksStep, UIElement};

use super::tools::AgentAction;
use super::transcript::Transcript;
use device::exec;
use library::save_element;

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
            Ok((line(TksCommand::Click, vec![el_param(name)]), detail))
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
            Ok((line(TksCommand::Input, vec![el_param(name), TksParam::Text(text.clone())]), detail))
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
            Ok((line(TksCommand::Press, vec![el_param(name), TksParam::Number(*duration_ms as i32)]), detail))
        }
        AgentAction::Clear { element_id, name, desc } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            save_element(device, element_path, name, desc, c.x, c.y, tx, round).await;
            // 先点击聚焦，再清空（ControlAction::Clear 自身不带坐标）
            let _ = exec(device, ControlAction::Click { point: c }).await?;
            let detail = exec(device, ControlAction::Clear).await?;
            Ok((line(TksCommand::Clear, vec![el_param(name)]), detail))
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
            Ok((
                line(
                    TksCommand::DirectionalSwipe,
                    vec![
                        TksParam::Coordinate(Point::new(cx, cy)),
                        TksParam::Direction(direction.clone()),
                        TksParam::Number(dist),
                    ],
                ),
                detail,
            ))
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
            let mut params = vec![TksParam::Text(target.clone())];
            if let Some(act) = activity {
                params.push(TksParam::Text(act.clone()));
            }
            Ok((line(TksCommand::Launch, params), detail))
        }
        AgentAction::Close { target } => {
            let detail = exec(device, ControlAction::Close { package: target.clone() }).await?;
            Ok((line(TksCommand::Close, vec![TksParam::Text(target.clone())]), detail))
        }
        AgentAction::Back => {
            let detail = exec(device, ControlAction::Back).await?;
            Ok((line(TksCommand::Back, vec![]), detail))
        }
        AgentAction::HideKeyboard => {
            let detail = exec(device, ControlAction::HideKeyboard).await?;
            Ok((line(TksCommand::HideKeyboard, vec![]), detail))
        }
        AgentAction::Wait { ms, element } => {
            if let Some(ms) = ms {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(*ms as i32)]), format!("等待 {}ms", ms)))
            } else if let Some(elem) = element {
                // v1：仅记录可回放的 .tks 行，实时不阻塞（下一轮采集会反映新状态）
                Ok((line(TksCommand::Wait, vec![el_param(elem)]), format!("记录等待元素出现: {}", elem)))
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(1000)]), "等待 1000ms".to_string()))
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

/// 构造 TksStep 并序列化成 .tks 行（统一经 Phase 2 序列化器，保证可回放）
fn line(command: TksCommand, params: Vec<TksParam>) -> String {
    step_to_source(&TksStep { command, params, raw: String::new(), line_number: 0 })
}

/// `{元素名}`（auto 策略）参数
fn el_param(name: &str) -> TksParam {
    TksParam::Element { name: name.to_string(), strategy: LocatorStrategy::Auto }
}

/// 取元素列表中第 id 个元素
fn lookup(elements: &[UIElement], id: usize) -> Result<&UIElement> {
    elements.get(id).ok_or_else(|| {
        TkeError::ScriptExecuteError(format!("无效的 element_id={}（共 {} 个元素）", id, elements.len()))
    })
}
