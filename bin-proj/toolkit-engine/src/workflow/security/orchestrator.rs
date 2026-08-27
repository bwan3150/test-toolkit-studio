//! orchestrator —— 安全测试的**对话外壳**（ADR-0019：`tke security` 默认形态）。
//!
//! 形态同 harness 的编排官（ADR-0002）：一个与用户对话的 REPL，复用 harness 的 `Frontend`
//! 三前端（Plain/Json/**TUI**）——「共享 TUI」就是直接用 `TuiFrontend`。
//! 它按用户的方向调度探测（recon/http），有风险的事先 `ask_user` 问，随时可 `report` 出报告。
//!
//! 与自主 prober 的区别：prober 无人值守一路跑到底；orchestrator **把方向盘交给用户**，
//! 每当它对用户说话（纯文本回复），就停下等用户的下一句（REPL 回合）。
//!
//! 只借 provider（LlmSession）与 ui（Frontend）；提示词走 security 自己的 SecurityPrompts。

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::workflow::agent::ui::Tokens;
use crate::{AiConfig, Frontend, Level, LlmReply, LlmSession, LlmTool, Result, UiCommand, UiEvent};
use super::analyst;
use super::evidence::{EvidenceDir, EvidenceRef};
use super::finding::{Category, Finding, ProbeReport, Severity};
use super::http::{HttpEngine, HttpRequest, UreqEngine};
use super::prompt::SecurityPrompts;
use super::recon;
use super::report as reporter;

const BODY_EXCERPT: usize = 2000;

/// 编排官工具集（description 从 SecurityPrompts 取，可外部覆盖）。
fn tools(prompts: &SecurityPrompts) -> Vec<LlmTool> {
    let d = |name: &str| prompts.tool("orchestrator", name);
    vec![
        LlmTool::new("http", d("http"), json!({
            "type": "object",
            "properties": {
                "method": {"type": "string"}, "url": {"type": "string"},
                "headers": {"type": "array", "items": {"type": "object",
                    "properties": {"name": {"type": "string"}, "value": {"type": "string"}}, "required": ["name","value"]}},
                "body": {"type": "string"}
            }, "required": ["method","url"]
        })),
        LlmTool::new("recon", d("recon"), json!({
            "type": "object",
            "properties": {
                "verb": {"type": "string", "enum": ["headers","fingerprint","detect","cors","graphql","bundle","endpoints","tls"]},
                "url": {"type": "string"}
            }, "required": ["verb","url"]
        })),
        LlmTool::new("record_finding", d("record_finding"), json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"}, "severity": {"type": "string", "enum": ["critical","high","medium","low","info"]},
                "category": {"type": "string", "enum": ["auth","data-exposure","injection","transport","config"]},
                "title": {"type": "string"}, "detail": {"type": "string"},
                "confirmed": {"type": "boolean"}, "repro": {"type": "string"},
                "evidence_steps": {"type": "array", "items": {"type": "integer"}}
            }, "required": ["id","severity","category","title","detail","confirmed"]
        })),
        LlmTool::new("ask_user", d("ask_user"), json!({
            "type": "object",
            "properties": {
                "question": {"type": "string"},
                "options": {"type": "array", "items": {"type": "string"},
                    "description": "可选：给用户几个可选项（TUI 渲染成列表，用户可选或直接打字）"}
            },
            "required": ["question"]
        })),
        LlmTool::new("set_scope", d("set_scope"), json!({
            "type": "object",
            "properties": {
                "target": {"type": "string"},
                "mode": {"type": "string", "enum": ["passive","safe","aggressive","red-team"]},
                "focus": {"type": "string"}
            }
        })),
        LlmTool::new("report", d("report"), json!({
            "type": "object", "properties": {}
        })),
        LlmTool::new("finish", d("finish"), json!({
            "type": "object", "properties": {"summary": {"type": "string"}}, "required": ["summary"]
        })),
    ]
}

fn parse_severity(s: &str) -> Severity {
    match s.trim().to_ascii_lowercase().as_str() {
        "critical" => Severity::Critical, "high" => Severity::High, "medium" => Severity::Medium,
        "low" => Severity::Low, _ => Severity::Info,
    }
}
fn headers_from(v: &Value) -> Vec<(String, String)> {
    v.get("headers").and_then(|h| h.as_array()).map(|arr| arr.iter().filter_map(|h| {
        Some((h.get("name")?.as_str()?.to_string(), h.get("value")?.as_str()?.to_string()))
    }).collect()).unwrap_or_default()
}

