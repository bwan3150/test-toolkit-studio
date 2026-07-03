// 【脚本医生 / 编辑器 agent】—— 代码 agent 式的脚本修复 + 提炼。三分结构：
//   mod.rs        doctor_repair 主循环（会话/编辑调度/护栏/收尾校验）
//   diagnosis.rs  诊断回放（每步留页面的富 trace + 无效步/页面重复分析 + trace 提示词）
//   edits.rs      编辑工具（EditOp/schema/行校验/元素重选辅助）
//
// 取代旧的「断点续接修复(repair_once) + 候选删冗余(minimize_candidates)」。把两件事
// 统一成一个**专职会话**(独立工具集)的主循环：
//   ① 诊断回放：整脚本跑一遍，**每步都留下页面**(结构 + OCR)，产出富 trace；
//   ② 把「编号脚本 + 富 trace」交给医生 agent，它用编辑工具**改任意行**：
//        delete_lines / replace_line / insert_after —— 纯文本编辑(删/改/插)，
//          其中 replace/insert 只准引用**已在元素库里的元素**(校验，杜绝凭空捏造)；
//        reexplore —— 需要全新导航/新元素时的**活体逃生口**：回放前缀定位设备后
//          现场重探那一段再拼回(复用探索会话，有设备知识)；
//        run —— 重新诊断回放(医生「测试自己的修改」的方式)；
//        finish —— 改完收尾(系统仍以诊断兜底校验)。
//   ③ 自动护栏：启动步不许删/覆盖；replace/insert 校验元素引用；finish 后诊断兜底 + 终点语义校验。
//
// 与旧版的本质区别：旧版只会「从某点把尾巴整段重探」，删不掉中间冗余步、改不了单行坏参数、
// 也看不到每步页面。医生能看每步去了哪、做最小必要的外科手术。

mod diagnosis;
mod edits;

pub(super) use diagnosis::diagnose;

use std::path::Path;
use std::sync::Arc;

use crate::tools::element::{add_element_target, OcrChannel};
use crate::{AiConfig, LlmReply, LlmSession, Params, Platform};

use super::super::execution::{auto_name, visual_auto_name};
use super::super::perception::{capture, render_element_list};
use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::super::ui::{Level, Phase, SubAgent, Tokens, UiCommand, UiEvent};
use super::ctx::DriveCtx;
use super::fmt::{brief, friendly, is_launch_line, parse_desc_json};
use super::options::VerifyReport;
use diagnosis::render_trace_prompt;
use edits::{build_action_line, build_doctor_tools, parse_edit, reposition, tier_for, validate_line, EditOp, PendingReselect};

// 医生主循环诊断轮数上限来自 config [harness].doctor_iters（params.harness.doctor_iters）。
/// 单轮内医生最多调用几次 LLM(防在编辑里空转)
const MAX_EDITS_PER_ITER: usize = 24;

