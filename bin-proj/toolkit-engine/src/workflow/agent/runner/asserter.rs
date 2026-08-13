// 【断言官】子 agent（单次调用、工具型）：探索主 agent 每做一次"让页面变化了"的导航点击/切换后，
// 由它根据页面 diff（新出现的元素）挑一个**能证明这次点击有效、确实进到了预期新页面**的独有标志元素，
// 系统据此自动插入一条断言把这步踩实——把"断言"从主 agent 手里拿走，避免主 agent 漏断言。
// 独立会话(无工具)，按作用域归属 "asserter"；token 单独计、并入总量。

use crate::{AiConfig, Platform, StepResult};

use super::super::execution;
use super::super::perception::Perceived;
use super::super::prompt::{render, PromptSet};
use super::super::tools::AgentAction;
use super::super::transcript::Transcript;
use super::super::ui::{Level, StepState, SubAgent, Tokens, UiEvent};
use super::ctx::DriveCtx;
use super::fmt::{friendly, preview_action};
use super::fmt::brief;

/// 探索循环中被踩实写入的脚本状态（auto_checkpoint 借用 drive 的可变局部）
pub(super) struct CheckpointScript<'a> {
    pub lines: &'a mut Vec<String>,
    pub step_comments: &'a mut Vec<String>,
    pub steps: &'a mut Vec<StepResult>,
    pub created: &'a mut Vec<String>,
}

/// 探索循环的「踩实」全流程（从 flow.rs 拆出）：上一步导航让页面变了 → 断言官从**本次新出现
/// 的元素**里挑一个标志（只喂新增、带真实序号、上限 80——两页全量元素纯属烧 token），系统自动
/// 插一条断言步。无新增元素=无从踩实，整次 LLM 调用直接省掉；挑不出=疑似点错，只记日志。
/// 返回断言官本次消耗的 (prompt_tokens, completion_tokens)。
#[allow(clippy::too_many_arguments)]
pub(super) async fn auto_checkpoint(
    ctx: &DriveCtx<'_>,
    tx: &mut Transcript,
    p: &Perceived,
    delta_indices: &[usize],
    known_names: &[Option<String>],
    round: usize,
    s: CheckpointScript<'_>,
) -> (i64, i64) {
    if delta_indices.is_empty() {
        tx.log("checkpoint_skip", serde_json::json!({ "round": round, "reason": "页面变了但没有新出现的元素，无从踩实" }));
        return (0, 0);
    }
    if super::interrupt::aborted() {
        return (0, 0);
    }
    let action_desc = s.lines.last().map(|l| friendly(l)).unwrap_or_default();
    // 只列新出现的元素（序号 = 它在当前页元素列表里的真实下标）；上限防"整页全新"时刷爆
    const MAX_DELTA_LIST: usize = 80;
    let mut new_listing = delta_indices
        .iter()
        .take(MAX_DELTA_LIST)
        .map(|&i| format!("[{}] {}", i, brief(&p.elements[i].to_ai_text(), 100)))
        .collect::<Vec<_>>()
        .join("\n");
    if delta_indices.len() > MAX_DELTA_LIST {
        new_listing.push_str(&format!("\n（其余 {} 个新元素略）", delta_indices.len() - MAX_DELTA_LIST));
    }
    let pick = pick_checkpoint(ctx.ai, ctx.prompts, tx, ctx.device, ctx.case, &action_desc, p.elements.len(), delta_indices.len(), &new_listing).await;
    // 先 emit 一句「断言官的判断 + 它本次的 token 用量」
    ctx.ui.emit(UiEvent::SubAgent {
        kind: SubAgent::Asserter,
        level: if pick.index.is_some() { Level::Info } else { Level::Warn },
        text: if pick.reason.is_empty() { "（断言官未说明理由）".to_string() } else { pick.reason.clone() },
        tokens: Tokens::new(pick.pt, pick.ct),
    });
    match pick.index {
        Some(idx) if idx < p.elements.len() => {
            let act = AgentAction::Assert { element_id: idx, name: String::new(), desc: None, exist: true };
            let step_no = s.steps.len() + 1;
            ctx.ui.emit(UiEvent::Step {
                step: step_no,
                state: StepState::Running,
                preview: preview_action(&act, &p.elements).unwrap_or_else(|| "断言".into()),
                line: None,
                duration_ms: None,
                error: None,
            });
            let t0 = std::time::Instant::now();
            match execution::apply(ctx.device, ctx.element_path, &act, &p.elements, known_names, &p.shot_path, tx, round).await {
                Ok((line, detail, trace, saved)) => {
                    ctx.ui.emit(UiEvent::Step {
                        step: step_no,
                        state: StepState::Ok,
                        preview: preview_action(&act, &p.elements).unwrap_or_else(|| "断言".into()),
                        line: Some(line.clone()),
                        duration_ms: Some(t0.elapsed().as_millis() as u64),
                        error: None,
                    });
                    if let Some(sv) = saved {
                        if sv.created && !s.created.contains(&sv.name) {
                            s.created.push(sv.name.clone());
                        }
                    }
                    let step_index = s.steps.len();
                    let (screenshot, xml) = ctx.artifacts.save_step(ctx.workarea, step_index, &trace, &line, true);
                    tx.log("tks_step", serde_json::json!({ "round": round, "step": step_index + 1, "line": line.clone(), "exec": detail.clone(), "screenshot": screenshot.clone(), "xml": xml.clone(), "auto_checkpoint": true, "reason": pick.reason.clone() }));
                    s.steps.push(StepResult { index: step_index, command: line.clone(), success: true, error: None, duration_ms: 0, line: None, screenshot, xml, healed: None, note: None });
                    s.lines.push(line.clone());
                    s.step_comments.push(format!("断言：{}", pick.reason));
                }
                Err(e) => {
                    ctx.ui.emit(UiEvent::Step {
                        step: step_no,
                        state: StepState::Fail,
                        preview: preview_action(&act, &p.elements).unwrap_or_else(|| "断言".into()),
                        line: None,
                        duration_ms: Some(t0.elapsed().as_millis() as u64),
                        error: Some(format!("断言跳过：{}", e)),
                    });
                }
            }
        }
        _ => {
            // index=null：断言官判定没到预期页/无可断言标志（理由已在上面那行输出），只记日志
            tx.log("checkpoint_skip", serde_json::json!({ "round": round, "reason": pick.reason }));
        }
    }
    (pick.pt, pick.ct)
}

