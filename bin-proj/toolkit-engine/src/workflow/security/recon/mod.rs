//! 侦察检查（curated）：在 HTTP 原语之上，把常见**被动/低强度**判据固化成确定性检查。
//!
//! 每个 verb 是一个 primitive（无 AI）：`tke recon <verb> <url>` 可直接脚本化，也被 P2 的
//! recon agent 当工具用——同一份实现。判据默认落在 passive/safe 档能跑的范围（ADR-0019）。
//!
//! 统一结果结构 `ReconResult`：findings（命中的问题）+ probes（该检查发出的所有 请求/响应，
//! 供证据落盘，INV-14）。单请求检查 = 1 个 probe，多路径检查（endpoints）= N 个 probe。

use serde::Serialize;

use super::http::{HttpRequest, HttpResponse};

pub mod headers;
pub mod fingerprint;
pub mod cors;
pub mod graphql;
pub mod bundle;
pub mod endpoints;
pub mod tls;

pub use headers::headers_check;
pub use fingerprint::fingerprint_check;
pub use cors::cors_check;
pub use graphql::graphql_check;
pub use bundle::bundle_check;
pub use endpoints::endpoints_check;
pub use tls::tls_check;

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

/// 一条侦察发现。这里的都是**被动观察到的事实**（头缺失、端点可读、密钥出现在 bundle）——
/// 是硬事实，但仍不等于「漏洞判定」；判定要过 P2 的 analyst 闸门（INV-13）。
#[derive(Debug, Clone, Serialize)]
pub struct ReconFinding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
}

impl ReconFinding {
    pub fn new(id: &str, severity: Severity, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { id: id.into(), severity, title: title.into(), detail: detail.into() }
    }
}

/// 一次探测的 请求/响应 对（供证据落盘）。
pub struct Probe {
    pub request: HttpRequest,
    pub response: HttpResponse,
}

/// 一次侦察检查的结果。
pub struct ReconResult {
    pub findings: Vec<ReconFinding>,
    pub probes: Vec<Probe>,
}

impl ReconResult {
    pub fn new() -> Self {
        Self { findings: Vec::new(), probes: Vec::new() }
    }
    /// 追加一次探测记录。
    pub fn probe(&mut self, request: HttpRequest, response: HttpResponse) {
        self.probes.push(Probe { request, response });
    }
    /// 追加一条 finding。
    pub fn finding(&mut self, f: ReconFinding) {
        self.findings.push(f);
    }
    /// 最近一次探测的响应（多数检查只发一个请求，方便取用）。
    pub fn last_response(&self) -> Option<&HttpResponse> {
        self.probes.last().map(|p| &p.response)
    }
}

impl Default for ReconResult {
    fn default() -> Self {
        Self::new()
    }
}