/// 脚本医生主循环：把 lines 修到稳定到达目标(顺带提炼最短)。
/// 返回**最短的达标版本**；始终修不好则返回 None(上层据此判失败、回滚)。
#[allow(clippy::too_many_arguments)]
pub(super) async fn doctor_repair(
    ai: &AiConfig,
    prompts: &PromptSet,
    txp: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    marker: &str,
    mut lines: Vec<String>,
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    // 进入「脚本医生 agent」作用域：本函数产出的事件（会话/请求/编辑/诊断/重选元素）都归属 doctor。
    let mut _dscope = txp.scoped("doctor");
    let tx = &mut *_dscope;

    // 独立医生会话(独立工具集)。系统提示词与工具描述均走 PromptSet（内置 md，可外部覆盖）。
    // 其 token 单独累计、最后并入总量(报告 extra_*)。
    let platform = Platform::from_device(Some(ctx.device));
    let system = prompts.role_system("doctor", ctx.device, platform.name());
    let tools = build_doctor_tools(prompts);
    let mut editor = match LlmSession::new_for_role(ai, "doctor", system.clone(), tools.clone()) {
        Ok(s) => s,
        Err(e) => {
            ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("脚本医生会话创建失败：{}", e) });
            return None;
        }
    };
    // 记录医生会话(独立 agent)的系统提示词 + 工具定义，便于在 conversation.json 里按 agent 复盘
    tx.log(
        "doctor_session",
        serde_json::json!({            "model": editor.model(),
            "system_prompt": system,
            "tools": tools.iter().map(|t| serde_json::json!({ "name": t.name, "description": t.description, "schema": t.schema })).collect::<Vec<_>>(),
        }),
    );

    let mut stagnation = 0usize; // 连续「没改动且没达标」次数
    let mut pending: Option<PendingReselect> = None; // reexplore 定位后暂存的实时页面（供 pick）
    let mut finish_rechecks = 0usize; // finish 终点校验被打回的次数（防无限）

    for iter in 1..=params.harness.doctor_iters {
        if super::interrupt::aborted() {
            ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已中断（Ctrl+C），停止医生修复".into() });
            return None;
        }
        ctx.ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
        ctx.ui.emit(UiEvent::Notice { level: Level::Info, text: format!("▶ 诊断回放（第 {} 轮，重启净化中…）", iter) });
        let diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;

        // 医生只管**正确性**：一旦能跑通且到达目标，立即把正确脚本交回上层（路径优化是反思官的事）
        if diag.reached {
            tx.log("doctor_reached", serde_json::json!({ "iter": iter, "steps": lines.len() }));
            return Some(lines);
        }

        let prompt = render_trace_prompt(prompts, case, marker, &diag);
        tx.log("doctor_request", serde_json::json!({ "iter": iter, "reached": false, "prompt": prompt.clone() }));
        // user_trace：自动省略上一轮 trace 的页面详情，只保留最新一份完整 trace（防上下文暴涨）
        editor.user_trace(prompt);

        // 内层：收医生的编辑动作，直到它 run / finish / 触发重新诊断
        let lines_before = lines.clone();
        let mut did_reexplore = false;
        let mut edit_calls = 0usize;
        let mut go_finish = false;
        loop {
            // 安全点：取前端命令（Abort 终止；Guidance 即时注入医生会话；Pause 软停等用户给医生建议再继续）。
            // 回放/滚动查找被 Esc 软停后控制权回到本循环，下一轮顶部就能 drain 到 Pause（仿 flow.rs）。
            for c in ctx.ui.drain_commands() {
                match c {
                    UiCommand::Abort => {
                        ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已终止（用户中断），停止医生修复".into() });
                        return None;
                    }
                    UiCommand::Guidance { text } => {
                        let m = format!("【用户给诊断医生的建议】{}\n请据此调整诊断/修复方向。", text);
                        tx.log("user_guidance", serde_json::json!({ "iter": iter, "content": m.clone() }));
                        editor.user(m);
                        ctx.ui.emit(UiEvent::GuidanceAccepted { text });
                    }
                    UiCommand::Pause => {
                        super::interrupt::clear_pause(); // 已响应软停，清标志（回放/诊断动作已停下）
                        match ctx
                            .ui
                            .await_answer(0, "已暂停。给诊断医生一条建议（或回车让它自行诊断），再按 Ctrl+C 退出".to_string())
                            .await
                        {
                            Some(g) if !g.trim().is_empty() => {
                                let m = format!("【用户给诊断医生的建议】{}\n请据此调整诊断/修复方向。", g);
                                tx.log("user_guidance", serde_json::json!({ "iter": iter, "content": m.clone() }));
                                editor.user(m);
                                ctx.ui.emit(UiEvent::GuidanceAccepted { text: g });
                            }
                            Some(_) => {}
                            None => {
                                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已终止（用户中断），停止医生修复".into() });
                                return None;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if super::interrupt::aborted() {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已中断（Ctrl+C），停止医生修复".into() });
                return None;
            }
            if edit_calls >= MAX_EDITS_PER_ITER {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "医生单轮编辑过多，强制重新诊断".into() });
                break;
            }
            edit_calls += 1;
            let reply = match editor.next().await {
                Ok(r) => r,
                Err(e) => {
                    ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("医生决策出错：{}", e) });
                    return None;
                }
            };
            let (pt, ct) = editor.last_usage();
            // 医生会话独立计 token，累进报告的 extra（最终并入总量统计）
            report.extra_prompt += pt;
            report.extra_completion += ct;

            let (text, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 160);
                    ctx.ui.emit(UiEvent::SubAgent {
                        kind: SubAgent::Doctor,
                        level: Level::Dim,
                        text: if b.is_empty() { "（未调用工具）".into() } else { b },
                        tokens: Tokens::new(pt, ct),
                    });
                    let m = prompts.message("doctor", "nudge_use_tool");
                    tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
                    editor.user(m);
                    continue;
                }
                LlmReply::ToolCalls { text, calls } => (text, calls),
            };
            // 协议：所有 tool_call 都要回执，只处理第一个
            let primary = calls[0].clone();
            for extra in &calls[1..] {
                editor.tool_result(extra.call_id.as_str(), "已忽略：每轮仅处理第一个工具调用");
            }
            let op = match parse_edit(&primary) {
                Ok(v) => v,
                Err(e) => {
                    editor.tool_result(primary.call_id.as_str(), format!("参数错误：{}", e));
                    continue;
                }
            };
            // 一行思考：理由优先(reason 字段)，其次模型同时给的文字；工具名用紫色标签区分
            let reason = primary
                .arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let say = reason
                .map(|s| brief(s, 180))
                .or_else(|| text.as_deref().map(|s| brief(s, 180)).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "（未说明理由）".to_string());
            ctx.ui.emit(UiEvent::SubAgent {
                kind: SubAgent::Doctor,
                level: Level::Info,
                text: format!("⟫ {}：{}", primary.name, say),
                tokens: Tokens::new(pt, ct),
            });
            // 编辑前脚本全貌（变更后由各 arm 记 doctor_edit_applied）
            let script_before = lines.clone();
            tx.log(
                "doctor_edit",
                serde_json::json!({ "iter": iter, "tool": primary.name.clone(), "reason": reason, "thinking": text.clone(), "args": primary.arguments.clone(), "script_before": script_before }),
            );

            match op {
                EditOp::Delete { from, to } => {
                    let n = lines.len();
                    if from < 1 || to < from || from > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法删 {}..={}", n, from, to));
                        continue;
                    }
                    let to = to.min(n);
                    // 保护启动步：删除范围内若含「启动」步则拒绝——删了浏览器/App 再也起不来、重启净化也空转。
                    if lines[(from - 1)..to].iter().any(|l| is_launch_line(l)) {
                        editor.tool_result(primary.call_id.as_str(), "删除范围里含「启动」步——它打开浏览器/拉起 App，删了脚本就再也起不来、重启净化也会空转。请缩小范围、保留启动步。");
                        continue;
                    }
                    let removed: Vec<String> = lines.drain((from - 1)..to).collect();
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 删第 {}-{} 行  {}", from, to, removed.iter().map(|l| friendly(l)).collect::<Vec<_>>().join(" / ")) });
                    editor.tool_result(primary.call_id.as_str(), format!("已删第 {}-{} 行，脚本现 {} 行。改完记得 run 验证。", from, to, lines.len()));
                }
                EditOp::Replace { line, content } => {
                    let n = lines.len();
                    if line < 1 || line > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法替换第 {} 行", n, line));
                        continue;
                    }
                    // 保护启动步：启动步只能改成另一个「启动」(如换网址/App)，不能改成点击/其它，否则浏览器/App 起不来。
                    if is_launch_line(&lines[line - 1]) && !is_launch_line(&content) {
                        editor.tool_result(primary.call_id.as_str(), format!("第 {} 步是「启动」步，只能改成另一个「启动 …」(如换网址/App)，不能改成点击/其它——否则浏览器/App 起不来。若是导航不对，请 insert_after 在它之后插入导航步。", line));
                        continue;
                    }
                    if let Err(why) = validate_line(&content, ctx.element_path) {
                        editor.tool_result(primary.call_id.as_str(), format!("替换被拒：{}", why));
                        continue;
                    }
                    let old = std::mem::replace(&mut lines[line - 1], content.trim().to_string());
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 改第 {} 行  {} → {}", line, friendly(&old), friendly(&lines[line - 1])) });
                    editor.tool_result(primary.call_id.as_str(), format!("已替换第 {} 行。改完记得 run 验证。", line));
                }
                EditOp::Insert { after, content } => {
                    let n = lines.len();
                    if after > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法在第 {} 行后插入", n, after));
                        continue;
                    }
                    if let Err(why) = validate_line(&content, ctx.element_path) {
                        editor.tool_result(primary.call_id.as_str(), format!("插入被拒：{}", why));
                        continue;
                    }
                    lines.insert(after, content.trim().to_string());
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 在第 {} 行后插入  {}", after, friendly(&lines[after])) });
                    editor.tool_result(primary.call_id.as_str(), format!("已插入，脚本现 {} 行。改完记得 run 验证。", lines.len()));
                }
                EditOp::Reexplore { step, reason } => {
                    if report.repairs >= params.harness.repairs {
                        editor.tool_result(primary.call_id.as_str(), format!("已达重选上限（{} 次），请改用文本编辑或 finish。", params.harness.repairs));
                        continue;
                    }
                    let n = lines.len();
                    if step < 1 || step > n {
                        editor.tool_result(primary.call_id.as_str(), format!("step 越界：脚本共 {} 行", n));
                        continue;
                    }
                    // 保护启动步：第 step 步是「启动」(打开浏览器/拉起 App)时不能 reexplore——
                    // reexplore 之后的 pick 会用点击覆盖这一步，毁掉脚本入口、后续重启净化空转。
                    if is_launch_line(&lines[step - 1]) {
                        editor.tool_result(primary.call_id.as_str(), format!(
                            "第 {} 步是「启动」步(打开浏览器/拉起 App)，不能 reexplore 重选成点击——\
                             那会用点击覆盖启动步、毁掉脚本入口，后续重启净化也会因找不到启动目标而空转。\
                             若是「启动后导航入口不对」：保留这一步，用 insert_after 在它之后插入正确的导航步；\
                             若要换网址/App：用 replace_line 把它改成另一个「启动 …」。", step));
                        continue;
                    }
                    report.repairs += 1;
                    let cut = step - 1; // 回放到目标步的前一步
                    tx.log("doctor_reexplore", serde_json::json!({ "iter": iter, "step": step, "kept_prefix": cut, "reason": reason }));
                    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("◆ 重新定位到第 {} 步（重启+回放前 {} 步）：{}", step, cut, brief(&reason, 50)) });
                    reposition(ctx, params, script_path, case, &lines, cut).await;
                    // fetch 实时页面，交给医生重选
                    match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
                        Ok(p) => {
                            let none = vec![None; p.elements.len()];
                            let list = render_element_list(&p.elements, &none, &prompts.message("explorer", "element_tag"));
                            let count = p.elements.len();
                            let shot = p.shot_path.clone();
                            pending = Some(PendingReselect { step, elements: p.elements, shot_path: shot.clone() });
                            editor.tool_result(primary.call_id.as_str(), "已定位到该步实时页面（元素列表 + 截图见下一条消息）");
                            // 同时发**元素列表 + 截图**：列表里有就 pick(id)，列表里没有/反复点不中（纯图标、
                            // 滑动没到位）就看截图用 pick_visual 给像素框/点。
                            let msg = format!(
                                "已回放到第 {} 步、设备停在第 {} 步将操作的实时页面。\n实时元素（共 {} 个）：\n{}\n\n\
                                 请为第 {} 步选正确目标：\n\
                                 · 目标在上面列表里 → pick(id, name, action)；\n\
                                 · 列表里没有、或之前反复点这个元素都没反应（很可能是纯图标、或目标被滑出屏幕外）→ \
                                 看下面的截图用 pick_visual 给像素框 region=[x1,y1,x2,y2]（优先）或点 (x,y)。\n\
                                 input 动作再给 text。",
                                cut, step, count, list, step
                            );
                            tx.log("llm_message", serde_json::json!({ "iter": iter, "content": msg.clone(), "image": shot.to_string_lossy() }));
                            if editor.user_with_image(&msg, &shot).is_err() {
                                editor.user(msg);
                            }
                        }
                        Err(e) => {
                            editor.tool_result(primary.call_id.as_str(), format!("定位后采集页面失败：{}。可重试 reexplore 或改用文本编辑。", e));
                        }
                    }
                    continue; // 等医生 pick / pick_visual；不重诊断
                }
                EditOp::Pick { id, name, action, text } => {
                    let Some(pend) = pending.as_ref() else {
                        editor.tool_result(primary.call_id.as_str(), "还没定位：请先 reexplore 到要改的步骤，拿到实时元素再 pick。");
                        continue;
                    };
                    let Some(el) = pend.elements.get(id) else {
                        editor.tool_result(primary.call_id.as_str(), format!("id 越界：该页共 {} 个元素", pend.elements.len()));
                        continue;
                    };
                    // 元素名按特征自动生成（与探索一致），医生不必起名；AI 传的 name 忽略
                    let _ = &name;
                    let eff_name = auto_name(el);
                    let line = match build_action_line(&action, &eff_name, text.as_deref()) {
                        Ok(l) => l,
                        Err(why) => {
                            editor.tool_result(primary.call_id.as_str(), why);
                            continue;
                        }
                    };
                    // 实时落库（从 reexplore 那次 capture 写入工作区的截图裁 img）
                    let (structure, ocr) = tier_for(el);
                    if let Err(e) =
                        add_element_target(ctx.device.to_string(), ctx.element_path, &eff_name, None, el.bounds.clone(), structure, ocr, false).await
                    {
                        editor.tool_result(primary.call_id.as_str(), format!("元素落库失败：{}", e));
                        continue;
                    }
                    if !report.created.contains(&eff_name) {
                        report.created.push(eff_name.clone());
                    }
                    let step_idx = pend.step - 1;
                    let old = lines.get(step_idx).cloned().unwrap_or_default();
                    if step_idx < lines.len() {
                        lines[step_idx] = line.clone();
                    }
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 重选第 {} 步元素  {} → {}", pend.step, friendly(&old), friendly(&line)) });
                    tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": "pick", "step": pend.step, "name": eff_name, "script_after": lines.clone() }));
                    editor.tool_result(primary.call_id.as_str(), format!("已把第 {} 步改为「{}」（元素「{}」已实时存库）。请 run 验证。", pend.step, friendly(&line), eff_name));
                    pending = None;
                    did_reexplore = true;
                    break; // 重选后重新诊断
                }
                EditOp::PickVisual { region, x, y, name, action, text } => {
                    let _ = &name; // 元素名按特征(坐标)自动生成，AI 传的 name 忽略
                    let Some(pend) = pending.as_ref() else {
                        editor.tool_result(primary.call_id.as_str(), "还没定位：请先 reexplore 到要改的步骤，看到截图再 pick_visual。");
                        continue;
                    };
                    // 看图框选 → 像素 bounds（region 优先；否则以 (x,y) 取屏宽 15% 方块），落纯 img 元素
                    let (sw, sh) = image::image_dimensions(&pend.shot_path).unwrap_or((1080, 1920));
                    let bounds = match region {
                        Some([x1, y1, x2, y2]) => crate::Bounds::new(x1, y1, x2, y2),
                        None => {
                            let cx = x.unwrap_or(sw as i32 / 2);
                            let cy = y.unwrap_or(sh as i32 / 2);
                            let half = (sw as i32 * 15 / 100).max(20) / 2;
                            crate::Bounds::new(cx - half, cy - half, cx + half, cy + half)
                        }
                    };
                    let name = visual_auto_name(&bounds); // 自动命名
                    let line = match build_action_line(&action, &name, text.as_deref()) {
                        Ok(l) => l,
                        Err(why) => {
                            editor.tool_result(primary.call_id.as_str(), why);
                            continue;
                        }
                    };
                    // 三级·仅视觉：结构空、ocr 空、仅 img 模板（从 reexplore 那帧截图裁）
                    if let Err(e) =
                        add_element_target(ctx.device.to_string(), ctx.element_path, &name, None, bounds, None, OcrChannel::None, false).await
                    {
                        editor.tool_result(primary.call_id.as_str(), format!("视觉元素落库失败：{}", e));
                        continue;
                    }
                    if !report.created.contains(&name) {
                        report.created.push(name.clone());
                    }
                    let step_idx = pend.step - 1;
                    let old = lines.get(step_idx).cloned().unwrap_or_default();
                    if step_idx < lines.len() {
                        lines[step_idx] = line.clone();
                    }
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 视觉重选第 {} 步元素  {} → {}", pend.step, friendly(&old), friendly(&line)) });
                    tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": "pick_visual", "step": pend.step, "name": name, "script_after": lines.clone() }));
                    editor.tool_result(primary.call_id.as_str(), format!("已把第 {} 步改为视觉点击「{}」（img 模板已存库）。请 run 验证。", pend.step, friendly(&line)));
                    pending = None;
                    did_reexplore = true;
                    break;
                }
                EditOp::Run => {
                    editor.tool_result(primary.call_id.as_str(), "重新回放诊断中，稍后给你最新 trace。");
                    break;
                }
                EditOp::Finish { reason } => {
                    editor.tool_result(primary.call_id.as_str(), "收到收尾请求，系统再诊断一次兜底校验。");
                    tx.log("doctor_finish", serde_json::json!({ "iter": iter, "reason": reason }));
                    go_finish = true;
                    break;
                }
            }
            // 走到这里 = 文本编辑(删/改/插)成功落地（失败/越界已 continue、reexplore/run/finish 已 break）：
            // 记录变更后脚本全貌，与 doctor_edit 的 script_before 对照。
            tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": primary.name.clone(), "script_after": lines.clone() }));
        }

        // 收尾：finish 后再诊断一次确认是否真达标；达标即返回正确脚本，否则打回继续修
        if go_finish {
            ctx.ui.emit(UiEvent::Notice { level: Level::Info, text: "▶ 收尾兜底诊断（重启净化中…）".into() });
            let final_diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;
            if final_diag.reached {
                // 终点校验：把最终页面 + 用户原话需求发给医生判断是否真到对的地方（marker 命中只是文本，
                // 这里再做一层语义判断，防"标志在但页面不对"）。被打回有上限，避免无限。
                if finish_rechecks < 2 && !final_diag.final_page.trim().is_empty() {
                    finish_rechecks += 1;
                    let check = render(&prompts.message("doctor", "finish_check"), &[("case", case), ("page", &final_diag.final_page)]);
                    tx.log("llm_message", serde_json::json!({ "iter": iter, "content": check.clone() }));
                    editor.user(check);
                    if let Ok(LlmReply::Text(t)) = editor.next().await {
                        let (pt, ct) = editor.last_usage();
                        report.extra_prompt += pt;
                        report.extra_completion += ct;
                        tx.log("finish_check", serde_json::json!({ "iter": iter, "content": t.clone() }));
                        if let Some(obj) = parse_desc_json(&t) {
                            if obj["done"].as_bool() == Some(false) {
                                let why = obj["reason"].as_str().unwrap_or("最终页面不符合用户需求").to_string();
                                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("终点校验未通过：{}", brief(&why, 80)) });
                                let m = format!("终点校验未通过：{}。脚本最终到的页面不是用户要的目的地，请继续修（看下面的最新 trace）。", why);
                                tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
                                editor.user(m);
                                continue; // 不收尾，继续修
                            }
                        }
                    }
                }
                tx.log("doctor_reached", serde_json::json!({ "iter": iter, "steps": lines.len() }));
                return Some(lines);
            }
            let m = prompts.message("doctor", "finish_pushback");
            tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
            editor.user(m);
            // 落到下一轮
        }

        // 停滞检测：本轮没改动、没重探、且没达标 → 计一次停滞；连续 2 次就收手（修不动了）
        let unchanged = lines == lines_before;
        if unchanged && !did_reexplore && !diag.reached {
            stagnation += 1;
            if stagnation >= 2 {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "医生连续两轮无有效改动且未达标，放弃修复".into() });
                return None;
            }
        } else {
            stagnation = 0;
        }
    }

    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("已达医生诊断轮数上限（{} 轮）仍未修到目标", params.harness.doctor_iters) });
    None
}
