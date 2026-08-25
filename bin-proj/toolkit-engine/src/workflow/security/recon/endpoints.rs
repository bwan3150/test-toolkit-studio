//! 常见敏感路径探测：对目标 origin 逐个 GET 一批「不该公开」的路径，看有没有 200 且内容对得上。
//!
//! 关键防假阳：很多 SPA 对任意路径都回 200 + HTML（catch-all）。所以命中判据不只看状态码，
//! 还要 (a) 内容类型不是 HTML 或 (b) 响应体带该文件的特征签名——否则只是被首页兜底了。
//!
//! P1 做常见路径；吃 OpenAPI/Swagger 把声明端点纳入攻击面是后续（信息层级，见 ADR-0019）。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest, HttpResponse};
use super::{ReconFinding, ReconResult, Severity};

/// 一条待探路径的判据。`sig` 收到响应体，返回「确实是这个东西」。
struct Target {
    path: &'static str,
    id: &'static str,
    severity: Severity,
    title: &'static str,
    detail: &'static str,
    sig: fn(&str) -> bool,
}

fn always(_: &str) -> bool { true }

fn targets() -> Vec<Target> {
    vec![
        Target { path: "/.env", id: "exposed-dotenv", severity: Severity::High,
            title: "暴露 .env 环境文件", detail: "根目录可读取 .env，通常含数据库口令、密钥。",
            sig: |b| b.contains('=') && (b.contains("KEY") || b.contains("SECRET") || b.contains("PASSWORD") || b.contains("DB_")) },
        Target { path: "/.git/HEAD", id: "exposed-git", severity: Severity::High,
            title: "暴露 .git 目录", detail: "可读 .git，能还原源码与历史（含可能被删的密钥提交）。",
            sig: |b| b.trim_start().starts_with("ref:") },
        Target { path: "/.git/config", id: "exposed-git-config", severity: Severity::High,
            title: "暴露 .git/config", detail: "泄露仓库地址与配置，佐证 .git 目录可读。",
            sig: |b| b.contains("[core]") },
        Target { path: "/server-status", id: "apache-server-status", severity: Severity::Medium,
            title: "Apache server-status 对外开放", detail: "泄露实时请求、客户端 IP、内部路径。",
            sig: |b| b.contains("Apache Server Status") },
        Target { path: "/actuator/env", id: "spring-actuator-env", severity: Severity::High,
            title: "Spring Actuator /env 对外开放", detail: "泄露全部环境变量与配置，常含凭据。",
            sig: |b| b.contains("\"propertySources\"") || b.contains("systemEnvironment") },
        Target { path: "/.DS_Store", id: "exposed-dsstore", severity: Severity::Low,
            title: "暴露 .DS_Store", detail: "泄露目录结构，辅助进一步枚举。",
            sig: |b| b.starts_with("\u{0}\u{0}\u{0}\u{1}Bud1") || b.contains("Bud1") },
        Target { path: "/.well-known/security.txt", id: "security-txt", severity: Severity::Info,
            title: "存在 security.txt（良好实践）", detail: "提供了安全联系方式，属正向信号。",
            sig: always },
        Target { path: "/robots.txt", id: "robots", severity: Severity::Info,
            title: "robots.txt 可读", detail: "其中的 Disallow 路径可作为攻击面线索。",
            sig: |b| b.to_ascii_lowercase().contains("user-agent") || b.to_ascii_lowercase().contains("disallow") },
    ]
}

/// 从任意 URL 取 origin：`scheme://host[:port]`。
fn origin_of(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest.split('/').next().unwrap_or(rest);
            format!("{scheme}://{host}")
        }
        None => url.trim_end_matches('/').to_string(),
    }
}

fn is_html(resp: &HttpResponse) -> bool {
    resp.header("Content-Type").map_or(false, |c| c.to_ascii_lowercase().contains("text/html"))
}

pub fn endpoints_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let origin = origin_of(url);
    let mut out = ReconResult::new();

    for t in targets() {
        let full = format!("{origin}{}", t.path);
        let req = HttpRequest::new("GET", &full);
        let resp = match engine.send(&req) {
            Ok(r) => r,
            Err(_) => continue, // 单条探测失败不影响其余
        };

        // 命中判据：200 + 非 HTML 兜底 + 内容签名对得上
        let hit = resp.status == 200 && !is_html(&resp) && (t.sig)(&resp.text());
        if hit {
            out.finding(ReconFinding::new(
                t.id,
                t.severity,
                format!("{}（{}）", t.title, t.path),
                t.detail,
            ));
        }
        out.probe(req, resp);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn origin_extraction() {
        assert_eq!(origin_of("https://t.example/a/b?x=1"), "https://t.example");
        assert_eq!(origin_of("http://h:8080/"), "http://h:8080");
    }

    #[test]
    fn flags_exposed_dotenv_but_not_spa_catchall() {
        // .env 返回真 env 内容（非 HTML）→ 命中；其余路径被 SPA 用 HTML 兜底 → 不算
        let eng = FakeEngine {
            routes: vec![],
            strict: false, // 未列出的路径回 404
        }
        .route("/.env", 200, &[("Content-Type", "text/plain")], "DB_PASSWORD=secret\nAPI_KEY=xyz")
        .route("/robots.txt", 200, &[("Content-Type", "text/html")], "<html>catch-all</html>");

        let r = endpoints_check(&eng, "https://t.example/").unwrap();
        let ids: Vec<&str> = r.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"exposed-dotenv"), "应命中 .env");
        // robots 被 HTML 兜底，不该当成真 robots
        assert!(!ids.contains(&"robots"), "HTML 兜底不该算命中");
        assert!(r.probes.len() >= 2);
    }

    #[test]
    fn clean_site_no_findings() {
        let mut eng = FakeEngine::new();
        eng.strict = false; // 全部 404
        let r = endpoints_check(&eng, "https://t.example/").unwrap();
        assert!(r.findings.is_empty());
    }
}
