//! 安全领域 CLI（参数翻译层，禁止业务逻辑——逻辑在 `workflow::security`）。
//!
//! P1 只暴露两个 primitive：
//!   `tke http <METHOD> <URL> [-H k:v]... [-d body]`  —— 原始 HTTP 探测，落证据
//!   `tke recon headers <URL>`                        —— 安全响应头检查
//!
//! 两者都：走同一 HTTP 引擎、`--log <目录>` 给了就把 请求/响应 落进 `evidence/`（INV-14）。
//! AI 编排层 `tke security` 是 P2。

use std::sync::Arc;
use std::time::Duration;

use tke::workflow::security::evidence::EvidenceDir;
use tke::workflow::security::http::{HttpEngine, HttpRequest, UreqEngine};
use tke::workflow::security::recon;
use tke::{JsonOutput, Params, Result};

/// `tke http` 参数。
#[derive(clap::Args)]
pub struct HttpArgs {
    /// HTTP 方法（GET/POST/PUT/DELETE/…）
    pub method: String,
    /// 目标 URL
    pub url: String,
    /// 附加请求头，可多次：`-H 'Authorization: Bearer x'`
    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,
    /// 请求体（原样发送）。注意：短名 `-d` 被全局 `--device` 占用，这里只用长名 `--data`
    #[arg(long = "data")]
    pub data: Option<String>,
    /// 超时秒数
    #[arg(long, default_value = "15")]
    pub timeout: u64,
}

/// `tke recon <子命令>`。
#[derive(clap::Subcommand)]
pub enum ReconCommands {
    /// 安全响应头检查：HSTS / CSP / 点击劫持防护 / nosniff / Server 版本暴露
    Headers {
        /// 目标 URL
        url: String,
        /// 超时秒数
        #[arg(long, default_value = "15")]
        timeout: u64,
    },
}

/// 把 `-H 'K: V'` 列表解析成键值对（容忍无空格的 `K:V`）。
fn parse_headers(raw: &[String]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for h in raw {
        match h.split_once(':') {
            Some((k, v)) => out.push((k.trim().to_string(), v.trim().to_string())),
            None => {
                return Err(tke::TkeError::InvalidArgument(format!(
                    "请求头格式应为 'K: V'，收到: {h}"
                )))
            }
        }
    }
    Ok(out)
}

/// 若 `--log` 给了目录，开一个证据目录；否则 None（探测照跑，只是不留档）。
fn evidence_for(params: &Params) -> Result<Option<EvidenceDir>> {
    match &params.log {
        Some(dir) => Ok(Some(EvidenceDir::new(dir)?)),
        None => Ok(None),
    }
}

/// `tke http` 处理。
pub async fn http(args: HttpArgs, params: Arc<Params>) -> Result<()> {
    let engine = UreqEngine::new(Duration::from_secs(args.timeout));

    let mut req = HttpRequest::new(args.method.to_uppercase(), &args.url);
    req.headers = parse_headers(&args.headers)?;
    if let Some(d) = args.data {
        req = req.body(d.into_bytes());
    }

    let resp = engine.send(&req)?;

    let mut evidence_paths = serde_json::Value::Null;
    if let Some(mut evi) = evidence_for(&params)? {
        let r = evi.record(&req, &resp)?;
        evidence_paths = serde_json::json!({
            "request": r.request.to_string_lossy(),
            "response": r.response.to_string_lossy(),
        });
    }

    JsonOutput::print(serde_json::json!({
        "success": true,
        "method": req.method,
        "url": req.url,
        "status": resp.status,
        "elapsed_ms": resp.elapsed_ms,
        "header_count": resp.headers.len(),
        "body_bytes": resp.body.len(),
        "truncated": resp.truncated,
        "evidence": evidence_paths,
    }));
    Ok(())
}

/// `tke recon` 处理。
pub async fn recon(cmd: ReconCommands, params: Arc<Params>) -> Result<()> {
    match cmd {
        ReconCommands::Headers { url, timeout } => {
            let engine = UreqEngine::new(Duration::from_secs(timeout));
            let result = recon::headers_check(&engine, &url)?;

            let mut evidence_paths = serde_json::Value::Null;
            if let Some(mut evi) = evidence_for(&params)? {
                let r = evi.record(&HttpRequest::new("GET", &url), &result.response)?;
                evidence_paths = serde_json::json!({
                    "request": r.request.to_string_lossy(),
                    "response": r.response.to_string_lossy(),
                });
            }

            JsonOutput::print(serde_json::json!({
                "success": true,
                "check": "headers",
                "url": url,
                "status": result.response.status,
                "finding_count": result.findings.len(),
                "findings": result.findings,
                "evidence": evidence_paths,
            }));
            Ok(())
        }
    }
}
