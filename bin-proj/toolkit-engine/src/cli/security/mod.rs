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

/// `tke recon <子命令>`。每个都是确定性、可脚本、落证据的被动/低强度检查。
#[derive(clap::Subcommand)]
pub enum ReconCommands {
    /// 安全响应头：HSTS / CSP / 点击劫持防护 / nosniff / Server 版本暴露
    Headers { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// 技术指纹：从头/Cookie/页面特征认出框架与服务器
    Fingerprint { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// CORS 配置：反射任意 Origin / 通配 / 带凭据放行
    Cors { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// GraphQL introspection 是否对外开放
    Graphql { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// Bundle 密钥扫描：JS/文本里疑似硬编码的密钥（脱敏呈现）
    Bundle { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// 常见敏感路径：.env / .git / actuator / server-status / robots …
    Endpoints { url: String, #[arg(long, default_value = "15")] timeout: u64 },
    /// 传输层（轻量）：明文 HTTP 是否强制跳 HTTPS + HSTS
    Tls { url: String, #[arg(long, default_value = "15")] timeout: u64 },
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

/// `tke recon` 处理：按 verb 选检查函数，其余（引擎、证据落盘、输出）统一。
pub async fn recon(cmd: ReconCommands, params: Arc<Params>) -> Result<()> {
    // 拆出 (verb 名, url, timeout, 检查函数)
    let (check_name, url, timeout): (&str, String, u64) = match &cmd {
        ReconCommands::Headers { url, timeout } => ("headers", url.clone(), *timeout),
        ReconCommands::Fingerprint { url, timeout } => ("fingerprint", url.clone(), *timeout),
        ReconCommands::Cors { url, timeout } => ("cors", url.clone(), *timeout),
        ReconCommands::Graphql { url, timeout } => ("graphql", url.clone(), *timeout),
        ReconCommands::Bundle { url, timeout } => ("bundle", url.clone(), *timeout),
        ReconCommands::Endpoints { url, timeout } => ("endpoints", url.clone(), *timeout),
        ReconCommands::Tls { url, timeout } => ("tls", url.clone(), *timeout),
    };

    let engine = UreqEngine::new(Duration::from_secs(timeout));
    let result = match &cmd {
        ReconCommands::Headers { .. } => recon::headers_check(&engine, &url)?,
        ReconCommands::Fingerprint { .. } => recon::fingerprint_check(&engine, &url)?,
        ReconCommands::Cors { .. } => recon::cors_check(&engine, &url)?,
        ReconCommands::Graphql { .. } => recon::graphql_check(&engine, &url)?,
        ReconCommands::Bundle { .. } => recon::bundle_check(&engine, &url)?,
        ReconCommands::Endpoints { .. } => recon::endpoints_check(&engine, &url)?,
        ReconCommands::Tls { .. } => recon::tls_check(&engine, &url)?,
    };

    // 该检查发出的每个探测都落证据（INV-14）
    let mut evidence_refs = Vec::new();
    if let Some(mut evi) = evidence_for(&params)? {
        for p in &result.probes {
            let r = evi.record(&p.request, &p.response)?;
            evidence_refs.push(serde_json::json!({
                "request": r.request.to_string_lossy(),
                "response": r.response.to_string_lossy(),
            }));
        }
    }

    JsonOutput::print(serde_json::json!({
        "success": true,
        "check": check_name,
        "url": url,
        "probe_count": result.probes.len(),
        "finding_count": result.findings.len(),
        "findings": result.findings,
        "evidence": evidence_refs,
    }));
    Ok(())
}