/// 断言官的选择结果
pub(super) struct Pick {
    /// 选中的元素序号（当前页元素列表里的 index）；None=这页找不到可踩实的标志（可能上一步点错了）
    pub index: Option<usize>,
    /// 依据：选中=为什么这是该页独有标志；None=为什么找不到/疑似点错
    pub reason: String,
    pub pt: i64,
    pub ct: i64,
}

/// 让断言官挑一个标志元素断言。只给它**本次新出现的元素**（new_listing，序号是各元素在
/// 当前页元素列表里的真实下标）+ 页面/新增计数——它本就该从新增里挑标志，两页全量元素
/// 纯属烧 token（每次导航一个独立会话）。会话/解析失败返回 index=None（上层跳过）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn pick_checkpoint(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    device: &str,
    case: &str,
    action_desc: &str,
    total_elements: usize,
    new_count: usize,
    new_listing: &str,
) -> Pick {
    let platform = Platform::from_device(Some(device));
    let mut scope = tx.scoped("asserter");
    let tx = &mut *scope;

    let system = prompts.role_system("asserter", device, platform.name());
    tx.log("asserter_session", serde_json::json!({ "system_prompt": system.clone() }));

    let prompt = render(
        &prompts.message("asserter", "pick"),
        &[
            ("case", case),
            ("action", action_desc),
            ("total", &total_elements.to_string()),
            ("news", &new_count.to_string()),
            ("current", new_listing),
        ],
    );
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    // 强制工具调用（schema 供应商侧校验），取代文字 JSON 手术
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "index": { "type": ["integer", "null"], "description": "选中的元素序号；没有合适的独特标志就 null" },
            "reason": { "type": "string", "description": "一句话依据" }
        },
        "required": ["reason"]
    });
    let (obj, pt, ct) = super::oneshot::one_shot(ai, "asserter", system, "提交断言判断结果", schema, prompt).await;
    let Some(obj) = obj else {
        return Pick { index: None, reason: "断言官未提交结构化结果".into(), pt, ct };
    };
    tx.log("asserter_pick", serde_json::json!({ "content": obj.clone() }));
    let index = obj["index"].as_u64().map(|n| n as usize);
    let reason = obj["reason"].as_str().unwrap_or("").trim().to_string();
    Pick { index, reason, pt, ct }
}
