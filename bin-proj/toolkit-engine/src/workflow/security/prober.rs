//! prober —— 黑盒安全**探测官**：多轮、接地、顺藤摸瓜。
//!
//! 形态学上像 harness 的 orchestrator/explorer：一个带工具的 LLM 循环，每轮基于**刚看到的真实响应**
//! 决定下一步（接地，INV-1）。工具：http / recon / record_finding / finish。
//! 每个探测都过 evidence（INV-14）；findings 由 prober 显式 `record_finding` 才进候选（recon 结果只是线索）。
//!
//! 与 harness 解耦：只借 provider（LlmSession）这个领域无关件；提示词走 security 自己的 SecurityPrompts。
//! run() 收 LlmSession（依赖注入）——真实路径 CLI 用 new_for_role 建，测试用 new_fake 脚本化建。

use serde_json::{json, Value};

use crate::{LlmReply, LlmSession, LlmTool, Result};
use super::evidence::{EvidenceDir, EvidenceRef};
use super::finding::{Category, Finding, ProbeReport, Severity};
use super::http::{HttpEngine, HttpRequest};
use super::prompt::{render, SecurityPrompts};
use super::recon;

/// 本次探测的上下文。
pub struct ProbeCtx {
    pub target: String,
    pub mode: String,
    pub focus: String,
}

/// 响应体摘要给 LLM 的上限（保历史紧凑；完整内容在证据文件里）。
const BODY_EXCERPT: usize = 2000;

/// 组装 prober 系统提示词（含占位替换）。
pub fn system_prompt(prompts: &SecurityPrompts, ctx: &ProbeCtx) -> String {
    render(
        &prompts.system("prober"),
        &[("target", &ctx.target), ("mode", &ctx.mode), ("focus", &ctx.focus)],
    )
}

/// 组装 prober 的四个工具定义（description 从 SecurityPrompts 取，可外部覆盖）。
pub fn tools(prompts: &SecurityPrompts) -> Vec<LlmTool> {
    let d = |name: &str| prompts.tool("prober", name);
    vec![
        LlmTool::new("http", d("http"), json!({
            "type": "object",
            "properties": {
                "method": {"type": "string"},
                "url": {"type": "string"},
                "headers": {"type": "array", "items": {"type": "object",
                    "properties": {"name": {"type": "string"}, "value": {"type": "string"}},
                    "required": ["name", "value"]}},
                "body": {"type": "string"}
            },
            "required": ["method", "url"]
        })),
        LlmTool::new("recon", d("recon"), json!({
            "type": "object",
            "properties": {
                "verb": {"type": "string", "enum": ["headers","fingerprint","cors","graphql","bundle","endpoints","tls"]},
                "url": {"type": "string"}
            },
            "required": ["verb", "url"]
        })),
        LlmTool::new("record_finding", d("record_finding"), json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "severity": {"type": "string", "enum": ["critical","high","medium","low","info"]},
                "category": {"type": "string", "enum": ["auth","data-exposure","injection","transport","config"]},
                "title": {"type": "string"},
                "detail": {"type": "string"},
                "confirmed": {"type": "boolean"},
                "repro": {"type": "string"},
                "evidence_steps": {"type": "array", "items": {"type": "integer"}}
            },
            "required": ["id","severity","category","title","detail","confirmed"]
        })),
        LlmTool::new("finish", d("finish"), json!({
            "type": "object",
            "properties": {"summary": {"type": "string"}},
            "required": ["summary"]
        })),
    ]
}

fn parse_severity(s: &str) -> Severity {
    match s.trim().to_ascii_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        _ => Severity::Info,
    }
}

fn headers_from(v: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(arr) = v.get("headers").and_then(|h| h.as_array()) {
        for h in arr {
            if let (Some(n), Some(val)) = (h.get("name").and_then(|x| x.as_str()), h.get("value").and_then(|x| x.as_str())) {
                out.push((n.to_string(), val.to_string()));
            }
        }
    }
    out
}

