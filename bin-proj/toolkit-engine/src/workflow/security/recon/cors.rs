//! CORS 配置检查：带一个「攻击者来源」的 Origin 发请求，看服务端 CORS 响应头怎么放行。
//!
//! 最危险的组合是「反射任意 Origin + Allow-Credentials: true」——等于任何站点都能带着
//! 用户 Cookie 读你的接口。`*` 通配在不带凭据时危害小（浏览器禁止 `*` + 凭据）。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

/// 探测用的假来源——一个明显不属于目标的站点。
const EVIL_ORIGIN: &str = "https://evil.example";

pub fn cors_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("GET", url).header("Origin", EVIL_ORIGIN);
    let resp = engine.send(&req)?;
    let mut out = ReconResult::new();

    let acao = resp.header("Access-Control-Allow-Origin").map(|s| s.to_string());
    let acac = resp
        .header("Access-Control-Allow-Credentials")
        .map_or(false, |v| v.eq_ignore_ascii_case("true"));

    match acao.as_deref() {
        Some(o) if o == EVIL_ORIGIN => {
            if acac {
                out.finding(ReconFinding::new(
                    "cors-reflect-origin-with-credentials",
                    Severity::High,
                    "CORS 反射任意 Origin 且允许携带凭据",
                    "服务端把请求的 Origin 原样回显进 Allow-Origin，并 Allow-Credentials: true——\
                     任何站点都能带用户 Cookie 跨域读取本接口。",
                ));
            } else {
                out.finding(ReconFinding::new(
                    "cors-reflect-origin",
                    Severity::Low,
                    "CORS 反射任意 Origin（未带凭据）",
                    "Allow-Origin 回显请求 Origin，但未开 Allow-Credentials，危害有限；仍建议白名单。",
                ));
            }
        }
        Some("*") => {
            out.finding(ReconFinding::new(
                "cors-wildcard",
                Severity::Info,
                "CORS 使用通配 *",
                "对公开、无凭据的接口通常可接受；若接口返回敏感数据应改白名单。",
            ));
        }
        _ => {}
    }

    out.probe(req, resp);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::http::FakeEngine;

    #[test]
    fn high_when_reflect_origin_with_credentials() {
        let eng = FakeEngine::new().route(
            "/api",
            200,
            &[
                ("Access-Control-Allow-Origin", EVIL_ORIGIN),
                ("Access-Control-Allow-Credentials", "true"),
            ],
            "{}",
        );
        let r = cors_check(&eng, "https://t.example/api").unwrap();
        assert_eq!(r.findings[0].id, "cors-reflect-origin-with-credentials");
        assert_eq!(r.findings[0].severity, Severity::High);
    }

    #[test]
    fn wildcard_is_info() {
        let eng = FakeEngine::new().route("/api", 200, &[("Access-Control-Allow-Origin", "*")], "{}");
        let r = cors_check(&eng, "https://t.example/api").unwrap();
        assert_eq!(r.findings[0].id, "cors-wildcard");
    }

    #[test]
    fn clean_when_no_cors_headers() {
        let eng = FakeEngine::new().route("/api", 200, &[], "{}");
        let r = cors_check(&eng, "https://t.example/api").unwrap();
        assert!(r.findings.is_empty());
    }
}
