//! 安全领域 CLI（参数翻译层，禁止业务逻辑——逻辑在 `workflow::security`）。
//!
//! P1 只暴露两个 primitive：
//!   `tke http <METHOD> <URL> [-H k:v]... [-d body]`  —— 原始 HTTP 探测，落证据
//!   `tke recon headers <URL>`                        —— 安全响应头检查
//!
//! 两者都：走同一 HTTP 引擎、`--log <目录>` 给了就把 请求/响应 落进 `evidence/`（INV-14）。
//! AI 编排层 `tke security` 是 P2。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tke::workflow::security::evidence::EvidenceDir;
use tke::workflow::security::http::{HttpEngine, HttpRequest, UreqEngine};
use tke::workflow::security::prompt::SecurityPrompts;
use tke::workflow::security::{prober, recon};
use tke::{JsonOutput, LlmSession, Params, Result};

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

/// `tke security` —— 安全测试**唯一入口**（ADR-0019）。
/// 默认进**对话式编排**（像 tke harness：你下指令、它探测、有风险先问你）；
/// `--json` / 非终端 → **无头一次性**（内部 探测→复核→出报告，输出给 Electron/CI）。
/// prober/analyst/reporter 是内部阶段，不是子命令。
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct SecurityArgs {
    /// 目标 URL（可选：不给则由主 agent 在 TUI 里问你）
    pub url: Option<String>,
    /// 强度档：passive / safe / aggressive / red-team（不给则交互时由 agent 用选项问你）
    #[arg(long)]
    pub mode: Option<String>,
    /// 只测某一面：auth/injection/data-exposure/transport/config（不给则问你或默认全量）
    #[arg(long)]
    pub focus: Option<String>,
    /// 自定义提示词目录（布局同 builtin：agents/<role>.md、tools/<role>/<name>.md）
    #[arg(long)]
    pub prompts_dir: Option<PathBuf>,
    /// 单个内部 agent 的最大推理步数（兜底防跑飞）
    #[arg(long, default_value = "24")]
    pub max_steps: usize,
    /// 子命令（目前只有 report）；不给 = 进对话式编排
    #[command(subcommand)]
    pub action: Option<SecuritySub>,
}

