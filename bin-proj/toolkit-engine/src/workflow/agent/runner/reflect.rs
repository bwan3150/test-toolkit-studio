// 【探索反思官】两类职责：
//  ① 探索失败 → `reflect()` 出「重探指导」，上层带指导重探（治"绕大圈没找到"，探索阶段安全网）；
//  ② 脚本已正确 → `optimize()` 做**软优化**：看正确脚本的 trace，指出可删的绕路/冗余步并删掉
//     （大胆删、不逐条验证；删坏了由医生在下一轮复检兜底——医生⇄反思官交替收敛）。
// 独立会话(无工具)，按作用域归属 "reflector"；token 单独计、并入总量。

use std::path::Path;
use std::sync::Arc;

use crate::{AiConfig, Fetcher, LlmReply, LlmSession, LlmTool, Params, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::super::ui::{Frontend, Level, SubAgent, Tokens, UiEvent};
use super::ctx::{DriveCtx, DriveOutcome};
use super::fmt::{brief, friendly, is_assert_line, parse_desc_json};
use super::options::VerifyReport;

/// 反思产物
pub(super) struct Reflection {
    /// 报告文本（重探指导 / 绕路报告）
    pub report: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

/// 把探索全程拼成 journey 文本：步号 · 动作 · 当时的理由 · 该步所在页面(结构元素前几项)。
/// 页面取探索阶段已落盘的每步 xml（离线解析，不重放、不跑 OCR，求快）。
fn build_journey(outcome: &DriveOutcome, fetcher: &Fetcher, run_dir: &Path) -> String {
    let mut s = String::new();
    for (i, line) in outcome.lines.iter().enumerate() {
        let why = outcome
            .step_comments
            .get(i)
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .unwrap_or("（无）");
        let page = outcome
            .steps
            .get(i)
            .and_then(|st| st.xml.as_ref())
            .map(|rel| run_dir.join(rel))
            .and_then(|p| fetcher.fetch_elements_from_file(&p).ok())
            .map(|els| {
                els.iter()
                    .take(6)
                    .map(|e| e.to_ai_text())
                    .filter(|t| !t.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        s.push_str(&format!("{}. {}　理由：{}", i + 1, friendly(line), brief(why, 80)));
        if !page.trim().is_empty() {
            s.push_str(&format!("　页面：{}", brief(&page, 120)));
        }
        s.push('\n');
    }
    s
}

/// 复盘一轮探索。success=true 用 analyze_success(给 doctor 的绕路报告)，否则 analyze_failure(重探指导)。
/// 返回 None 表示无产物（空脚本 / 会话创建失败 / LLM 没给文本）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn reflect(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    device: &str,
    fetcher: &Fetcher,
    run_dir: &Path,
    case: &str,
    outcome: &DriveOutcome,
    success: bool,
) -> Option<Reflection> {
    if outcome.lines.is_empty() {
        return None;
    }
    let platform = Platform::from_device(Some(device));
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;

    let system = prompts.role_system("reflector", device, platform.name());
    let mut sess = LlmSession::new(ai, system.clone(), Vec::new()).ok()?;
    tx.log("reflector_session", serde_json::json!({ "system_prompt": system }));

    let journey = build_journey(outcome, fetcher, run_dir);
    let template = if success { "analyze_success" } else { "analyze_failure" };
    let prompt = render(&prompts.message("reflector", template), &[("case", case), ("journey", &journey)]);
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    sess.user(prompt);

    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    let (prompt_tokens, completion_tokens) = sess.total_usage();
    tx.log(
        "reflector_report",
        serde_json::json!({ "mode": if success { "success" } else { "failure" }, "content": reply.clone() }),
    );
    Some(Reflection { report: reply, prompt_tokens, completion_tokens })
}

/// 软优化：脚本已正确时，看其逐步 trace 找出可删的绕路/冗余步并删掉（大胆删、不逐条验证；
/// 删坏了由医生下一轮复检兜底）。返回优化后的脚本（有改动）或 None（无可优化/失败）。
#[allow(clippy::too_many_arguments)]
/// 脚本优化官的工具表（name + JSON schema）；description 由 tools/optimizer/*.md 提供。
fn optimizer_tool_schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "delete_step",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "step": { "type": "integer", "description": "要删的步号(1-based)，必须是空操作步" },
                    "reason": { "type": "string", "description": "为什么这步是没改变页面的空操作、删了安全" }
                },
                "required": ["step", "reason"]
            }),
        ),
        (
            "merge_swipes",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "integer", "description": "起始步号(1-based)" },
                    "to": { "type": "integer", "description": "结束步号(1-based，含)；from..to 之间必须全是「定向滑动」" },
                    "target": { "type": "string", "description": "这段滑动结束后会出现的目标可见文字" },
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "滚动方向，通常 up" },
                    "reason": { "type": "string", "description": "为什么这段是盲滑、合并更稳" }
                },
                "required": ["from", "to", "target", "direction"]
            }),
        ),
        (
            "done",
            serde_json::json!({
                "type": "object",
                "properties": { "reason": { "type": "string", "description": "一句话总结：已无可优化" } },
                "required": []
            }),
        ),
    ]
}

