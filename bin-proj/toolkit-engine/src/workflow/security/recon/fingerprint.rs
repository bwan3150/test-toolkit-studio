//! 指纹：GET 目标，从响应头 + Set-Cookie + 响应体特征认出用了什么技术栈。
//! 全是 info 级——指纹本身不是漏洞，是攻击面测绘的起点，也帮后续判据挑方向。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

/// (响应体子串标记, 技术名)
const BODY_MARKERS: &[(&str, &str)] = &[
    ("__NEXT_DATA__", "Next.js"),
    ("window.__NUXT__", "Nuxt.js"),
    ("/wp-content/", "WordPress"),
    ("/_next/", "Next.js"),
    ("ng-version", "Angular"),
    ("data-reactroot", "React"),
    ("csrfmiddlewaretoken", "Django"),
    ("__typename", "GraphQL 前端"),
];

/// (Set-Cookie 名, 技术名)
const COOKIE_MARKERS: &[(&str, &str)] = &[
    ("PHPSESSID", "PHP"),
    ("laravel_session", "Laravel/PHP"),
    ("connect.sid", "Express/Node"),
    ("JSESSIONID", "Java (Servlet)"),
    ("csrftoken", "Django"),
    ("_rails_session", "Ruby on Rails"),
];

pub fn fingerprint_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("GET", url);
    let resp = engine.send(&req)?;
    let mut out = ReconResult::new();
    let mut techs: Vec<String> = Vec::new();

    if let Some(v) = resp.header("Server") {
        techs.push(format!("Server: {v}"));
    }
    if let Some(v) = resp.header("X-Powered-By") {
        techs.push(format!("X-Powered-By: {v}"));
    }
    // Set-Cookie 可能有多个；headers 里同名多行都在
    for (k, v) in &resp.headers {
        if k.eq_ignore_ascii_case("set-cookie") {
            for (needle, tech) in COOKIE_MARKERS {
                if v.contains(needle) {
                    techs.push(tech.to_string());
                }
            }
        }
    }
    let body = resp.text();
    // <meta name="generator" content="...">
    if let Some(gen) = extract_generator(&body) {
        techs.push(format!("generator: {gen}"));
    }
    for (needle, tech) in BODY_MARKERS {
        if body.contains(needle) {
            techs.push(tech.to_string());
        }
    }

    techs.sort();
    techs.dedup();

    if techs.is_empty() {
        out.finding(ReconFinding::new(
            "fingerprint-none",
            Severity::Info,
            "未识别出明显技术指纹",
            "响应头与页面无常见框架/服务器特征。",
        ));
    } else {
        out.finding(ReconFinding::new(
            "fingerprint",
            Severity::Info,
            format!("识别到技术栈：{}", techs.join("、")),
            "指纹用于测绘攻击面、为后续判据挑方向；本身不是漏洞。",
        ));
    }

    out.probe(req, resp);
    Ok(out)
}

/// 从 HTML 里粗取 `<meta name="generator" content="X">` 的 X（不引 HTML 解析库，够用即可）。
fn extract_generator(body: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let pos = lower.find("name=\"generator\"")?;
    // 在该标签附近找 content="..."
    let tail = &body[pos..(pos + 300).min(body.len())];
    let lower_tail = tail.to_ascii_lowercase();
    let c = lower_tail.find("content=\"")?;
    let start = c + "content=\"".len();
    let rest = &tail[start..];
    let end = rest.find('"')?;
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn detects_from_headers_cookie_and_body() {
        let eng = FakeEngine::new().route(
            "/",
            200,
            &[
                ("X-Powered-By", "Express"),
                ("Set-Cookie", "connect.sid=abc; Path=/"),
            ],
            r#"<html><head><meta name="generator" content="WordPress 6.5"></head><body><div id="__NEXT_DATA__"></div></body></html>"#,
        );
        let r = fingerprint_check(&eng, "https://t.example/").unwrap();
        let title = &r.findings[0].title;
        assert!(title.contains("Express/Node"), "得到: {title}");
        assert!(title.contains("WordPress 6.5"), "得到: {title}");
        assert!(title.contains("Next.js"), "得到: {title}");
    }

    #[test]
    fn none_when_no_markers() {
        let eng = FakeEngine::new().route("/", 200, &[], "<html>plain</html>");
        let r = fingerprint_check(&eng, "https://t.example/").unwrap();
        assert_eq!(r.findings[0].id, "fingerprint-none");
    }
}