/// `tke security` 的子命令。与设备轨的 `tke ui report` 对称：`tke <track> report`。
#[derive(clap::Subcommand)]
pub enum SecuritySub {
    /// 从 findings JSON 确定性出报告（无 AI）——给 skill / 脚本 / CI 用，
    /// 调用方自己收集 findings、喂进来就得到品牌 HTML 报告 + 每个确认漏洞一份。
    Report {
        /// findings JSON 文件路径（含 target/mode/findings；结构见 findings.json）
        findings: PathBuf,
        /// 报告输出目录（证据也应在此；不给则用 --log，再不给用临时目录）
        #[arg(long)]
        out: Option<PathBuf>,
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

/// 任务目录：--log 给了就用它，否则临时目录（告知路径，别把证据/报告弄丢）。
fn task_dir_of(params: &Params) -> PathBuf {
    match &params.log {
        Some(d) => d.clone(),
        None => {
            let d = std::env::temp_dir().join(format!("tke-security-{}", std::process::id()));
            eprintln!("未指定 --log，本次证据/报告落在临时目录：{}", d.display());
            d
        }
    }
}

/// 跑 prober，返回 ProbeReport（run/probe 共用）。
async fn run_prober(
    params: &Arc<Params>,
    url: &str,
    mode: &str,
    focus: &str,
    prompts: &SecurityPrompts,
    task_dir: &std::path::Path,
    max_steps: usize,
) -> Result<tke::workflow::security::finding::ProbeReport> {
    let mut evidence = EvidenceDir::new(task_dir)?;
    let ctx = prober::ProbeCtx { target: url.to_string(), mode: mode.to_string(), focus: focus.to_string() };
    let system = prober::system_prompt(prompts, &ctx);
    let tools = prober::tools(prompts);
    let session = LlmSession::new_for_role(&params.ai, "prober", system, tools)?;
    eprintln!("▶ prober 开跑：{url}（{mode} 档，聚焦 {focus}）");
    prober::run(session, &UreqEngine::default(), &mut evidence, &ctx, max_steps).await
}

/// `tke security` 处理：单一入口，按前端能力自动分流。
///   交互终端 → 对话式编排（orchestrator，复用 harness 的 Frontend/TUI）。
///   `--json` / 非终端 → 无头一次性（探测→复核→出报告，一次性 JSON 输出，给 Electron/CI）。
pub async fn security(args: SecurityArgs, params: Arc<Params>) -> Result<()> {
    use std::io::IsTerminal;

    // 子命令：report（确定性出报告，无 AI）——与交互/无头分流之前先处理
    if let Some(SecuritySub::Report { findings, out }) = &args.action {
        let dir = out.clone().or_else(|| params.log.clone())
            .unwrap_or_else(|| std::env::temp_dir().join(format!("tke-security-report-{}", std::process::id())));
        let json = std::fs::read_to_string(findings)
            .map_err(|e| tke::TkeError::InvalidArgument(format!("读不到 findings 文件 {}：{e}", findings.display())))?;
        let paths = tke::workflow::security::report::write_reports_from_json(&dir, &json)?;
        JsonOutput::print(serde_json::json!({
            "success": true,
            "report_html": paths.html.to_string_lossy(),
            "findings_json": paths.json.to_string_lossy(),
            "vuln_reports": paths.vulns.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "out_dir": dir.to_string_lossy(),
        }));
        return Ok(());
    }

    let task_dir = task_dir_of(&params);
    let prompts = SecurityPrompts::load(args.prompts_dir.clone());

    let interactive = !params.json && std::io::stdin().is_terminal() && std::io::stderr().is_terminal();

    if interactive {
        // 对话式：复用 harness 的前端（TUI，spawn 失败回落 Plain）。
        // url/mode/focus 都可能为 None——由主 agent 在 TUI 里用选项/追问补齐（用户要的开场面试）。
        let frontend: Box<dyn tke::Frontend> = match tke::TuiFrontend::spawn() {
            Ok(f) => Box::new(f),
            Err(_) => Box::new(tke::PlainFrontend::new()),
        };
        tke::workflow::security::orchestrator::run(
            &params.ai, &prompts, frontend, task_dir, args.url, args.mode, args.focus, args.max_steps,
        ).await
    } else {
        headless(&params, &prompts, &task_dir, args).await
    }
}

/// 无头一次性：探测→复核→出报告，一次性 JSON。给 `--json`/CI/Electron。
/// 无头没法交互问，所以 mode/focus 用默认（safe/全量）兜底。
async fn headless(
    params: &Arc<Params>,
    prompts: &SecurityPrompts,
    task_dir: &std::path::Path,
    args: SecurityArgs,
) -> Result<()> {
    let url = match &args.url {
        Some(u) => u.clone(),
        None => return Err(tke::TkeError::InvalidArgument(
            "无头模式（--json/非终端）需要显式给目标 URL：tke security <url> --json".into())),
    };
    let mode = args.mode.clone().unwrap_or_else(|| "safe".to_string());
    let focus = args.focus.clone().unwrap_or_else(|| "全量".to_string());
    let probe = run_prober(params, &url, &mode, &focus, prompts, task_dir, args.max_steps).await?;
    let analyzed = tke::workflow::security::analyst::analyze(&params.ai, prompts, task_dir, probe).await?;
    let paths = tke::workflow::security::report::write_reports(task_dir, &analyzed)?;

    JsonOutput::print(serde_json::json!({
        "success": true,
        "target": analyzed.target,
        "mode": analyzed.mode,
        "focus": focus,
        "summary": analyzed.summary,
        "finding_count": analyzed.findings.len(),
        "confirmed": analyzed.findings.iter().filter(|f| f.confirmed).count(),
        "dropped": analyzed.dropped,
        "report_html": paths.html.to_string_lossy(),
        "findings_json": paths.json.to_string_lossy(),
        "vuln_reports": paths.vulns.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
        "task_dir": task_dir.to_string_lossy(),
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
