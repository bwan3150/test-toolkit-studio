// 【探索反思官】两类职责：
//  ① 探索失败 → `reflect()` 出「重探指导」，上层带指导重探（治"绕大圈没找到"，探索阶段安全网）；
//  ② 脚本已正确 → `optimize()` 做**软优化**：看正确脚本的 trace，指出可删的绕路/冗余步并删掉
//     （大胆删、不逐条验证；删坏了由医生在下一轮复检兜底——医生⇄反思官交替收敛）。
// 独立会话(无工具)，按作用域归属 "reflector"；token 单独计、并入总量。

use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use crate::{AiConfig, Fetcher, LlmReply, LlmSession, Params, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::flow::{brief, fmt_tokens, friendly, paint, parse_desc_json, DriveCtx, DriveOutcome};
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
    let tty = std::io::stderr().is_terminal();
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;

    // 诊断当前脚本拿逐步 trace（调用前应已正确；不正确则不优化）
    let diag = super::doctor::diagnose(tx, ctx, params, script_path, case, lines, marker, "reflect_diagnose", 0, false).await;
    if !diag.reached {
        return None;
    }
    let noops = diag.noop_step_nos();
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
    let mut sess = LlmSession::new(ai, system, Vec::new()).ok()?;
    let intro = render(
        &prompts.message("optimizer", "intro"),
        &[("case", case), ("marker", marker), ("trace", &trace), ("pages", &pages), ("noops", &noop_list), ("script", &numbered)],
    );
    tx.log("llm_message", serde_json::json!({ "content": intro.clone() }));
    sess.user(intro);

    // 逐步收集优化（像医生：LLM 每次只提一个动作，执行/拒绝后反馈，它据此再提下一个）。
    // 在**稳定的原始行号**上工作：删除收进 removable、合并收进 merges，最后一次性重建 + 一次诊断验证，
    // 避免每改一步就重跑回放。护栏：只准删空操作步（noops），承重步当场拒。
    struct Merge { from: usize, to: usize, line: String }
    let mut removable: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut merges: Vec<Merge> = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for _ in 0..16usize {
        if super::interrupt::aborted() {
            break;
        }
        let reply = match sess.next().await {
            Ok(LlmReply::Text(t)) => t,
            _ => break,
        };
        let (pt, ct) = sess.last_usage();
        report.extra_prompt += pt;
        report.extra_completion += ct;
        let toks = paint(tty, "2", &format!("↑{} ↓{}", fmt_tokens(pt), fmt_tokens(ct)));
        let obj = match parse_desc_json(&reply) {
            Some(o) => o,
            None => {
                sess.user("请只返回一个 JSON 动作：{\"action\":\"delete|merge|done\", ...}".to_string());
                continue;
            }
        };
        let action = obj["action"].as_str().unwrap_or("").trim();
        let raw_reason = obj["reason"].as_str().unwrap_or("").trim();
        let why = if raw_reason.is_empty() { "（未说明理由）".to_string() } else { brief(raw_reason, 160) };
        match action {
            "done" => {
                eprintln!("  {}  {}  {}", paint(tty, "1;36", "✓ 优化结束"), paint(tty, "2", &why), toks);
                break;
            }
            "delete" => {
                let step = obj["step"].as_u64().unwrap_or(0) as usize;
                if step < 1 || step > lines.len() {
                    sess.user(format!("第 {} 步不存在（共 {} 步），换一步或 done。", step, lines.len()));
                    continue;
                }
                if removable.contains(&step) {
                    sess.user(format!("第 {} 步已删过，提下一个或 done。", step));
                    continue;
                }
                if !noops.contains(&step) {
                    eprintln!("  {}  {}", paint(tty, "33", &format!("↩ 第 {} 步改变了页面（承重步），不删", step)), toks);
                    sess.user(format!("第 {} 步执行后页面变了（承重步），删了会让后续失效、不能删。换一步或 done。", step));
                    continue;
                }
                removable.insert(step);
                eprintln!("  {}  {}  {}", paint(tty, "1;35", &format!("⟫ 删第 {} 步", step)), paint(tty, "2", &format!("{}（{}）", why, friendly(&lines[step - 1]))), toks);
                sess.user(format!("已删第 {} 步（空操作、安全）。继续提下一个，或 done。", step));
            }
            "merge" => {
                let from = obj["from"].as_u64().unwrap_or(0) as usize;
                let to = obj["to"].as_u64().unwrap_or(0) as usize;
                let target = obj["target"].as_str().unwrap_or("").trim();
                let direction = obj["direction"].as_str().unwrap_or("up").trim();
                if from < 1 || to < from || to > lines.len() || target.is_empty() {
                    sess.user("merge 参数无效：需 1<=from<=to<=步数 且 target 非空。".to_string());
                    continue;
                }
                if !lines[from - 1..to].iter().all(|l| l.trim_start().starts_with("定向滑动")) {
                    sess.user(format!("第 {}–{} 步不全是「定向滑动」，不能合并。", from, to));
                    continue;
                }
                if spans.iter().any(|&(a, b)| from <= b && a <= to) {
                    sess.user("这段与已合并的段重叠，换一段或 done。".to_string());
                    continue;
                }
                let dir_cn = match direction { "down" => "下", "left" => "左", "right" => "右", _ => "上" };
                let mline = format!("滚动查找 [\"{}\", {}]", target, dir_cn);
                eprintln!("  {}  {}  {}", paint(tty, "1;35", &format!("⟫ 合并第 {}–{} 步 → {}", from, to, mline)), paint(tty, "2", &why), toks);
                merges.push(Merge { from, to, line: mline });
                spans.push((from, to));
                sess.user(format!("已合并第 {}–{} 步。继续，或 done。", from, to));
            }
            _ => {
                sess.user("action 只能是 delete / merge / done。".to_string());
            }
        }
    }

    if removable.is_empty() && merges.is_empty() {
        eprintln!("  {}", paint(tty, "2", "脚本优化官：当前路径已无可删冗余/可合并的盲滑"));
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
        eprintln!("  {}", paint(tty, "33", "↩ 优化后跑不到目标，放弃本次优化（保留医生的正确版本）"));
        return None;
    }
    eprintln!("  {}  {} 步 → {} 步", paint(tty, "1;36", "◇ 优化完成"), lines.len(), new.len());
    Some(new)
}

/// 定稿语义命名：对终稿引用的「特征自动名」元素，让 AI 给稳定的**语义名**（名词，便于复用/人读），
/// 改临时库 key(特征名→语义名，desc/通道/img 字段不变) + 改脚本里的元素引用。
/// 返回 (改名后的脚本, 语义名列表[供 commit 到正式库])。AI 没给名的元素保留特征名。
pub(super) async fn finalize_names(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    tmp_lib: &Path,
    lines: &[String],
    feat_names: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;
    if feat_names.is_empty() {
        return (lines.to_vec(), Vec::new());
    }
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
        Err(_) => return keep(),
    };
    sess.user(prompt);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return keep(),
    };
    let (pt, ct) = sess.total_usage();
    tx.log("reflector_finalize", serde_json::json!({ "content": reply.clone(), "prompt_tokens": pt, "completion_tokens": ct }));
    let obj = match parse_desc_json(&reply) {
        Some(o) => o,
        None => return keep(),
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
