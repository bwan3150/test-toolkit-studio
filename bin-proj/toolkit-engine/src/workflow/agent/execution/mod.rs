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
pub use library::SavedInfo;
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
    _known: &[Option<String>], // 元素名改为按特征自动生成（auto_name），不再用库匹配名；参数暂留以减少 flow 改动
    shot_path: &Path,
    tx: &mut Transcript,
    round: usize,
) -> Result<(String, String, ActionTrace, Option<SavedInfo>)> {
    match action {
        // 注：desc 不在创建时写（approach A：避免想当然）；探索结束后据实际作用统一生成
        AgentAction::Click { element_id, .. } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let name = auto_name(el);
            let (structure, ocr) = tier_for(el);
            let saved = save_target(device, element_path, &name, &None, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(device, ControlAction::Click { point: c }).await?;
            Ok((line(TksCommand::Click, vec![el_param(&name)]), detail, el_trace(c, el, &name), saved))
        }
        AgentAction::Input { element_id, text, .. } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let name = auto_name(el);
            let (structure, ocr) = tier_for(el);
            let saved = save_target(device, element_path, &name, &None, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Input { text: text.clone(), point: Some(c) },
            )
            .await?;
            Ok((line(TksCommand::Input, vec![el_param(&name), TksParam::Text(text.clone())]), detail, el_trace(c, el, &name), saved))
        }
        AgentAction::LongPress { element_id, duration_ms, .. } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let name = auto_name(el);
            let (structure, ocr) = tier_for(el);
            let saved = save_target(device, element_path, &name, &None, el.bounds.clone(), structure, ocr, tx, round).await;
            let detail = exec(
                device,
                ControlAction::Press { point: c, duration_ms: *duration_ms as u32 },
            )
            .await?;
            Ok((line(TksCommand::Press, vec![el_param(&name), TksParam::Number(*duration_ms as i32)]), detail, el_trace(c, el, &name), saved))
        }
        AgentAction::Clear { element_id, .. } => {
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let name = auto_name(el);
            let (structure, ocr) = tier_for(el);
            let saved = save_target(device, element_path, &name, &None, el.bounds.clone(), structure, ocr, tx, round).await;
            // 先点击聚焦，再清空（ControlAction::Clear 自身不带坐标）
            let _ = exec(device, ControlAction::Click { point: c }).await?;
            let detail = exec(device, ControlAction::Clear).await?;
            Ok((line(TksCommand::Clear, vec![el_param(&name)]), detail, el_trace(c, el, &name), saved))
        }
        AgentAction::Assert { element_id, exist, .. } => {
            // 断言只产出可回放的**校验步**、不操作设备：当前页面 AI 已看到该元素，落库后
            // 回放时校验它是否存在；用于给脚本加闭环（到达关键页/目标页才能通过断言）。
            let el = lookup(elements, *element_id)?;
            let c = el.center();
            let name = auto_name(el);
            let (structure, ocr) = tier_for(el);
            let saved = save_target(device, element_path, &name, &None, el.bounds.clone(), structure, ocr, tx, round).await;
            let cond = if *exist { "存在" } else { "不存在" };
            let detail = format!("记录断言：{} {}", name, cond);
            Ok((line(TksCommand::Assert, vec![el_param(&name), TksParam::Text(cond.to_string())]), detail, el_trace(c, el, &name), saved))
        }
        AgentAction::ClickVisual { region, x, y, .. } => {
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
            let name = visual_auto_name(&bounds); // 自动命名，AI 不必起名
            // 三级·仅视觉：结构空、ocr 空、仅 img 模板
            let saved = save_target(device, element_path, &name, &None, bounds.clone(), None, OcrChannel::None, tx, round).await;
            let detail = exec(device, ControlAction::Click { point: c }).await?;
            let trace = ActionTrace {
                captured: false,
                points: vec![c],
                bounds: Some(bounds),
                element_name: Some(name.clone()),
            };
            Ok((line(TksCommand::Click, vec![el_param(&name)]), detail, trace, saved))
        }
        AgentAction::SwipeDir { direction, distance, amount } => {
            let (w, h) = image::image_dimensions(shot_path).unwrap_or((1080, 1920));
            let (cx, cy) = (w as i32 / 2, h as i32 / 2);
            // 滑动幅度沿方向轴的屏幕尺寸：上下用高度，左右用宽度
            let dim = match direction.as_str() {
                "left" | "right" => w as i32,
                _ => h as i32,
            };
            // amount(屏幕比例)优先；否则 distance(像素)；都没有则半屏
            let dist = match amount.as_deref() {
                Some("full") => dim * 4 / 5, // 整屏从中心只能滑 ~半，留边取 0.8 近整屏
                Some("quarter") => dim / 4,
                Some("half") => dim / 2,
                _ => distance.unwrap_or(dim / 2),
            };
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
                None,
            ))
        }
        AgentAction::SwipeToFind { target, direction } => {
            // 探索时就用"可复现的滚动找文字"（落 滚动查找 指令，回放与探索滚动位置一致）：
            // 朝 direction 循环滚一屏、采页面查 target 文字是否出现，最多 15 次。
            use crate::{Fetcher, Refresh, RefreshOptions, Workarea};
            let (w, h) = image::image_dimensions(shot_path).unwrap_or((1080, 1920));
            let from = Point::new(w as i32 / 2, h as i32 / 2);
            let dim = match direction.as_str() {
                "left" | "right" => w as i32,
                _ => h as i32,
            };
            let dist = dim / 2; // 每次滚半屏（与回放 execute_scroll_find 的力度一致取向）
            let workarea = Workarea::for_device(Some(device))?;
            let fetcher = Fetcher::new();
            let tgt = target.to_lowercase();
            let mut found = false;
            const MAX: usize = 15;
            for i in 0..=MAX {
                // 先采页面查文字是否已可见（先查再滚，避免滑过头）
                if Refresh::new(device.to_string())?.run(RefreshOptions::default()).await.is_ok() {
                    if let Ok(els) = fetcher.fetch_elements_from_file(&workarea.ui_tree_path()) {
                        if els.iter().any(|e| e.to_ai_text().to_lowercase().contains(&tgt)) {
                            found = true;
                            break;
                        }
                    }
                }
                if i == MAX {
                    break;
                }
                let _ = exec(
                    device,
                    ControlAction::SwipeDir { from, direction: direction.clone(), distance: dist, duration_ms: 300 },
                )
                .await?;
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            }
            if !found {
                return Err(TkeError::ScriptExecuteError(format!("滚动查找：滚动后仍未出现「{}」", target)));
            }
            let detail = format!("滚动找到「{}」", target);
            let trace = ActionTrace { captured: false, points: vec![from], bounds: None, element_name: None };
            Ok((
                line(TksCommand::ScrollFind, vec![TksParam::Text(target.clone()), TksParam::Direction(direction.clone())]),
                detail,
                trace,
                None,
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
            Ok((line(TksCommand::Launch, params), detail, ActionTrace::default(), None))
        }
        AgentAction::Close { target } => {
            let detail = exec(device, ControlAction::Close { package: target.clone() }).await?;
            Ok((line(TksCommand::Close, vec![TksParam::Text(target.clone())]), detail, ActionTrace::default(), None))
        }
        AgentAction::Back => {
            let detail = exec(device, ControlAction::Back).await?;
            Ok((line(TksCommand::Back, vec![]), detail, ActionTrace::default(), None))
        }
        AgentAction::Switch { target } => {
            let detail = exec(device, ControlAction::Switch { target: target.clone() }).await?;
            Ok((line(TksCommand::Switch, vec![TksParam::Text(target.clone())]), detail, ActionTrace::default(), None))
        }
        AgentAction::HideKeyboard => {
            let detail = exec(device, ControlAction::HideKeyboard).await?;
            Ok((line(TksCommand::HideKeyboard, vec![]), detail, ActionTrace::default(), None))
        }
        AgentAction::Wait { ms, element } => {
            if let Some(ms) = ms {
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(*ms as i32)]), format!("等待 {}ms", ms), ActionTrace::default(), None))
            } else if let Some(elem) = element {
                // v1：仅记录可回放的 .tks 行，实时不阻塞（下一轮采集会反映新状态）
                Ok((line(TksCommand::Wait, vec![el_param(elem)]), format!("记录等待元素出现: {}", elem), ActionTrace::default(), None))
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                Ok((line(TksCommand::Wait, vec![TksParam::Number(1000)]), "等待 1000ms".to_string(), ActionTrace::default(), None))
            }
        }
        // 控制流动作不在此处理（由主循环拦截）
        AgentAction::RequestScreenshot { .. }
        | AgentAction::AskUser { .. }
        | AgentAction::Finish { .. }
        | AgentAction::Rename { .. } => {
            Err(TkeError::ScriptExecuteError("控制流动作不应进入执行器".to_string()))
        }
    }
}

/// 临时库元素**自动命名**：用元素稳定特征（可见文字/描述/资源id/类名，截断）+ 中心坐标合成 key。
/// AI 全程不必起名、也不被名字误导；临时库重复无所谓——定稿提交时再由语义改名 pass 起正式名。
/// 名字里不含 .tks 特殊字符（{}[]、逗号），避免破坏脚本解析（坐标用下划线连接）。
pub(crate) fn auto_name(el: &UIElement) -> String {
    let feat = el
        .text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| el.content_desc.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .or_else(|| el.resource_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or(el.class_name.as_str());
    let feat: String = feat
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\t' | '{' | '}' | '[' | ']' | ','))
        .take(24)
        .collect();
    let c = el.center();
    format!("{}@{}_{}", feat.trim(), c.x, c.y)
}

/// 视觉元素（看图框选、无结构）自动命名：用框中心坐标。
pub(crate) fn visual_auto_name(b: &Bounds) -> String {
    let c = b.center();
    format!("visual@{}_{}", c.x, c.y)
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
