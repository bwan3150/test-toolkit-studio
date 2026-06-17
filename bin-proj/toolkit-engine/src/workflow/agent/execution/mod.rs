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

use crate::tools::element::OcrChannel;
use crate::workflow::step_to_source;
use crate::{ActionTrace, Bounds, ControlAction, LocatorStrategy, Point, Result, TkeError, TksCommand, TksParam, TksStep, UIElement};

use super::tools::AgentAction;
use super::transcript::Transcript;
use device::exec;
use library::save_target;

/// 执行一个设备动作：返回 (.tks 行, 执行详情, 执行轨迹)
///
/// 轨迹（ActionTrace）含本步点击点 + 目标元素 bounds，供 RunArtifacts 标注截图
/// （元素红框 + 点击蓝点），与 `tke run` 同构，便于事后核对 AI 实际点到哪。
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
) -> Result<(String, String, ActionTrace)> {
    match action {
        AgentAction::Click { element_id, name, desc } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let (structure, ocr) = tier_for(el);
            save_target(device, element_path, name, desc, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(device, ControlAction::Click { point: c }).await?;
            Ok((line(TksCommand::Click, vec![el_param(name)]), detail, el_trace(c, el, name)))
        }
        AgentAction::Input { element_id, name, desc, text } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let (structure, ocr) = tier_for(el);
            save_target(device, element_path, name, desc, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Input { text: text.clone(), point: Some(c) },
            )
            .await?;
            Ok((line(TksCommand::Input, vec![el_param(name), TksParam::Text(text.clone())]), detail, el_trace(c, el, name)))
        }
        AgentAction::LongPress { element_id, name, desc, duration_ms } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let (structure, ocr) = tier_for(el);
            save_target(device, element_path, name, desc, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Press { point: c, duration_ms: *duration_ms as u32 },
            )
            .await?;
            Ok((line(TksCommand::Press, vec![el_param(name), TksParam::Number(*duration_ms as i32)]), detail, el_trace(c, el, name)))
        }
        AgentAction::Clear { element_id, name, desc } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let (structure, ocr) = tier_for(el);
            save_target(device, element_path, name, desc, el.bounds.clone(), structure, ocr, tx, round).await;
            // 先点击聚焦，再清空（ControlAction::Clear 自身不带坐标）
            let _ = exec(device, ControlAction::Click { point: c }).await?;
            let detail = exec(device, ControlAction::Clear).await?;
            Ok((line(TksCommand::Clear, vec![el_param(name)]), detail, el_trace(c, el, name)))
        }
        AgentAction::ClickVisual { region, x, y, name, desc } => {
            // 看图后视觉点击：region 优先；否则 (x,y) 周围取屏宽 15% 方块
            let (sw, sh) = image::image_dimensions(shot_path).unwrap_or((1080, 1920));
            let bounds = match region {
                Some([x1, y1, x2, y2]) => Bounds::new(*x1, *y1, *x2, *y2),
                None => {
                    let cx = x.unwrap_or(sw as i32 / 2);
                    let cy = y.unwrap_or(sh as i32 / 2);
                    let half = (sw as i32 * 15 / 100).max(20) / 2;
                    Bounds::new(cx - half, cy - half, cx + half, cy + half)
                }
            };
            let c = bounds.center();
            // 三级·仅视觉：结构空、ocr 空、仅 img 模板
            save_target(device, element_path, name, desc, bounds.clone(), None, OcrChannel::None, tx, round).await;
            let detail = exec(device, ControlAction::Click { point: c }).await?;
            let trace = ActionTrace {
                captured: false,
                points: vec![c],
                bounds: Some(bounds),
                element_name: Some(name.clone()),
            };
            Ok((line(TksCommand::Click, vec![el_param(name)]), detail, trace))
        }
        AgentAction::SwipeDir { direction, distance } => {
            let (w, h) = image::image_dimensions(shot_path).unwrap_or((1080, 1920));
            let (cx, cy) = (w as i32 / 2, h as i32 / 2);
            let dist = distance.unwrap_or((h as i32) * 5 / 10);
            let from = Point::new(cx, cy);
            let to = swipe_end(from, direction, dist);
            let detail = exec(
                device,
                ControlAction::SwipeDir {
                    from,
                    direction: direction.clone(),
                    distance: dist,
                    duration_ms: 300,
                },
            )
            .await?;
            let trace = ActionTrace { captured: false, points: vec![from, to], bounds: None, element_name: None };
            Ok((
                line(
                    TksCommand::DirectionalSwipe,
                    vec![
                        TksParam::Coordinate(from),
                        TksParam::Direction(direction.clone()),
                        TksParam::Number(dist),
                    ],
                ),
                detail,
                trace,
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
            Ok((line(TksCommand::Launch, params), detail, ActionTrace::default()))
        }
        AgentAction::Close { target } => {
            let detail = exec(device, ControlAction::Close { package: target.clone() }).await?;
            Ok((line(TksCommand::Close, vec![TksParam::Text(target.clone())]), detail, ActionTrace::default()))
        }
        AgentAction::Back => {
            let detail = exec(device, ControlAction::Back).await?;
            Ok((line(TksCommand::Back, vec![]), detail, ActionTrace::default()))
        }
        AgentAction::HideKeyboard => {
            let detail = exec(device, ControlAction::HideKeyboard).await?;
            Ok((line(TksCommand::HideKeyboard, vec![]), detail, ActionTrace::default()))
        }
        AgentAction::Wait { ms, element } => {
            if let Some(ms) = ms {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(*ms as i32)]), format!("等待 {}ms", ms), ActionTrace::default()))
            } else if let Some(elem) = element {
                // v1：仅记录可回放的 .tks 行，实时不阻塞（下一轮采集会反映新状态）
                Ok((line(TksCommand::Wait, vec![el_param(elem)]), format!("记录等待元素出现: {}", elem), ActionTrace::default()))
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(1000)]), "等待 1000ms".to_string(), ActionTrace::default()))
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

