//! 安全响应头检查：GET 目标，看关键防护头在不在。
//! 只报「缺什么」，不臆断影响面——响应头缺失是可直接复现的硬事实，正是 safe 档该给的东西。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

pub fn headers_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("GET", url);
    let resp = engine.send(&req)?;
    let mut out = ReconResult::new();

    if resp.header("Strict-Transport-Security").is_none() {
        out.finding(ReconFinding::new(
            "missing-hsts",
            Severity::Low,
            "缺少 HSTS（Strict-Transport-Security）",
            "浏览器不会强制走 HTTPS，存在 SSL 剥离与降级空间。",
        ));
    }
    if resp.header("Content-Security-Policy").is_none() {
        out.finding(ReconFinding::new(
            "missing-csp",
            Severity::Low,
            "缺少 Content-Security-Policy",
            "无 CSP，XSS 一旦存在缺少纵深防御。",
        ));
    }
    let frame_ancestors = resp
        .header("Content-Security-Policy")
        .map_or(false, |v| v.to_ascii_lowercase().contains("frame-ancestors"));
    if resp.header("X-Frame-Options").is_none() && !frame_ancestors {
        out.finding(ReconFinding::new(
            "missing-clickjacking-protection",
            Severity::Low,
            "缺少点击劫持防护（X-Frame-Options / frame-ancestors）",
            "页面可被任意站点 iframe 嵌套，存在点击劫持风险。",
        ));
    }
    if resp
        .header("X-Content-Type-Options")
        .map_or(true, |v| !v.eq_ignore_ascii_case("nosniff"))
    {
        out.finding(ReconFinding::new(
            "missing-nosniff",
            Severity::Info,
            "缺少 X-Content-Type-Options: nosniff",
            "浏览器可能对响应做 MIME 嗅探。",
        ));
    }
    if let Some(server) = resp.header("Server") {
        if server.chars().any(|c| c.is_ascii_digit()) {
            out.finding(ReconFinding::new(
                "server-banner-version",
                Severity::Info,
                format!("Server 头暴露版本：{server}"),
                "暴露组件与版本，便于攻击者比对已知漏洞。",
            ));
        }
    }

    out.probe(req, resp);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn flags_missing_security_headers() {
        let eng = FakeEngine::new().route("/", 200, &[("Server", "nginx/1.25.3")], "<html></html>");
        let r = headers_check(&eng, "https://t.example/").unwrap();
        let ids: Vec<&str> = r.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"missing-hsts"));
        assert!(ids.contains(&"missing-csp"));
        assert!(ids.contains(&"missing-clickjacking-protection"));
        assert!(ids.contains(&"missing-nosniff"));
        assert!(ids.contains(&"server-banner-version"));
        assert_eq!(r.probes.len(), 1);
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
