//! 后端服务识别：从一个 URL（首页或 JS bundle）扒出**后端服务标识**——projectId/dataset、
//! supabaseUrl、firebaseConfig、algolia appId、GraphQL 端点、S3 桶……
//!
//! 这是「往后端深挖」的桥：`fingerprint` 认框架、`bundle` 扫密钥，`detect` 专门把**能继续追的
//! 后端标识**捞出来递给调用方。命中都是 **info 级线索**（不是漏洞本身）——detail 里附上对照
//! service-playbook 的**零凭据探测式**，由 AI/analyst 真打一发确认才算漏洞（INV-13）。

use regex::Regex;

use crate::Result;
use super::super::http::{HttpEngine, HttpRequest};
use super::{ReconFinding, ReconResult, Severity};

fn re(p: &str) -> Regex {
    Regex::new(p).expect("内置 detect 正则应当合法")
}

pub fn detect_check(engine: &dyn HttpEngine, url: &str) -> Result<ReconResult> {
    let req = HttpRequest::new("GET", url);
    let resp = engine.send(&req)?;
    let body = resp.text();
    let mut out = ReconResult::new();

    // ── Sanity：projectId + dataset（或 <pid>.api.sanity.io）──
    let sanity_pid = re(r#"(?:"|')?projectId(?:"|')?\s*[:=]\s*(?:"|')([a-z0-9]{6,})(?:"|')"#)
        .captures(&body).map(|c| c[1].to_string())
        .or_else(|| re(r"([a-z0-9]{6,})\.api(?:cdn)?\.sanity\.io").captures(&body).map(|c| c[1].to_string()));
    if let Some(pid) = sanity_pid {
        let dataset = re(r#"(?:"|')?dataset(?:"|')?\s*[:=]\s*(?:"|')([a-z0-9_-]+)(?:"|')"#)
            .captures(&body).map(|c| c[1].to_string()).unwrap_or_else(|| "production".into());
        out.finding(ReconFinding::new(
            "detect-sanity", Severity::Info,
            format!("识别到 Sanity 后端：projectId={pid} dataset={dataset}"),
            format!("零凭据探测（public dataset 可能全量可读）：\
                GET https://{pid}.api.sanity.io/v2021-06-07/data/query/{dataset}?query=count(*) —— 有返回即 public。\
                对照 service-playbook 确认是否含敏感业务数据。"),
        ));
    }

    // ── Supabase：<ref>.supabase.co ──
    if let Some(c) = re(r"https://([a-z0-9]{10,})\.supabase\.co").captures(&body) {
        let refid = c[1].to_string();
        out.finding(ReconFinding::new(
            "detect-supabase", Severity::Info,
            format!("识别到 Supabase 后端：{refid}.supabase.co"),
            "缺 RLS 时凭公开 anon key 可读全表。用 bundle 里的 anon(JWT) 打 \
             GET https://<ref>.supabase.co/rest/v1/<表>?select=* -H 'apikey: <anon>'——返回行=缺 RLS。",
        ));
    }

    // ── Firebase：*.firebaseio.com ──
    if let Some(c) = re(r"([a-z0-9-]+)\.firebaseio\.com").captures(&body) {
        let proj = c[1].to_string();
        out.finding(ReconFinding::new(
            "detect-firebase", Severity::Info,
            format!("识别到 Firebase RTDB：{proj}.firebaseio.com"),
            format!("规则 read:true 时整库可读：GET https://{proj}.firebaseio.com/.json?shallow=true —— 非 null 即开放。"),
        ));
    }

    // ── Algolia：<appId>-dsn.algolia.net ──
    if let Some(c) = re(r"([A-Z0-9]{8,})-dsn\.algolia\.net").captures(&body) {
        out.finding(ReconFinding::new(
            "detect-algolia", Severity::Info,
            format!("识别到 Algolia：appId={}", &c[1]),
            "前端只该放 search-only key；若 bundle 里的是 admin key 则可改索引/读全量（对照 playbook 验权限）。",
        ));
    }

    // ── GraphQL 端点 ──
    if let Some(c) = re(r#"["'](/v1/graphql|/graphql)["']"#).captures(&body) {
        out.finding(ReconFinding::new(
            "detect-graphql", Severity::Info,
            format!("识别到 GraphQL 端点：{}", &c[1]),
            "跑 recon graphql 看 introspection 是否开放；再无 auth 头 query 业务表看匿名能不能读。",
        ));
    }

    // ── S3 桶 ──
    if let Some(c) = re(r"([a-z0-9][a-z0-9.-]{2,})\.s3[.-][a-z0-9-]*\.?amazonaws\.com").captures(&body) {
        out.finding(ReconFinding::new(
            "detect-s3", Severity::Info,
            format!("识别到 S3 桶：{}", &c[1]),
            "可能公开可列：GET https://<bucket>.s3.amazonaws.com/ —— 返回 <ListBucketResult> 且含非公开资产=泄露。",
        ));
    }

    if out.findings.is_empty() {
        out.finding(ReconFinding::new(
            "detect-none", Severity::Info,
            "未识别出常见后端服务标识",
            "此 URL 没扒到 Sanity/Supabase/Firebase/Algolia/GraphQL/S3 等标识。换真正的 JS bundle 再试。",
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
    fn detects_sanity_project_and_dataset() {
        let js = r#"const cfg={projectId:"dv01o3wh",dataset:"production",apiVersion:"2021-06-07"};"#;
        let eng = FakeEngine::new().route("/app.js", 200, &[], js);
        let r = detect_check(&eng, "https://t.example/app.js").unwrap();
        let f = r.findings.iter().find(|f| f.id == "detect-sanity").expect("应识别 Sanity");
        assert!(f.title.contains("dv01o3wh"));
        assert!(f.title.contains("production"));
        assert!(f.detail.contains("data/query"));
    }

    #[test]
    fn detects_supabase_url() {
        let eng = FakeEngine::new().route("/", 200, &[], r#"createClient("https://abcdefghij.supabase.co","eyJhbGc")"#);
        let r = detect_check(&eng, "https://t.example/").unwrap();
        assert!(r.findings.iter().any(|f| f.id == "detect-supabase"));
    }

    #[test]
    fn none_when_plain() {
        let eng = FakeEngine::new().route("/", 200, &[], "console.log('hi')");
        let r = detect_check(&eng, "https://t.example/").unwrap();
        assert_eq!(r.findings[0].id, "detect-none");
    }
}
