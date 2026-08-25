//! analyst —— 对抗式复核官：把 prober 的候选 findings 逐条过闸，压假阳、定软硬。
//!
//! 形态是**单次结构化输出**（INV-2：一个 report 工具 + 强制 tool_choice），不是多轮循环。
//! 判据必须来自真实证据（INV-13）：把每条 finding 关联的 req/resp 原文喂给它，让它据证据下判断，
//! 而不是据 prober 的一面之词。默认怀疑：拿不出复现就 keep=false 或 confirmed=false。
//!
//! 与 agent 的 runner::oneshot 解耦——这里内联一个精简版强制工具调用（只借 provider）。

use std::path::Path;

use serde_json::json;

use crate::{AiConfig, LlmReply, LlmSession, LlmTool, Result};
use super::finding::{Category, Finding, ProbeReport, Severity};
use super::prompt::{render, SecurityPrompts};

/// 复核后的报告（进 reporter）。
#[derive(Debug, Clone)]
pub struct AnalyzedReport {
    pub target: String,
    pub mode: String,
    pub findings: Vec<Finding>,
    pub summary: String,
    /// 被复核毙掉的候选数（假阳/无法复现）。
    pub dropped: usize,
    /// 全程用量（prober 的 + 复核的），平台按它计费（ADR-0023 D3）
    pub usage: super::usage::Usage,
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

/// 读取一条 finding 关联的证据原文（截断防过长），拼给 analyst 做判断依据。
fn evidence_text(task_dir: &Path, f: &Finding) -> String {
    let mut out = String::new();
    for er in &f.evidence {
        for (label, rel) in [("请求", &er.request), ("响应", &er.response)] {
            if let Ok(s) = std::fs::read_to_string(task_dir.join(rel)) {
                let excerpt: String = s.chars().take(3000).collect();
                out.push_str(&format!("--- step {} {label} ---\n{excerpt}\n\n", er.seq));
            }
        }
    }
    if out.is_empty() {
        out.push_str("（该 finding 未关联任何证据文件——这本身是「无法复现」的强信号）");
    }
    out
}

/// 复核 report：逐条 finding 过闸。0 findings 时不发起任何 LLM 调用。
pub async fn analyze(
    ai: &AiConfig,
    prompts: &SecurityPrompts,
    task_dir: &Path,
    report: ProbeReport,
) -> Result<AnalyzedReport> {
    let system = render(
        &prompts.system("analyst"),
        &[("target", &report.target), ("mode", &report.mode)],
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "keep": {"type": "boolean"},
            "confirmed": {"type": "boolean"},
            "severity": {"type": "string", "enum": ["critical","high","medium","low","info"]},
            "category": {"type": "string", "enum": ["auth","data-exposure","injection","transport","config"]},
            "title": {"type": "string"},
            "detail": {"type": "string"},
            "rationale": {"type": "string"}
        },
        "required": ["keep", "confirmed", "rationale"]
    });

    let mut kept: Vec<Finding> = Vec::new();
    let mut dropped = 0usize;
    // 从 prober 那段接着记：一次安全测试的账要是完整的，不能只算复核这一半
    let mut usage = report.usage.clone();

    for f in &report.findings {
        let ask = render(
            &prompts.tool("analyst", "ask"),
            &[
                ("finding", &serde_json::to_string_pretty(f).unwrap_or_default()),
                ("evidence", &evidence_text(task_dir, f)),
            ],
        );
        let (verdict, u) = one_shot(ai, prompts, system.clone(), schema.clone(), ask).await?;
        usage.merge(&u);
        match verdict {
            Some(v) => {
                let keep = v.get("keep").and_then(|x| x.as_bool()).unwrap_or(false);
                if !keep {
                    dropped += 1;
                    continue;
                }
                let mut nf = f.clone();
                nf.confirmed = v.get("confirmed").and_then(|x| x.as_bool()).unwrap_or(false);
                if let Some(s) = v.get("severity").and_then(|x| x.as_str()) {
                    nf.severity = parse_severity(s);
                }
                if let Some(c) = v.get("category").and_then(|x| x.as_str()) {
                    nf.category = Category::parse(c);
                }
                if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
                    if !t.trim().is_empty() { nf.title = t.to_string(); }
                }
                if let Some(d) = v.get("detail").and_then(|x| x.as_str()) {
                    if !d.trim().is_empty() { nf.detail = d.to_string(); }
                }
                kept.push(nf);
            }
            // 复核拿不到结论 → 保守保留但标为未确认（疑似），交给报告标疑似，不静默丢
            None => {
                let mut nf = f.clone();
                nf.confirmed = false;
                kept.push(nf);
            }
        }
    }

    let confirmed = kept.iter().filter(|f| f.confirmed).count();
    let summary = if kept.is_empty() {
        format!("复核完成：{} 条候选全部未通过或无候选，未确认任何问题。", report.findings.len())
    } else {
        format!("复核完成：{confirmed} 条已确认、{} 条疑似待查、{dropped} 条被毙。", kept.len() - confirmed)
    };

    Ok(AnalyzedReport { target: report.target, mode: report.mode, findings: kept, summary, dropped, usage })
}