/// 运行 prober 循环。`session` 已注入 system + tools；`max_steps` 兜底防跑飞。
pub async fn run(
    mut session: LlmSession,
    engine: &dyn HttpEngine,
    evidence: &mut EvidenceDir,
    ctx: &ProbeCtx,
    max_steps: usize,
) -> Result<ProbeReport> {
    let mut report = ProbeReport {
        target: ctx.target.clone(),
        mode: ctx.mode.clone(),
        findings: Vec::new(),
        summary: String::new(),
        steps: 0,
    };
    // seq → EvidenceRef，供 record_finding 按步号回指
    let mut evidence_by_seq: Vec<EvidenceRef> = Vec::new();

    session.user(render(
        "开始对 {target} 的黑盒探测（强度 {mode}，聚焦 {focus}）。先测绘，再顺线索追。",
        &[("target", &ctx.target), ("mode", &ctx.mode), ("focus", &ctx.focus)],
    ));

    let mut idle_texts = 0;
    for step in 0..max_steps {
        report.steps = step + 1;
        let reply = session.next().await?;
        let calls = match reply {
            LlmReply::ToolCalls { text, calls } => {
                if let Some(t) = text.as_deref() {
                    if !t.trim().is_empty() {
                        eprintln!("  · {}", t.trim());
                    }
                }
                calls
            }
            LlmReply::Text(t) => {
                // 没调工具：轻推一次；连续两次空转就收。
                idle_texts += 1;
                eprintln!("  · {}", t.trim());
                if idle_texts >= 2 {
                    report.summary = t.trim().to_string();
                    break;
                }
                session.user("请调用一个工具继续探测，或调用 finish 收尾。");
                continue;
            }
        };
        idle_texts = 0;

        let mut finished = false;
        for call in calls {
            match call.name.as_str() {
                "http" => {
                    let method = call.arguments.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_uppercase();
                    let url = call.arguments.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if url.is_empty() {
                        session.tool_result(&call.call_id, "错误：缺少 url");
                        continue;
                    }
                    eprintln!("▶ http {method} {url}");
                    let mut req = HttpRequest::new(&method, &url);
                    req.headers = headers_from(&call.arguments);
                    if let Some(b) = call.arguments.get("body").and_then(|v| v.as_str()) {
                        req = req.body(b.as_bytes().to_vec());
                    }
                    match engine.send(&req) {
                        Ok(resp) => {
                            let eref = evidence.record(&req, &resp)?;
                            let seq = eref.seq;
                            evidence_by_seq.push(eref);
                            let body = resp.text();
                            let excerpt: String = body.chars().take(BODY_EXCERPT).collect();
                            let summary = format!(
                                "step {seq} · HTTP {} · {}B{} · 证据 evidence/step_{seq:03}_*",
                                resp.status,
                                resp.body.len(),
                                if resp.truncated { "(截断)" } else { "" },
                            );
                            let full = format!(
                                "{summary}\n响应头:\n{}\n\n响应体(前 {BODY_EXCERPT} 字符):\n{excerpt}",
                                resp.headers.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n"),
                            );
                            session.tool_result_bulky(&call.call_id, full, summary);
                        }
                        Err(e) => session.tool_result(&call.call_id, format!("请求失败：{e}")),
                    }
                }
                "recon" => {
                    let verb = call.arguments.get("verb").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let url = call.arguments.get("url").and_then(|v| v.as_str()).unwrap_or(&ctx.target).to_string();
                    eprintln!("▶ recon {verb} {url}");
                    let result = match verb.as_str() {
                        "headers" => recon::headers_check(engine, &url),
                        "fingerprint" => recon::fingerprint_check(engine, &url),
                        "cors" => recon::cors_check(engine, &url),
                        "graphql" => recon::graphql_check(engine, &url),
                        "bundle" => recon::bundle_check(engine, &url),
                        "endpoints" => recon::endpoints_check(engine, &url),
                        "tls" => recon::tls_check(engine, &url),
                        other => {
                            session.tool_result(&call.call_id, format!("未知 verb：{other}"));
                            continue;
                        }
                    };
                    match result {
                        Ok(r) => {
                            let mut steps_note = Vec::new();
                            for p in &r.probes {
                                let eref = evidence.record(&p.request, &p.response)?;
                                steps_note.push(eref.seq);
                                evidence_by_seq.push(eref);
                            }
                            let payload = json!({
                                "verb": verb,
                                "url": url,
                                "evidence_steps": steps_note,
                                "findings": r.findings,
                            });
                            session.tool_result(&call.call_id, payload.to_string());
                        }
                        Err(e) => session.tool_result(&call.call_id, format!("recon {verb} 失败：{e}")),
                    }
                }
                "record_finding" => {
                    let a = &call.arguments;
                    let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("finding").to_string();
                    let mut f = Finding::new(
                        &id,
                        parse_severity(a.get("severity").and_then(|v| v.as_str()).unwrap_or("info")),
                        Category::parse(a.get("category").and_then(|v| v.as_str()).unwrap_or("info")),
                        a.get("title").and_then(|v| v.as_str()).unwrap_or("(无标题)"),
                        a.get("detail").and_then(|v| v.as_str()).unwrap_or(""),
                    );
                    f.confirmed = a.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
                    f.repro = a.get("repro").and_then(|v| v.as_str()).map(|s| s.to_string());
                    if let Some(steps) = a.get("evidence_steps").and_then(|v| v.as_array()) {
                        for s in steps {
                            if let Some(n) = s.as_u64() {
                                if let Some(er) = evidence_by_seq.iter().find(|e| e.seq == n as usize) {
                                    f.evidence.push(er.clone());
                                }
                            }
                        }
                    }
                    eprintln!("  ✔ finding [{}] {} ({})", if f.confirmed { "硬" } else { "疑似" }, f.title, id);
                    report.findings.push(f);
                    session.tool_result(&call.call_id, format!("已记录 finding: {id}"));
                }
                "finish" => {
                    report.summary = call.arguments.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    finished = true;
                    break;
                }
                other => {
                    session.tool_result(&call.call_id, format!("未知工具：{other}"));
                }
            }
        }
        if finished {
            break;
        }
    }

    if report.summary.is_empty() {
        report.summary = format!("达到步数上限（{max_steps}），未显式收尾。", );
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::agent::provider::FakeTurn;
    use super::super::http::FakeEngine;

    #[tokio::test]
    async fn prober_follows_leads_and_records_finding() {
        // 脚本化 LLM：先 recon fingerprint → http 拉 bundle → record_finding → finish
        let turns = vec![
            FakeTurn::tool("recon", json!({"verb": "fingerprint", "url": "https://t.example/"})),
            FakeTurn::tool("http", json!({"method": "GET", "url": "https://t.example/app.js"})),
            FakeTurn::tool("record_finding", json!({
                "id": "leaked-key", "severity": "high", "category": "data-exposure",
                "title": "bundle 泄露 API Key", "detail": "前端 JS 含可用密钥",
                "confirmed": true, "repro": "curl https://t.example/app.js", "evidence_steps": [2]
            })),
            FakeTurn::tool("finish", json!({"summary": "发现 1 个高危密钥泄露"})),
        ];
        let prompts = SecurityPrompts::load(None);
        let ctx = ProbeCtx { target: "https://t.example/".into(), mode: "safe".into(), focus: "全量".into() };
        let session = LlmSession::new_fake(system_prompt(&prompts, &ctx), tools(&prompts), turns);

        let engine = FakeEngine::new()
            .route("/app.js", 200, &[], "const k='AIzaSyA1234567890abcdefghijklmnopqrstuvw'")
            .route("/", 200, &[("Server", "Framer/1")], "<html></html>");

        let tmp = std::env::temp_dir().join(format!("tke-prober-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut evi = EvidenceDir::new(&tmp).unwrap();

        let report = run(session, &engine, &mut evi, &ctx, 20).await.unwrap();

        assert_eq!(report.findings.len(), 1);
        let f = &report.findings[0];
        assert_eq!(f.id, "leaked-key");
        assert!(f.confirmed);
        assert!(!f.evidence.is_empty(), "finding 应关联到 http 那步的证据");
        assert!(report.summary.contains("密钥"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
