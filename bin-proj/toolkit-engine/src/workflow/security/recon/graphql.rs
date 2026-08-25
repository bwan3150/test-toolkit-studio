//! GraphQL introspection 检查：向端点 POST 一个最小 introspection 查询，看 schema 是否对外开放。
//!
//! introspection 开着不是高危漏洞，但它把完整 schema（所有类型/字段/mutation）交给攻击者，
//! 大幅降低后续 GROQ/越权探测的门槛——生产环境通常应关。

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

/// 最小 introspection 查询：只问 __schema 的类型名，够判断开没开。
const INTROSPECTION_QUERY: &str = r#"{"query":"{__schema{types{name}}}"}"#;

pub fn graphql_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("POST", url)
        .header("Content-Type", "application/json")
        .body(INTROSPECTION_QUERY.as_bytes().to_vec());
    let resp = engine.send(&req)?;
    let mut out = ReconResult::new();

    let body = resp.text();
    // 开着的标志：200 且响应里带 __schema/types 结构（且不是报错）
    let looks_enabled = resp.status == 200
        && body.contains("__schema")
        && body.contains("types")
        && !body.contains("\"errors\"");

    if looks_enabled {
        out.finding(ReconFinding::new(
            "graphql-introspection-enabled",
            Severity::Low,
            "GraphQL introspection 对外开放",
            "端点返回了完整 schema。攻击者可据此枚举全部类型/字段/mutation，\
             为越权与数据探测铺路；生产环境建议关闭。",
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
    fn flags_when_schema_returned() {
        let eng = FakeEngine::new().route(
            "/graphql",
            200,
            &[("Content-Type", "application/json")],
            r#"{"data":{"__schema":{"types":[{"name":"Query"},{"name":"User"}]}}}"#,
        );
        let r = graphql_check(&eng, "https://t.example/graphql").unwrap();
        assert_eq!(r.findings[0].id, "graphql-introspection-enabled");
    }

    #[test]
    fn clean_when_introspection_disabled() {
        let eng = FakeEngine::new().route(
            "/graphql",
            200,
            &[],
            r#"{"errors":[{"message":"introspection is disabled"}]}"#,
        );
        let r = graphql_check(&eng, "https://t.example/graphql").unwrap();
        assert!(r.findings.is_empty());
    }
}