/// 精简版强制工具单次调用（INV-2 同款；与 runner::oneshot 解耦，只借 provider）。
async fn one_shot(
    ai: &AiConfig,
    prompts: &SecurityPrompts,
    system: String,
    schema: serde_json::Value,
    ask: String,
) -> Result<(Option<serde_json::Value>, super::usage::Usage)> {
    const TOOL: &str = "report";
    let tool = LlmTool::new(TOOL, prompts.tool("analyst", "report"), schema);
    let mut sess = match LlmSession::new_for_role(ai, "analyst", system, vec![tool]) {
        Ok(s) => s.with_forced_tool(TOOL),
        Err(e) => return Err(e),
    };
    sess.user(ask);
    let mut out = None;
    for attempt in 0..2 {
        match sess.next().await {
            Ok(LlmReply::ToolCalls { calls, .. }) if !calls.is_empty() => {
                out = Some(calls[0].arguments.clone());
                break;
            }
            Ok(_) if attempt == 0 => {
                sess.user(format!("请调用工具 {TOOL} 提交结果，不要用文字回复。"));
            }
            _ => break,
        }
    }
    // **失败那次也要记账**：重试一轮同样烧了 token，不记就是漏账
    let mut u = super::usage::Usage::default();
    let (p, c) = sess.total_usage();
    u.add("analyst", sess.model(), p, c);
    Ok((out, u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::agent::provider::{enqueue_fake_role_session, FakeTurn};

    fn finding(id: &str) -> Finding {
        Finding::new(id, Severity::High, Category::DataExposure, "疑似泄露", "detail")
    }

    #[tokio::test]
    async fn analyst_drops_false_positive_keeps_confirmed() {
        // fake：第一条 keep+confirmed，第二条 keep=false（毙掉）
        let ai = AiConfig { provider: Some("fake".into()), model: Some("m".into()), ..Default::default() };
        enqueue_fake_role_session("m", "analyst", vec![
            FakeTurn::tool("report", json!({"keep": true, "confirmed": true, "severity": "high", "rationale": "证据坐实"})),
        ]);
        enqueue_fake_role_session("m", "analyst", vec![
            FakeTurn::tool("report", json!({"keep": false, "confirmed": false, "rationale": "无法复现，误报"})),
        ]);
        let prompts = SecurityPrompts::load(None);
        let report = ProbeReport {
            usage: Default::default(),
            target: "https://t.example/".into(), mode: "safe".into(),
            findings: vec![finding("real"), finding("fp")],
            summary: "".into(), steps: 3,
        };
        let tmp = std::env::temp_dir();
        let out = analyze(&ai, &prompts, &tmp, report).await.unwrap();
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.findings[0].id, "real");
        assert!(out.findings[0].confirmed);
        assert_eq!(out.dropped, 1);
    }

    /// 一次安全测试的账要**完整**：prober 那一段 + 每条 finding 的复核，一条都不能漏。
    /// 平台按这个数计费（ADR-0023 D3），漏账就是少收钱、错账就是收错钱
    #[tokio::test]
    async fn 用量把探测与复核两段都算上() {
        let ai = AiConfig { provider: Some("fake".into()), model: Some("m".into()), ..Default::default() };
        for _ in 0..2 {
            enqueue_fake_role_session("m", "analyst", vec![FakeTurn {
                reply: FakeTurn::tool("report", json!({"keep": true, "confirmed": true, "rationale": "ok"})).reply,
                prompt_tokens: 60,
                completion_tokens: 10,
            }]);
        }
        let prompts = SecurityPrompts::load(None);
        // prober 那一段先记了 500+80
        let mut usage = super::super::usage::Usage::default();
        usage.add("prober", "m", 500, 80);
        let report = ProbeReport {
            usage,
            target: "https://t.example/".into(), mode: "safe".into(),
            findings: vec![finding("a"), finding("b")],
            summary: "".into(), steps: 3,
        };
        let out = analyze(&ai, &prompts, &std::env::temp_dir(), report).await.unwrap();

        assert_eq!(out.usage.by_role["prober"].prompt_tokens, 500, "prober 那段不能丢");
        assert_eq!(out.usage.by_role["analyst"].calls, 2, "每条 finding 复核一次");
        assert_eq!(out.usage.by_role["analyst"].prompt_tokens, 120);
        assert_eq!(out.usage.total_tokens, 500 + 80 + 120 + 20);
        assert!(out.usage.is_measured());
        assert_eq!(out.usage.to_json()["total_tokens"], 720);
    }

    #[tokio::test]
    async fn analyst_noop_on_zero_findings() {
        let ai = AiConfig { provider: Some("fake".into()), model: Some("m".into()), ..Default::default() };
        let prompts = SecurityPrompts::load(None);
        let report = ProbeReport {
            usage: Default::default(),
            target: "https://t.example/".into(), mode: "safe".into(),
            findings: vec![], summary: "空".into(), steps: 5,
        };
        // 无 finding → 不该触发任何 fake 会话（没 enqueue 也不报错）
        let out = analyze(&ai, &prompts, &std::env::temp_dir(), report).await.unwrap();
        assert!(out.findings.is_empty());
    }
}