/// 运行对话式编排。阻塞直到 finish / 用户 Abort。
/// url/mode/focus 都可能为 None——由主 agent 在 TUI 里**开场面试**补齐（选项/追问）。
#[allow(clippy::too_many_arguments)]
pub async fn run(
    ai: &AiConfig,
    prompts: &SecurityPrompts,
    frontend: Box<dyn Frontend>,
    task_dir: PathBuf,
    opening_url: Option<String>,
    mode_opt: Option<String>,
    focus_opt: Option<String>,
    _max_steps: usize,
) -> Result<()> {
    let engine = UreqEngine::default();
    let mut evidence = EvidenceDir::new(&task_dir)?;
    let mut findings: Vec<Finding> = Vec::new();
    let mut evidence_by_seq: Vec<EvidenceRef> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();

    // 运行态：由 CLI 参数初始化，可被主 agent 的 set_scope 覆盖（面试的结果真正生效）
    let mut target: Option<String> = opening_url.clone();
    let mut mode: String = mode_opt.clone().unwrap_or_else(|| "safe".into());
    let mut focus: String = focus_opt.clone().unwrap_or_else(|| "全量".into());

    frontend.emit(UiEvent::SessionInfo {
        device: format!("目标 {}", target.clone().unwrap_or_else(|| "（待 agent 问你）".into())),
        platform: format!("强度 {mode} · 聚焦 {focus}"),
        model: ai.model.clone().unwrap_or_else(|| "默认".into()),
        provider: ai.provider.clone().unwrap_or_else(|| "anthropic（默认）".into()),
        // 同 harness:说实际会发生什么(模型不支持思考时我们不发那个参数)
        reasoning: match ai.model.as_deref() {
            Some(m) if !crate::model_supports_reasoning(m) => format!("关闭（{m} 不支持）"),
            _ => ai.reasoning_effort.clone().unwrap_or_else(|| "medium".into()),
        },
    });

    let system = prompts.system("orchestrator");
    let mut session = match LlmSession::new_for_role(ai, "orchestrator", system, tools(prompts)) {
        Ok(s) => s,
        Err(e) => { frontend.emit(UiEvent::Notice { level: Level::Err, text: format!("AI 会话建立失败：{e}") }); frontend.shutdown().await; return Err(e); }
    };

    // 开场：把「已知/未知」交给 agent，让它做开场面试（缺什么问什么，强度/scope 用选项问）
    let known = format!(
        "会话开始。已知：目标={}，强度档={}，聚焦={}。\
         缺的信息（目标/强度/scope）请**用 ask_user 问我**补齐——强度和 scope 尽量给选项让我选；\
         问清后调 set_scope 记下，再开始测。",
        target.clone().unwrap_or_else(|| "未给".into()),
        mode_opt.clone().unwrap_or_else(|| "未给".into()),
        focus_opt.clone().unwrap_or_else(|| "未给".into()),
    );
    session.user(known);

    let mut round = 0usize;
    let mut aborted = false;
    'outer: loop {
        round += 1;
        // 安全点：取用户随时插入的指导 / 中断
        for cmd in frontend.drain_commands() {
            match cmd {
                UiCommand::Abort => { aborted = true; break 'outer; }
                UiCommand::Guidance { text } => { frontend.emit(UiEvent::GuidanceAccepted { text: text.clone() }); session.user(format!("[用户插话] {text}")); }
                _ => {}
            }
        }

        let reply = match session.next().await {
            Ok(r) => r,
            Err(e) => { frontend.emit(UiEvent::Notice { level: Level::Err, text: format!("AI 出错：{e}") }); break; }
        };
        let (pt, ct) = session.last_usage();

        let calls = match reply {
            LlmReply::Text(t) => {
                // 编排官对用户说话 → 用 Assistant 事件（多行完整渲染，不走 Notice 的缩进包裹）→ 停下等用户
                frontend.emit(UiEvent::Assistant { text: t, tokens: Tokens::new(pt, ct) });
                match frontend.await_answer(round, String::new()).await {
                    Some(a) => { session.user(a); continue; }
                    None => { aborted = true; break; }
                }
            }
            LlmReply::ToolCalls { text, calls } => {
                if let Some(t) = text.as_deref() {
                    if !t.trim().is_empty() {
                        // 调工具时的思考/说明也是对用户说的话 → Assistant，多行正确渲染
                        frontend.emit(UiEvent::Assistant { text: t.trim().to_string(), tokens: Tokens::new(pt, ct) });
                    }
                }
                calls
            }
        };

        for call in calls {
            match call.name.as_str() {
                "http" => {
                    let method = call.arguments.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_uppercase();
                    let url = call.arguments.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if url.is_empty() { session.tool_result(&call.call_id, "错误：缺少 url"); continue; }
                    let key = format!("{method} {url}");
                    if let Some(&p) = seen.get(&key) {
                        session.tool_result(&call.call_id, format!("已在 step {p} 取过，别重复。")); continue;
                    }
                    frontend.emit(UiEvent::Notice { level: Level::Dim, text: format!("→ http {method} {url}") });
                    let mut req = HttpRequest::new(&method, &url);
                    req.headers = headers_from(&call.arguments);
                    if let Some(b) = call.arguments.get("body").and_then(|v| v.as_str()) { req = req.body(b.as_bytes().to_vec()); }
                    match engine.send(&req) {
                        Ok(resp) => {
                            let er = evidence.record(&req, &resp)?; let seq = er.seq;
                            evidence_by_seq.push(er); seen.insert(key, seq);
                            let excerpt: String = resp.text().chars().take(BODY_EXCERPT).collect();
                            let brief = format!("step {seq} · HTTP {} · {}B", resp.status, resp.body.len());
                            session.tool_result_bulky(&call.call_id,
                                format!("{brief}\n响应头:\n{}\n\n体(前{BODY_EXCERPT}字):\n{excerpt}",
                                    resp.headers.iter().map(|(k,v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n")),
                                brief);
                        }
                        Err(e) => session.tool_result(&call.call_id, format!("请求失败：{e}")),
                    }
                }
                "recon" => {
                    let verb = call.arguments.get("verb").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let url = call.arguments.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let key = format!("recon:{verb} {url}");
                    if let Some(&p) = seen.get(&key) { session.tool_result(&call.call_id, format!("已在 step {p} 附近跑过，别重复。")); continue; }
                    frontend.emit(UiEvent::Notice { level: Level::Dim, text: format!("→ recon {verb} {url}") });
                    let res = match verb.as_str() {
                        "headers" => recon::headers_check(&engine, &url), "fingerprint" => recon::fingerprint_check(&engine, &url), "detect" => recon::detect_check(&engine, &url),
                        "cors" => recon::cors_check(&engine, &url), "graphql" => recon::graphql_check(&engine, &url),
                        "bundle" => recon::bundle_check(&engine, &url), "endpoints" => recon::endpoints_check(&engine, &url),
                        "tls" => recon::tls_check(&engine, &url), o => { session.tool_result(&call.call_id, format!("未知 verb：{o}")); continue; }
                    };
                    match res {
                        Ok(r) => {
                            let mut steps = Vec::new();
                            for p in &r.probes { let er = evidence.record(&p.request, &p.response)?; steps.push(er.seq); evidence_by_seq.push(er); }
                            seen.insert(key, *steps.last().unwrap_or(&0));
                            session.tool_result(&call.call_id, json!({"verb": verb, "url": url, "evidence_steps": steps, "findings": r.findings}).to_string());
                        }
                        Err(e) => session.tool_result(&call.call_id, format!("recon 失败：{e}")),
                    }
                }
                "record_finding" => {
                    let a = &call.arguments;
                    let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("finding").to_string();
                    let mut f = Finding::new(&id,
                        parse_severity(a.get("severity").and_then(|v| v.as_str()).unwrap_or("info")),
                        Category::parse(a.get("category").and_then(|v| v.as_str()).unwrap_or("info")),
                        a.get("title").and_then(|v| v.as_str()).unwrap_or("(无标题)"),
                        a.get("detail").and_then(|v| v.as_str()).unwrap_or(""));
                    f.confirmed = a.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
                    f.repro = a.get("repro").and_then(|v| v.as_str()).map(String::from);
                    if let Some(steps) = a.get("evidence_steps").and_then(|v| v.as_array()) {
                        for s in steps { if let Some(n) = s.as_u64() { if let Some(er) = evidence_by_seq.iter().find(|e| e.seq == n as usize) { f.evidence.push(er.clone()); } } }
                    }
                    frontend.emit(UiEvent::Notice { level: if f.confirmed { Level::Warn } else { Level::Dim },
                        text: format!("记下发现[{}] {}", if f.confirmed { "确认" } else { "疑似" }, f.title) });
                    findings.push(f);
                    session.tool_result(&call.call_id, format!("已记录：{id}"));
                }
                "ask_user" => {
                    let q = call.arguments.get("question").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let options: Vec<String> = call.arguments.get("options").and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    let ans = if options.is_empty() {
                        frontend.await_answer(round, q).await
                    } else {
                        use crate::workflow::agent::ui::ChoiceReply;
                        match frontend.await_choice_or_text(q, options.clone()).await {
                            Some(ChoiceReply::Pick(i)) => options.get(i).cloned(),
                            Some(ChoiceReply::Text(t)) => Some(t),
                            None => None,
                        }
                    };
                    match ans {
                        Some(a) => session.tool_result(&call.call_id, a),
                        None => { aborted = true; break; }
                    }
                }
                "set_scope" => {
                    let a = &call.arguments;
                    if let Some(t) = a.get("target").and_then(|v| v.as_str()) { if !t.trim().is_empty() { target = Some(t.to_string()); } }
                    if let Some(m) = a.get("mode").and_then(|v| v.as_str()) { if !m.trim().is_empty() { mode = m.to_string(); } }
                    if let Some(f) = a.get("focus").and_then(|v| v.as_str()) { if !f.trim().is_empty() { focus = f.to_string(); } }
                    frontend.emit(UiEvent::Notice { level: Level::Dim,
                        text: format!("范围已定：目标 {} · 强度 {mode} · 聚焦 {focus}", target.clone().unwrap_or_else(|| "未定".into())) });
                    session.tool_result(&call.call_id, format!("scope set: target={} mode={mode} focus={focus}", target.clone().unwrap_or_default()));
                }
                "report" => {
                    frontend.emit(UiEvent::Notice { level: Level::Info, text: format!("出报告：复核 {} 条候选……", findings.len()) });
                    // 对话式这条路上，编排官自己那段会话也在烧 token——
                    // 不带上它，交互式跑出来的账就只有复核那一半
                    let mut spent = super::usage::Usage::default();
                    let (p, c) = session.total_usage();
                    spent.add("orchestrator", session.model(), p, c);
                    let probe = ProbeReport { usage: spent, target: target.clone().unwrap_or_default(), mode: mode.clone(),
                        findings: findings.clone(), summary: String::new(), steps: round };
                    let analyzed = analyst::analyze(ai, prompts, &task_dir, probe).await?;
                    let paths = reporter::write_reports(&task_dir, &analyzed)?;
                    frontend.emit(UiEvent::Notice { level: Level::Ok, text: format!("报告已生成：{}", paths.html.display()) });
                    session.tool_result(&call.call_id, json!({
                        "report_html": paths.html.to_string_lossy(),
                        "findings_json": paths.json.to_string_lossy(),
                        "vuln_reports": paths.vulns.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
                        "summary": analyzed.summary,
                    }).to_string());
                    if analyzed.usage.is_measured() {
                        frontend.emit(UiEvent::Notice { level: Level::Dim,
                            text: format!("本次用量：{} tokens（{} 提示 / {} 生成）",
                                analyzed.usage.total_tokens, analyzed.usage.prompt_tokens, analyzed.usage.completion_tokens) });
                    }
                }
                "finish" => {
                    let s = call.arguments.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                    frontend.emit(UiEvent::Notice { level: Level::Ok, text: format!("结束：{s}") });
                    break 'outer;
                }
                other => session.tool_result(&call.call_id, format!("未知工具：{other}")),
            }
        }
        if aborted { break; }
    }

    frontend.emit(UiEvent::Notice { level: Level::Dim, text: format!("证据/报告目录：{}", task_dir.display()) });
    frontend.shutdown().await;
    if aborted { std::process::exit(130); }
    Ok(())
}