/// 组装优化官工具集：schema 来自 optimizer_tool_schemas，description 来自 tools/optimizer/*.md（可外部覆盖）。
fn build_optimizer_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    optimizer_tool_schemas()
        .into_iter()
        .map(|(name, schema)| LlmTool::new(name, prompts.role_tool_description("optimizer", name), schema))
        .collect()
}

pub(super) async fn optimize(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    marker: &str,
    lines: &[String],
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;

    // 诊断当前脚本拿逐步 trace（调用前应已正确；不正确则不优化）
    let diag = super::doctor::diagnose(tx, ctx, params, script_path, case, lines, marker, "reflect_diagnose", 0, false).await;
    if !diag.reached {
        return None;
    }
    // 断言步必然是「空操作」(不改变页面)，但它是「踩实校验」的承重点——绝不能当冗余删掉。
    // 先把断言步从可删 noop 集里剔除：AI 既看不到它是「可删空操作」，delete_step 也会被拦下。
    let noops: std::collections::HashSet<usize> = diag
        .noop_step_nos()
        .into_iter()
        .filter(|&s| !lines.get(s - 1).map(|l| is_assert_line(l)).unwrap_or(false))
        .collect();
    let trace = diag.trace_lines();
    let pages = diag.page_groups(); // 页面访问/重复分析：同一页反复=绕路，助判冗余
    let noop_list = {
        let mut v: Vec<usize> = noops.iter().copied().collect();
        v.sort_unstable();
        if v.is_empty() { "（无）".to_string() } else { v.iter().map(usize::to_string).collect::<Vec<_>>().join(", ") }
    };
    let numbered: String = lines.iter().enumerate().map(|(i, l)| format!("{}. {}", i + 1, friendly(l))).collect::<Vec<_>>().join("\n");

    // 独立「脚本优化官」会话——专用系统提示词（不再复用 reflect 的"只分析"提示词）。
    let platform = Platform::from_device(Some(ctx.device));
    let system = prompts.role_system("optimizer", ctx.device, platform.name());
    // 真正的工具调用（与医生同机制）：工具 schema + tools/optimizer/*.md 描述，LLM 调 tool、我回 tool_result。
    let tools = build_optimizer_tools(prompts);
    let mut sess = LlmSession::new(ai, system, tools).ok()?;
    let intro = render(
        &prompts.message("optimizer", "intro"),
        &[("case", case), ("marker", marker), ("trace", &trace), ("pages", &pages), ("noops", &noop_list), ("script", &numbered)],
    );
    tx.log("llm_message", serde_json::json!({ "content": intro.clone() }));
    sess.user(intro);

    // 逐步优化（像医生：LLM 每次只调一个工具，执行/拒绝后回 tool_result，它据此再调下一个）。
    // 在**稳定的原始行号**上工作：删除收进 removable、合并收进 merges，最后一次性重建 + 一次诊断验证，
    // 避免每改一步就重跑回放。护栏：只准删空操作步（noops），承重步当场拒。
    struct Merge { from: usize, to: usize, line: String }
    let mut removable: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut merges: Vec<Merge> = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for _ in 0..24usize {
        if super::interrupt::aborted() {
            break;
        }
        let (text, calls) = match sess.next().await {
            Ok(LlmReply::ToolCalls { text, calls }) => (text, calls),
            Ok(LlmReply::Text(_)) => {
                sess.user(prompts.message("optimizer", "nudge"));
                continue;
            }
            Err(_) => break,
        };
        let (pt, ct) = sess.last_usage();
        report.extra_prompt += pt;
        report.extra_completion += ct;
        let primary = calls[0].clone();
        for extra in &calls[1..] {
            sess.tool_result(extra.call_id.as_str(), "已忽略：每轮仅处理第一个工具调用");
        }
        let arg_u = |k: &str| primary.arguments.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let arg_s = |k: &str| primary.arguments.get(k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let raw_reason = primary.arguments.get("reason").and_then(|v| v.as_str()).map(str::trim).unwrap_or("");
        let why = if !raw_reason.is_empty() { brief(raw_reason, 160) } else { text.as_deref().map(|s| brief(s, 160)).filter(|s| !s.is_empty()).unwrap_or_else(|| "（未说明理由）".to_string()) };
        match primary.name.as_str() {
            "done" => {
                ctx.ui.emit(UiEvent::SubAgent { kind: SubAgent::Optimizer, level: Level::Ok, text: format!("✓ 优化结束：{}", why), tokens: Tokens::new(pt, ct) });
                break;
            }
            "delete_step" => {
                let step = arg_u("step");
                if step < 1 || step > lines.len() {
                    sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_step_oob"), &[("step", &step.to_string()), ("total", &lines.len().to_string())]));
                    continue;
                }
                if removable.contains(&step) {
                    sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_step_dup"), &[("step", &step.to_string())]));
                    continue;
                }
                // 断言步是「踩实校验」承重点，永不删（虽然它不改变页面、看着像空操作）
                if is_assert_line(&lines[step - 1]) {
                    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("↩ 第 {} 步是断言(踩实校验)，不删", step) });
                    sess.tool_result(primary.call_id.as_str(), format!("第 {} 步是断言(踩实校验)——它是脚本自检的承重点，删了回放时「走错页」就不会在断言处暴露、问题定位会变难，不能删。", step));
                    continue;
                }
                if !noops.contains(&step) {
                    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("↩ 第 {} 步改变了页面（承重步），不删", step) });
                    sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_step_loadbearing"), &[("step", &step.to_string())]));
                    continue;
                }
                removable.insert(step);
                ctx.ui.emit(UiEvent::SubAgent { kind: SubAgent::Optimizer, level: Level::Info, text: format!("⟫ 删第 {} 步：{}（{}）", step, why, friendly(&lines[step - 1])), tokens: Tokens::new(pt, ct) });
                sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_step_deleted"), &[("step", &step.to_string())]));
            }
            "merge_swipes" => {
                let from = arg_u("from");
                let to = arg_u("to");
                let target = arg_s("target");
                let direction = arg_s("direction");
                let direction = if direction.is_empty() { "up".to_string() } else { direction };
                if from < 1 || to < from || to > lines.len() || target.is_empty() {
                    sess.tool_result(primary.call_id.as_str(), prompts.message("optimizer", "fb_merge_invalid"));
                    continue;
                }
                if !lines[from - 1..to].iter().all(|l| l.trim_start().starts_with("定向滑动")) {
                    sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_merge_notswipe"), &[("from", &from.to_string()), ("to", &to.to_string())]));
                    continue;
                }
                if spans.iter().any(|&(a, b)| from <= b && a <= to) {
                    sess.tool_result(primary.call_id.as_str(), prompts.message("optimizer", "fb_merge_overlap"));
                    continue;
                }
                let dir_cn = match direction.as_str() { "down" => "下", "left" => "左", "right" => "右", _ => "上" };
                let mline = format!("滚动查找 [\"{}\", {}]", target, dir_cn);
                ctx.ui.emit(UiEvent::SubAgent { kind: SubAgent::Optimizer, level: Level::Info, text: format!("⟫ 合并第 {}–{} 步 → {}：{}", from, to, mline, why), tokens: Tokens::new(pt, ct) });
                merges.push(Merge { from, to, line: mline });
                spans.push((from, to));
                sess.tool_result(primary.call_id.as_str(), render(&prompts.message("optimizer", "fb_merge_done"), &[("from", &from.to_string()), ("to", &to.to_string())]));
            }
            _ => {
                sess.tool_result(primary.call_id.as_str(), prompts.message("optimizer", "fb_bad_action"));
            }
        }
    }

    if removable.is_empty() && merges.is_empty() {
        ctx.ui.emit(UiEvent::Notice { level: Level::Dim, text: "脚本优化官：当前路径已无可删冗余/可合并的盲滑".into() });
        return None;
    }

    // 按原行号一次性重建：merge 段输出滚动查找、跳过其余；removable 跳过；其它保留。
    let mut new: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let step = i + 1;
        if let Some(m) = merges.iter().find(|m| m.from == step) {
            new.push(m.line.clone());
            i = m.to;
            continue;
        }
        if !removable.contains(&step) {
            new.push(lines[i].clone());
        }
        i += 1;
    }
    if new == lines {
        return None;
    }

    // 一次性验证：重建后还到不到目标？到不了就放弃本次优化（上层用医生确认的正确版本兜底）。
    let verify = super::doctor::diagnose(tx, ctx, params, script_path, case, &new, marker, "reflect_verify", 0, false).await;
    if !verify.reached {
        ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "↩ 优化后跑不到目标，放弃本次优化（保留医生的正确版本）".into() });
        return None;
    }
    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("◇ 优化完成  {} 步 → {} 步", lines.len(), new.len()) });
    Some(new)
}

