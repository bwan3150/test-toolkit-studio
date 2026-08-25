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

use crate::{AiConfig, Frontend, Level, LlmReply, LlmSession, LlmTool, Result, UiCommand, UiEvent};
use super::analyst;
use super::evidence::{EvidenceDir, EvidenceRef};
use super::finding::{Category, Finding, ProbeReport, Severity};
use super::http::{HttpEngine, HttpRequest, UreqEngine};
use super::prompt::{render, SecurityPrompts};
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
                "verb": {"type": "string", "enum": ["headers","fingerprint","cors","graphql","bundle","endpoints","tls"]},
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
            "type": "object", "properties": {"question": {"type": "string"}}, "required": ["question"]
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
#[allow(clippy::too_many_arguments)]
pub async fn run(
    ai: &AiConfig,
    prompts: &SecurityPrompts,
    frontend: Box<dyn Frontend>,
    task_dir: PathBuf,
    opening_url: Option<String>,
    mode: String,
    focus: String,
    _max_steps: usize,
) -> Result<()> {
    let engine = UreqEngine::default();
    let mut evidence = EvidenceDir::new(&task_dir)?;
    let mut findings: Vec<Finding> = Vec::new();
    let mut evidence_by_seq: Vec<EvidenceRef> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();

    frontend.emit(UiEvent::SessionInfo {
        device: format!("目标 {}", opening_url.clone().unwrap_or_else(|| "（待定，稍后问你）".into())),
        platform: format!("强度 {mode} · 聚焦 {focus}"),
        model: ai.model.clone().unwrap_or_else(|| "默认".into()),
        provider: ai.provider.clone().unwrap_or_else(|| "anthropic（默认）".into()),
        reasoning: ai.reasoning_effort.clone().unwrap_or_else(|| "medium".into()),
    });

    let system = render(&prompts.system("orchestrator"),
        &[("mode", &mode), ("focus", &focus)]);
    let mut session = match LlmSession::new_for_role(ai, "orchestrator", system, tools(prompts)) {
        Ok(s) => s,
        Err(e) => { frontend.emit(UiEvent::Notice { level: Level::Err, text: format!("AI 会话建立失败：{e}") }); frontend.shutdown().await; return Err(e); }
    };

    // 开场：给了 url 直接进入；否则先问
    match &opening_url {
        Some(u) => session.user(render(
            "目标：{url}（强度 {mode}，聚焦 {focus}）。先跟我确认范围/方向，再按我说的来。",
            &[("url", u), ("mode", &mode), ("focus", &focus)])),
        None => match frontend.await_answer(0, "要测哪个目标？给我 URL，也可以说测试重点。".into()).await {
            Some(a) => session.user(a),
            None => { frontend.shutdown().await; return Ok(()); }
        },
    }

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

        let calls = match reply {
            LlmReply::Text(t) => {
                // 编排官对用户说话 → 停下等用户下一句（REPL 回合）
                frontend.emit(UiEvent::Notice { level: Level::Info, text: t });
                match frontend.await_answer(round, String::new()).await {
                    Some(a) => { session.user(a); continue; }
                    None => { aborted = true; break; }
                }
            }
            LlmReply::ToolCalls { text, calls } => {
                if let Some(t) = text.as_deref() {
                    if !t.trim().is_empty() {
                        frontend.emit(UiEvent::Notice { level: Level::Info, text: t.trim().to_string() });
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
                        "headers" => recon::headers_check(&engine, &url), "fingerprint" => recon::fingerprint_check(&engine, &url),
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
                    match frontend.await_answer(round, q).await {
                        Some(a) => session.tool_result(&call.call_id, a),
                        None => { aborted = true; break; }
                    }
                }
                "report" => {
                    frontend.emit(UiEvent::Notice { level: Level::Info, text: format!("出报告：复核 {} 条候选……", findings.len()) });
                    let probe = ProbeReport { target: opening_url.clone().unwrap_or_default(), mode: mode.clone(),
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
