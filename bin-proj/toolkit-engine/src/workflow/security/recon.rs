//! 侦察检查（curated）：在 HTTP 原语之上，把常见的**被动**判据固化成确定性检查。
//!
//! 这些是 primitive 层（无 AI）：`tke recon headers <url>` 可直接脚本化调用，也被 P2 的
//! recon agent 当工具用——同一份实现。判据只做「被动/只读」的那类（safe 档也能跑，见 ADR-0019）。
//!
//! P1 先落 `headers`（安全响应头）。后续按 platform-matrix 的思路逐个加：cors / graphql / bundle / tls / …

use serde::Serialize;

use crate::Result;
use super::http::{HttpEngine, HttpRequest, HttpResponse};

/// 严重度（与报告 spec 的五级一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// 一条侦察发现。`confirmed` 恒为被动观察到的事实（响应头缺失是硬事实），
/// 但仍不等于「漏洞判定」——判定要过 P2 的 analyst 闸门（INV-13）。
#[derive(Debug, Clone, Serialize)]
pub struct ReconFinding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
}

/// 一次侦察检查的结果：命中的 findings + 原始响应（供证据落盘）。
pub struct ReconResult {
    pub response: HttpResponse,
    pub findings: Vec<ReconFinding>,
}

/// 安全响应头检查：GET 目标，看关键防护头在不在。
///
/// 只报「缺什么」，不臆断影响面——响应头缺失是可直接复现的硬事实，正是 safe 档该给的东西。
pub fn headers_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let resp = engine.send(&HttpRequest::new("GET", url))?;
    let mut findings = Vec::new();

    if resp.header("Strict-Transport-Security").is_none() {
        findings.push(ReconFinding {
            id: "missing-hsts".into(),
            severity: Severity::Low,
            title: "缺少 HSTS（Strict-Transport-Security）".into(),
            detail: "浏览器不会强制走 HTTPS，存在 SSL 剥离与降级空间。".into(),
        });
    }
    if resp.header("Content-Security-Policy").is_none() {
        findings.push(ReconFinding {
            id: "missing-csp".into(),
            severity: Severity::Low,
            title: "缺少 Content-Security-Policy".into(),
            detail: "无 CSP，XSS 一旦存在缺少纵深防御。".into(),
        });
    }
    // X-Frame-Options 或 CSP frame-ancestors 二者有其一即可防点击劫持
    let frame_ancestors = resp
        .header("Content-Security-Policy")
        .map_or(false, |v| v.to_ascii_lowercase().contains("frame-ancestors"));
    if resp.header("X-Frame-Options").is_none() && !frame_ancestors {
        findings.push(ReconFinding {
            id: "missing-clickjacking-protection".into(),
            severity: Severity::Low,
            title: "缺少点击劫持防护（X-Frame-Options / frame-ancestors）".into(),
            detail: "页面可被任意站点 iframe 嵌套，存在点击劫持风险。".into(),
        });
    }
    if resp
        .header("X-Content-Type-Options")
        .map_or(true, |v| !v.eq_ignore_ascii_case("nosniff"))
    {
        findings.push(ReconFinding {
            id: "missing-nosniff".into(),
            severity: Severity::Info,
            title: "缺少 X-Content-Type-Options: nosniff".into(),
            detail: "浏览器可能对响应做 MIME 嗅探。".into(),
        });
    }
    if let Some(server) = resp.header("Server") {
        // 带数字通常意味着暴露了具体版本号
        if server.chars().any(|c| c.is_ascii_digit()) {
            findings.push(ReconFinding {
                id: "server-banner-version".into(),
                severity: Severity::Info,
                title: format!("Server 头暴露版本：{server}"),
                detail: "暴露组件与版本，便于攻击者比对已知漏洞。".into(),
            });
        }
    }

    Ok(ReconResult { response: resp, findings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::http::FakeEngine;

    #[test]
    fn flags_missing_security_headers() {
        // 一个「什么防护头都没有 + 暴露版本」的响应
        let eng = FakeEngine::new().route("/", 200, &[("Server", "nginx/1.25.3")], "<html></html>");
        let r = headers_check(&eng, "https://t.example/").unwrap();
        let ids: Vec<&str> = r.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"missing-hsts"));
        assert!(ids.contains(&"missing-csp"));
        assert!(ids.contains(&"missing-clickjacking-protection"));
        assert!(ids.contains(&"missing-nosniff"));
        assert!(ids.contains(&"server-banner-version"));
    }

    #[test]
    fn clean_when_all_headers_present() {
        let eng = FakeEngine::new().route(
            "/",
            200,
            &[
                ("Strict-Transport-Security", "max-age=63072000"),
                ("Content-Security-Policy", "default-src 'self'; frame-ancestors 'none'"),
                ("X-Content-Type-Options", "nosniff"),
                ("Server", "nginx"),
            ],
            "ok",
        );
        let r = headers_check(&eng, "https://t.example/").unwrap();
        assert!(r.findings.is_empty(), "全头齐备不该有 finding，实得: {:?}", r.findings);
    }
}