/// 定稿语义命名：对终稿引用的「特征自动名」元素，让 AI 给稳定的**语义名**（名词，便于复用/人读），
/// 改临时库 key(特征名→语义名，desc/通道/img 字段不变) + 改脚本里的元素引用。
/// 返回 (改名后的脚本, 语义名列表[供 commit 到正式库])。AI 没给名的元素保留特征名。
pub(super) async fn finalize_names(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    ui: &dyn Frontend,
    tmp_lib: &Path,
    lines: &[String],
    feat_names: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;
    if feat_names.is_empty() {
        return (lines.to_vec(), Vec::new());
    }
    // 失败不静默：保留特征名不影响回放，但要让用户知道这批元素没拿到语义名
    let name_warn = |why: &str| {
        ui.emit(UiEvent::Notice {
            level: Level::Warn,
            text: format!("语义命名失败（{}），元素保留特征名", why),
        });
    };
    let lib: serde_json::Value = std::fs::read_to_string(tmp_lib)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "elements": {} }));
    // 给 AI 列出每个特征名 + 可读特征(ocr 文字 / desc)，帮它起语义名
    let mut info = String::new();
    for f in feat_names {
        let e = &lib["elements"][f];
        let ocr = e["ocr"].as_str().unwrap_or("");
        let desc = e["desc"].as_str().unwrap_or("");
        info.push_str(&format!("- {}（文字/特征：{}；desc：{}）\n", f, ocr, desc));
    }
    let keep = || (lines.to_vec(), feat_names.to_vec());
    let prompt = render(&prompts.message("reflector", "finalize"), &[("elements", &info)]);
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    let mut sess = match LlmSession::new(ai, prompts.role_system("reflector", "", ""), Vec::new()) {
        Ok(s) => s,
        Err(e) => {
            name_warn(&brief(&e.to_string(), 60));
            return keep();
        }
    };
    sess.user(prompt);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        Ok(_) => {
            name_warn("模型没给文本回复");
            return keep();
        }
        Err(e) => {
            name_warn(&brief(&e.to_string(), 60));
            return keep();
        }
    };
    let (pt, ct) = sess.total_usage();
    tx.log("reflector_finalize", serde_json::json!({ "content": reply.clone(), "prompt_tokens": pt, "completion_tokens": ct }));
    let obj = match parse_desc_json(&reply) {
        Some(o) => o,
        None => {
            name_warn("回复不是 JSON");
            return keep();
        }
    };
    // 建 特征名→语义名 映射：去掉 .tks 特殊字符 + 去重(冲突加后缀)；AI 漏给的保留特征名
    let mut map: Vec<(String, String)> = Vec::new();
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for f in feat_names {
        let sem = obj
            .get(f)
            .and_then(|v| v.as_str().or_else(|| v["name"].as_str()))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(f.as_str());
        let mut name: String = sem.chars().filter(|c| !matches!(c, '{' | '}' | '[' | ']' | ',')).collect();
        name = name.trim().to_string();
        if name.is_empty() {
            name = f.clone();
        }
        let mut final_name = name.clone();
        let mut k = 2;
        while used.contains(&final_name) {
            final_name = format!("{}_{}", name, k);
            k += 1;
        }
        used.insert(final_name.clone());
        map.push((f.clone(), final_name));
    }
    // 改临时库 key（条目内容含 img 字段不变；回放按 key 找元素、读其 img 字段，故 img 文件名无需改）
    let mut lib2 = lib.clone();
    if let Some(els) = lib2["elements"].as_object_mut() {
        for (f, sem) in &map {
            if f == sem {
                continue;
            }
            if let Some(entry) = els.remove(f) {
                els.insert(sem.clone(), entry);
            }
        }
    }
    let _ = std::fs::write(tmp_lib, serde_json::to_string_pretty(&lib2).unwrap_or_default());
    // 改脚本里的元素引用 [{特征名}] → [{语义名}]
    let mut new_lines = lines.to_vec();
    for (f, sem) in &map {
        if f == sem {
            continue;
        }
        let (from, to) = (format!("{{{}}}", f), format!("{{{}}}", sem));
        for l in new_lines.iter_mut() {
            if l.contains(&from) {
                *l = l.replace(&from, &to);
            }
        }
    }
    let semantic: Vec<String> = map.iter().map(|(_, s)| s.clone()).collect();
    (new_lines, semantic)
}