/// 判断 AI 选中元素走哪级落库：
/// - OCR 伪元素（class=OcrText，仅来自 OCR 增强）→ 结构空 + ocr 用其文字（二级）
/// - 其它（真实结构元素，含被 OCR 回填文字的图标）→ 结构通道 + ocr(结构文本优先，否则裁剪图兜底)（一级）
fn tier_for(el: &UIElement) -> (Option<&UIElement>, OcrChannel) {
    if el.class_name == "OcrText" {
        let ocr = el
            .text
            .clone()
            .filter(|t| !t.trim().is_empty())
            .map(OcrChannel::Text)
            .unwrap_or(OcrChannel::FromCrop);
        (None, ocr)
    } else {
        let ocr = el
            .text
            .clone()
            .or_else(|| el.content_desc.clone())
            .filter(|t| !t.trim().is_empty())
            .map(OcrChannel::Text)
            .unwrap_or(OcrChannel::FromCrop);
        (Some(el), ocr)
    }
}

/// 元素动作的轨迹：点击点 + 元素实时 bounds（用于截图画框 + 蓝点）
fn el_trace(c: Point, el: &UIElement, name: &str) -> ActionTrace {
    ActionTrace {
        captured: false,
        points: vec![c],
        bounds: Some(el.bounds.clone()),
        element_name: Some(name.to_string()),
    }
}

/// 由起点+方向+距离推出定向滑动终点（与 execute_action 内一致）
fn swipe_end(from: Point, direction: &str, distance: i32) -> Point {
    match direction {
        "up" => Point::new(from.x, from.y - distance),
        "down" => Point::new(from.x, from.y + distance),
        "left" => Point::new(from.x - distance, from.y),
        "right" => Point::new(from.x + distance, from.y),
        _ => from,
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
