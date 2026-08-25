//! Bundle 密钥扫描：拉一个 JS/文本资源，用正则找疑似硬编码的密钥/令牌。
//!
//! 前端 bundle 里出现的密钥往往是真能用的（Google/AWS/Slack…），因为它被打进了浏览器下发的代码。
//! 命中即 High——但**报告里必须脱敏**（只留前缀），绝不回显完整密钥（承 P-45 精神）。
//!
//! 注意：这是「疑似」，判定仍需 analyst 复核（INV-13）；有些是公开可用的（如 Google Maps 前端 key）。

use regex::Regex;

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

/// (id, 人类可读名, 正则)
fn patterns() -> Vec<(&'static str, &'static str, Regex)> {
    let mk = |p: &str| Regex::new(p).expect("内置密钥正则应当合法");
    vec![
        ("aws-akid", "AWS Access Key ID", mk(r"AKIA[0-9A-Z]{16}")),
        ("google-api-key", "Google API Key", mk(r"AIza[0-9A-Za-z_\-]{35}")),
        ("slack-token", "Slack Token", mk(r"xox[baprs]-[0-9A-Za-z-]{10,}")),
        ("jwt", "JWT", mk(r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}")),
        ("private-key", "私钥块", mk(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
        (
            "generic-secret",
            "疑似密钥赋值",
            mk(r#"(?i)(api[_-]?key|secret|access[_-]?token|client[_-]?secret)["']?\s*[:=]\s*["'][A-Za-z0-9_\-]{16,}["']"#),
        ),
    ]
}

/// 脱敏：只留前 6 字符，其余打码——够辨认类型/定位，不泄露可用凭据。
fn redact(s: &str) -> String {
    let head: String = s.chars().take(6).collect();
    format!("{head}••••••（已脱敏，共 {} 字符）", s.chars().count())
}

pub fn bundle_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("GET", url);
    let resp = engine.send(&req)?;
    let mut out = ReconResult::new();

    let body = resp.text();
    for (id, name, re) in patterns() {
        // 每类只报第一处命中，避免同类刷屏；数量放进 detail
        let count = re.find_iter(&body).count();
        if let Some(m) = re.find(&body) {
            out.finding(ReconFinding::new(
                id,
                Severity::High,
                format!("bundle 中疑似 {name}"),
                format!(
                    "在 {url} 命中 {count} 处；样例（已脱敏）：{}。前端 bundle 里的密钥对任何访客可见——\
                     请确认是否为可公开的前端 key，否则视为泄露。",
                    redact(m.as_str())
                ),
            ));
        }
    }

    if out.findings.is_empty() {
        out.finding(ReconFinding::new(
            "bundle-clean",
            Severity::Info,
            "bundle 未命中已知密钥特征",
            "未发现 AWS/Google/Slack/JWT/私钥等常见模式；不代表绝对无泄露。",
        ));
    }

    out.probe(req, resp);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn flags_google_key_and_redacts() {
        let js = r#"const cfg={mapsKey:"AIzaSyA1234567890abcdefghijklmnopqrstuvw"};"#;
        let eng = FakeEngine::new().route("/app.js", 200, &[], js);
        let r = bundle_check(&eng, "https://t.example/app.js").unwrap();
        let f = r.findings.iter().find(|f| f.id == "google-api-key").expect("应命中 google key");
        assert_eq!(f.severity, Severity::High);
        // 脱敏：不得回显完整 key
        assert!(f.detail.contains("AIzaSy"));
        assert!(!f.detail.contains("AIzaSyA1234567890abcdefghijklmnopqrstuvw"));
    }

    #[test]
    fn clean_bundle_reports_info() {
        let eng = FakeEngine::new().route("/app.js", 200, &[], "console.log('hello world');");
        let r = bundle_check(&eng, "https://t.example/app.js").unwrap();
        assert_eq!(r.findings[0].id, "bundle-clean");
    }
}
