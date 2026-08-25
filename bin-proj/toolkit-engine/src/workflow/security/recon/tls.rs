//! 传输层检查（轻量版）：从 HTTP 层能观察到的传输安全问题。
//!
//! P1 只做两条 HTTP 可观察的判据：
//!   1. 明文 http:// 是否被强制跳转到 https（不跳 = 明文可访问，Medium）。
//!   2. https 响应有没有 HSTS（从传输视角，与 headers 检查互补）。
//!
//! TODO(P1 续)：证书链/有效期/协议版本/弱套件——需接 TLS 库（rustls）读握手，非 HTTP 层能拿到。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

fn origin_host(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest).to_string()
}

pub fn tls_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let host = origin_host(url);
    let mut out = ReconResult::new();

    // 1. 明文 HTTP：跳转还是直服？
    let http_url = format!("http://{host}/");
    let http_req = HttpRequest::new("GET", &http_url);
    if let Ok(resp) = engine.send(&http_req) {
        let is_redirect_to_https = (300..400).contains(&resp.status)
            && resp
                .header("Location")
                .map_or(false, |l| l.trim_start().to_ascii_lowercase().starts_with("https://"));
        if resp.status == 200 {
            out.finding(ReconFinding::new(
                "plaintext-http-served",
                Severity::Medium,
                "明文 HTTP 直接提供内容，未强制跳转 HTTPS",
                "http:// 返回 200 而非跳转到 https，存在明文传输与降级/中间人风险。",
            ));
        } else if !is_redirect_to_https && (300..400).contains(&resp.status) {
            out.finding(ReconFinding::new(
                "http-redirect-not-https",
                Severity::Low,
                "HTTP 有跳转但目标不是 HTTPS",
                format!("http:// 返回 {} 但 Location 未指向 https。", resp.status),
            ));
        }
        out.probe(http_req, resp);
    }

    // 2. HTTPS 上的 HSTS
    let https_url = format!("https://{host}/");
    let https_req = HttpRequest::new("GET", &https_url);
    if let Ok(resp) = engine.send(&https_req) {
        if resp.header("Strict-Transport-Security").is_none() {
            out.finding(ReconFinding::new(
                "tls-no-hsts",
                Severity::Low,
                "HTTPS 响应缺少 HSTS",
                "无 Strict-Transport-Security，首次访问仍可能被降级到明文。",
            ));
        }
        out.probe(https_req, resp);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn flags_plaintext_http_served() {
        // http:// 直接 200，https:// 带 HSTS
        let eng = FakeEngine {
            routes: vec![],
            strict: false,
        }
        .route("http://t.example/", 200, &[], "<html>plain</html>")
        .route("https://t.example/", 200, &[("Strict-Transport-Security", "max-age=1")], "ok");

        let r = tls_check(&eng, "https://t.example/x").unwrap();
        let ids: Vec<&str> = r.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"plaintext-http-served"));
        assert!(!ids.contains(&"tls-no-hsts"), "https 带了 HSTS 不该报");
    }

    #[test]
    fn clean_when_http_redirects_https_and_hsts_present() {
        let eng = FakeEngine {
            routes: vec![],
            strict: false,
        }
        .route("http://t.example/", 301, &[("Location", "https://t.example/")], "")
        .route("https://t.example/", 200, &[("Strict-Transport-Security", "max-age=63072000")], "ok");

        let r = tls_check(&eng, "https://t.example/").unwrap();
        assert!(r.findings.is_empty(), "规范配置不该有 finding，实得: {:?}", r.findings.iter().map(|f| &f.id).collect::<Vec<_>>());
    }
}
