//! reporter —— 把复核后的 AnalyzedReport 落成**自包含 HTML** + 机器可读 findings.json。
//!
//! **确定性生成**（不走 LLM）：结构化 findings 已经由 prober/analyst 产好，报告只负责渲染——
//! 这样可单测、可回归、不会因模型漂移而变样（技法承 workflow/report.rs）。风格对齐用户确认过的
//! 模板基线：暖色计分板 + 严重度环形 + Toolkit 品牌，亮/暗自适应，离线可开。
//!
//! 产物：`security-report.html`（全局）+ `findings.json`（给 --json/Electron）+
//! 每个**已确认**发现一份 `vuln-<id>.html`（软证据/疑似只进全局清单，不单独出——INV-13）。

use std::path::{Path, PathBuf};

use crate::{Result, TkeError};
use super::analyst::AnalyzedReport;
use super::finding::{Category, Finding, Severity};

const LOGO: &str = include_str!("report_assets/logo.txt");

/// 生成的报告文件路径。
pub struct ReportPaths {
    pub html: PathBuf,
    pub json: PathBuf,
    pub vulns: Vec<PathBuf>,
}

/// 在 `dir` 下生成全部报告文件。
pub fn write_reports(dir: &Path, r: &AnalyzedReport) -> Result<ReportPaths> {
    std::fs::create_dir_all(dir).map_err(TkeError::IoError)?;

    // findings.json
    let json_path = dir.join("findings.json");
    let json = serde_json::json!({
        "target": r.target,
        "mode": r.mode,
        "outcome": outcome(r),
        "score": grade(r).0,
        "summary": r.summary,
        "counts": counts_json(r),
        "findings": r.findings,
    });
    std::fs::write(&json_path, serde_json::to_string_pretty(&json).unwrap_or_default())
        .map_err(TkeError::IoError)?;

    // 每个已确认发现一份单独报告
    let mut vulns = Vec::new();
    for f in r.findings.iter().filter(|f| f.confirmed) {
        let p = dir.join(format!("vuln-{}.html", slug(&f.id)));
        std::fs::write(&p, render_vuln(r, f)).map_err(TkeError::IoError)?;
        vulns.push(p);
    }

    // 全局报告
    let html_path = dir.join("security-report.html");
    std::fs::write(&html_path, render_global(r)).map_err(TkeError::IoError)?;

    Ok(ReportPaths { html: html_path, json: json_path, vulns })
}

// ── 评级 / 计数 ────────────────────────────────────────────────────────

fn max_severity(r: &AnalyzedReport) -> Option<Severity> {
    r.findings.iter().map(|f| f.severity).max_by_key(sev_rank)
}

fn sev_rank(s: &Severity) -> u8 {
    match s { Severity::Critical => 5, Severity::High => 4, Severity::Medium => 3, Severity::Low => 2, Severity::Info => 1 }
}

/// (字母评级, 中文风险总评)
fn grade(r: &AnalyzedReport) -> (&'static str, &'static str) {
    match max_severity(r) {
        Some(Severity::Critical) => ("F", "严重"),
        Some(Severity::High) => ("D", "高危"),
        Some(Severity::Medium) => ("C", "中危"),
        Some(Severity::Low) => ("B", "低危"),
        Some(Severity::Info) | None => ("A", "良好"),
    }
}

fn outcome(r: &AnalyzedReport) -> &'static str {
    if r.findings.iter().any(|f| f.confirmed) { "findings" } else { "clean" }
}

fn count(r: &AnalyzedReport, s: Severity) -> usize {
    r.findings.iter().filter(|f| f.severity == s).count()
}

fn counts_json(r: &AnalyzedReport) -> serde_json::Value {
    serde_json::json!({
        "critical": count(r, Severity::Critical),
        "high": count(r, Severity::High),
        "medium": count(r, Severity::Medium),
        "low": count(r, Severity::Low),
        "info": count(r, Severity::Info),
    })
}

fn sev_class(s: Severity) -> &'static str {
    match s { Severity::Critical => "c", Severity::High => "h", Severity::Medium => "m", Severity::Low => "l", Severity::Info => "i" }
}
fn sev_label(s: Severity) -> &'static str {
    match s { Severity::Critical => "严重", Severity::High => "高危", Severity::Medium => "中危", Severity::Low => "低危", Severity::Info => "信息" }
}
fn cat_label(c: Category) -> &'static str {
    match c {
        Category::Auth => "auth", Category::DataExposure => "data-exposure", Category::Injection => "injection",
        Category::Transport => "transport", Category::Config => "config", Category::Info => "info",
    }
}

fn slug(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '-' }).collect()
}

/// HTML 转义（响应体里可能带 <script>，绝不能原样进 DOM）。
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ── 渲染 ───────────────────────────────────────────────────────────────

fn render_global(r: &AnalyzedReport) -> String {
    let (g, risk) = grade(r);
    let total = r.findings.len();
    let confirmed = r.findings.iter().filter(|f| f.confirmed).count();

    // 严重度 tiles + 环形分布
    let sevs = [Severity::Critical, Severity::High, Severity::Medium, Severity::Low, Severity::Info];
    let tiles: String = sevs.iter().map(|s| {
        let n = count(r, *s);
        let z = if n == 0 { " z" } else { "" };
        format!("<div class=\"tile {}{}\"><div class=\"num\">{}</div><div class=\"tl\">{}</div></div>",
            sev_class(*s), z, n, sev_label(*s))
    }).collect();

    // 环形（conic-gradient 分段）；无发现时画一整段绿
    let donut_bg = donut_gradient(r);

    // tldr
    let tldr = if total == 0 {
        format!("本次在 <b>{}</b> 档下未发现可确认的安全问题。{}", esc(&r.mode), esc(&r.summary))
    } else {
        format!("共 {total} 项发现（{confirmed} 已确认）。{}", esc(&r.summary))
    };

    // 发现清单
    let rows: String = if total == 0 {
        "<tr><td colspan=\"4\" class=\"kk\">无发现。</td></tr>".to_string()
    } else {
        r.findings.iter().map(|f| {
            let link = if f.confirmed {
                format!("<a href=\"vuln-{}.html\">详情 →</a>", slug(&f.id))
            } else { "<span class=\"kk\">疑似·待查</span>".to_string() };
            format!("<tr><td><span class=\"sev {}\">{}</span></td><td>{}</td><td class=\"cvss\">{}</td><td>{}</td></tr>",
                sev_class(f.severity), sev_label(f.severity), esc(&f.title), cat_label(f.category), link)
        }).collect()
    };

    format!(r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>安全评估报告 · {target_short}</title>
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🛡️</text></svg>">
<style>{css}</style></head><body>
<div class="top"><div class="bar"><img src="{logo}" alt="Toolkit Studio">
<div class="who">Toolkit Studio<small>tke security · 安全评估</small></div>
<div class="rt">SECURITY ASSESSMENT</div></div></div>
<div class="wrap">
<div class="hero"><div class="donut"><div class="ring" style="background:{donut}"></div>
<div class="mid"><div class="dn">{g}</div><div class="dl">risk</div></div></div>
<div class="heroR"><div class="title">{target}</div>
<div class="rr"><span class="grade">{g}</span><span class="rt">总体风险 <b>{risk}</b> · {total} 项发现</span></div>
<div class="tiles">{tiles}</div></div></div>
<div class="runmeta"><span class="chip mode">强度 · <b>{mode}</b></span>
<span class="chip">发现 · <b>{total}</b></span><span class="chip">已确认 · <b>{confirmed}</b></span></div>
<div class="tldr"><b>一句话：</b>{tldr}</div>
<h2>发现清单</h2><div class="card"><div class="scroll"><table class="find">
<thead><tr><th>严重度</th><th>标题</th><th>类别</th><th>详情</th></tr></thead>
<tbody>{rows}</tbody></table></div></div>
<div class="note">本报告由 tke security 在 {mode} 档下对目标做只读/受控探测生成，凭据已脱敏。
已确认发现见各自的 vuln-*.html；疑似项需进一步验证。findings.json 为机器可读版本。</div>
<div class="foot"><img src="{logo}" alt="">Generated by Toolkit Studio · tke security</div>
</div></body></html>"#,
        target_short = esc(&host_of(&r.target)), target = esc(&r.target), css = CSS, logo = LOGO,
        donut = donut_bg, g = g, risk = risk, total = total, confirmed = confirmed,
        mode = esc(&r.mode), tiles = tiles, tldr = tldr, rows = rows,
    )
}

fn render_vuln(r: &AnalyzedReport, f: &Finding) -> String {
    let repro = f.repro.as_deref().map(|c| format!(
        "<div class=\"kk\" style=\"font-size:12px\">复现：</div><pre><code>{}</code></pre>", esc(c)
    )).unwrap_or_default();
    let evi: String = if f.evidence.is_empty() {
        String::new()
    } else {
        let items: String = f.evidence.iter().map(|e| format!(
            "<code>{}</code> · <code>{}</code>", esc(&e.request.to_string_lossy()), esc(&e.response.to_string_lossy())
        )).collect::<Vec<_>>().join("<br>");
        format!("<div class=\"evi\">EVIDENCE<br>{items}</div>")
    };
    format!(r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><title>{title} · {host}</title>
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🛡️</text></svg>">
<style>{css}</style></head><body>
<div class="top"><div class="bar"><img src="{logo}" alt=""><div class="who">Toolkit Studio<small>tke security · 漏洞报告</small></div></div></div>
<div class="wrap"><div class="vuln"><div class="vh"><div class="row">
<span class="sev {sc}">{sl}</span><span class="cvss">{cat}</span>
<span class="softtag">已确认</span></div>
<h3>{title}</h3><div class="cvss" style="margin-top:6px">{target}</div></div>
<div class="vb"><div class="tldr" style="margin:12px 0"><b>一句话：</b>{detail}</div>
{repro}{evi}</div></div>
<div class="foot"><img src="{logo}" alt="">Generated by Toolkit Studio · tke security</div>
</div></body></html>"#,
        title = esc(&f.title), host = esc(&host_of(&r.target)), css = CSS, logo = LOGO,
        sc = sev_class(f.severity), sl = sev_label(f.severity), cat = cat_label(f.category),
        target = esc(&r.target), detail = esc(&f.detail), repro = repro, evi = evi,
    )
}

/// 严重度环形的 conic-gradient；无发现 → 一整圈绿。
fn donut_gradient(r: &AnalyzedReport) -> String {
    let total = r.findings.len();
    if total == 0 {
        return "conic-gradient(var(--green) 0 360deg)".to_string();
    }
    let sevs = [
        (Severity::Critical, "var(--crit)"), (Severity::High, "var(--high)"),
        (Severity::Medium, "var(--med)"), (Severity::Low, "var(--low)"), (Severity::Info, "var(--info)"),
    ];
    let mut acc = 0.0f64;
    let mut segs = Vec::new();
    for (s, color) in sevs {
        let n = count(r, s) as f64;
        if n == 0.0 { continue; }
        let start = acc / total as f64 * 360.0;
        acc += n;
        let end = acc / total as f64 * 360.0;
        segs.push(format!("{color} {start:.1}deg {end:.1}deg"));
    }
    format!("conic-gradient({})", segs.join(", "))
}

fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest).to_string()
}

const CSS: &str = r#"
:root{--bg:#1a1c22;--bg2:#1f2229;--card:#24262c;--card2:#2b2f37;--line:#31353e;--line2:#3b404b;
--tx:#e7e9ee;--mut:#9aa4b2;--dim:#697080;--crit:#e5565f;--high:#f0883e;--med:#ebc24e;--low:#5fa8e0;--info:#828b9a;
--green:#3fbf87;--brand:#f0883e;--code:#15171c;--mono:ui-monospace,Menlo,Consolas,monospace}
@media (prefers-color-scheme:light){:root{--bg:#f2f4f8;--bg2:#eaeef4;--card:#fff;--card2:#f5f7fb;--line:#e2e7f0;--line2:#d3dae6;
--tx:#1a2230;--mut:#5a6675;--dim:#8b95a5;--crit:#d92d3c;--high:#d9721e;--med:#c79216;--low:#1f74c4;--info:#6b7686;
--green:#0f9d64;--brand:#d9721e;--code:#f0f3f8}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--tx);
font:15px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI","PingFang SC","Microsoft YaHei",sans-serif;word-break:break-word}
.wrap{max-width:820px;margin:0 auto;padding:0 18px 48px}
code{background:var(--code);padding:1px 5px;border-radius:4px;font-size:12.5px;font-family:var(--mono)}
a{color:var(--low);text-decoration:none}a:hover{text-decoration:underline}
.top{background:var(--bg2);border-bottom:1px solid var(--line);padding:14px 18px;margin-bottom:22px}
.top .bar{max-width:820px;margin:0 auto;display:flex;align-items:center;gap:11px}
.top img{width:34px;height:34px;border-radius:8px}.top .who{font-weight:700;font-size:14px;line-height:1.25}
.top .who small{display:block;color:var(--mut);font-weight:500;font-size:11px}
.top .rt{margin-left:auto;color:var(--dim);font-size:11px;font-family:var(--mono)}
.hero{display:grid;grid-template-columns:auto 1fr;gap:24px;align-items:center;background:var(--card);
border:1px solid var(--line);border-radius:14px;padding:20px 22px;margin-bottom:14px}
.donut{position:relative;width:120px;height:120px}
.donut .ring{width:120px;height:120px;border-radius:50%;-webkit-mask:radial-gradient(farthest-side,transparent 60%,#000 61%);mask:radial-gradient(farthest-side,transparent 60%,#000 61%)}
.donut .mid{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center}
.donut .dn{font-size:34px;font-weight:800}.donut .dl{font-size:10px;color:var(--mut);text-transform:uppercase}
.heroR .title{font-size:19px;font-weight:700;margin-bottom:6px;font-family:var(--mono)}
.rr{display:flex;align-items:center;gap:9px;margin-bottom:14px}
.grade{display:inline-flex;align-items:center;justify-content:center;min-width:28px;height:26px;padding:0 8px;
border-radius:7px;font-weight:800;background:var(--med);color:#20180a}.rr .rt{color:var(--mut);font-size:13px}.rr .rt b{color:var(--med)}
.tiles{display:grid;grid-template-columns:repeat(5,1fr);gap:7px}
.tile{background:var(--card2);border:1px solid var(--line);border-radius:9px;padding:8px 6px;text-align:center;border-top:2px solid var(--line)}
.tile.c{border-top-color:var(--crit)}.tile.h{border-top-color:var(--high)}.tile.m{border-top-color:var(--med)}
.tile.l{border-top-color:var(--low)}.tile.i{border-top-color:var(--info)}
.tile .num{font-size:20px;font-weight:800;font-family:var(--mono)}
.tile.c .num{color:var(--crit)}.tile.h .num{color:var(--high)}.tile.m .num{color:var(--med)}.tile.l .num{color:var(--low)}.tile.i .num{color:var(--info)}
.tile.z .num{color:var(--dim)}.tile .tl{font-size:10px;color:var(--mut);margin-top:4px}
.runmeta{display:flex;flex-wrap:wrap;gap:7px;margin-bottom:20px}
.chip{font-size:11.5px;color:var(--mut);background:var(--card);border:1px solid var(--line);border-radius:7px;padding:5px 10px}
.chip b{color:var(--tx)}.chip.mode{color:var(--brand);border-color:var(--brand)}
h2{font-size:12.5px;margin:26px 0 12px;text-transform:uppercase;letter-spacing:1px;color:var(--mut)}
.card{background:var(--card);border:1px solid var(--line);border-radius:13px;padding:4px 18px;margin-bottom:11px}
.tldr{background:var(--card);border:1px solid var(--line);border-left:3px solid var(--med);border-radius:13px;padding:15px 18px}
.tldr b{color:var(--med)}
.scroll{overflow-x:auto}table.find{width:100%;border-collapse:collapse;font-size:13.5px;min-width:480px}
table.find th{color:var(--dim);font-weight:600;font-size:11px;text-transform:uppercase;text-align:left;padding:9px 8px;border-bottom:1px solid var(--line2);white-space:nowrap}
table.find td{padding:12px 8px;border-bottom:1px solid var(--line);white-space:nowrap}
.sev{display:inline-block;font-size:11px;font-weight:700;padding:2px 9px;border-radius:20px}
.sev.c{color:var(--crit);background:color-mix(in srgb,var(--crit) 16%,transparent)}
.sev.h{color:var(--high);background:color-mix(in srgb,var(--high) 16%,transparent)}
.sev.m{color:var(--med);background:color-mix(in srgb,var(--med) 16%,transparent)}
.sev.l{color:var(--low);background:color-mix(in srgb,var(--low) 16%,transparent)}
.sev.i{color:var(--info);background:color-mix(in srgb,var(--info) 16%,transparent)}
.cvss{font-family:var(--mono);font-size:12.5px;color:var(--mut)}.kk{color:var(--dim)}
.vuln{border:1px solid var(--line2);border-radius:14px;overflow:hidden;margin:6px 0}
.vuln .vh{background:var(--card2);padding:16px 18px;border-left:3px solid var(--med)}
.vuln .vh .row{display:flex;align-items:center;gap:9px;flex-wrap:wrap;margin-bottom:8px}
.vuln .vh h3{margin:0;font-size:16.5px;font-weight:700}.vuln .vb{padding:6px 18px 18px}
.softtag{font-size:10px;font-weight:700;color:var(--green);background:color-mix(in srgb,var(--green) 16%,transparent);border-radius:20px;padding:2px 8px}
pre{background:var(--code);border:1px solid var(--line2);border-radius:10px;padding:14px;overflow-x:auto;font-size:12px;font-family:var(--mono)}
pre code{background:none;padding:0}
.evi{color:var(--dim);font-size:12px;border-top:1px dashed var(--line2);margin-top:15px;padding-top:13px;font-family:var(--mono)}
.note{color:var(--dim);font-size:12.5px;border-top:1px dashed var(--line2);margin-top:26px;padding-top:15px}
.foot{display:flex;align-items:center;gap:9px;color:var(--dim);font-size:11.5px;margin-top:20px;padding-top:15px;border-top:1px solid var(--line)}
.foot img{width:17px;height:17px;border-radius:4px;opacity:.9}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::evidence::EvidenceRef;

    fn analyzed(findings: Vec<Finding>) -> AnalyzedReport {
        AnalyzedReport { target: "https://t.example/".into(), mode: "safe".into(),
            findings, summary: "测试".into(), dropped: 0 }
    }

    #[test]
    fn clean_report_grade_a_no_vuln_files() {
        let tmp = std::env::temp_dir().join(format!("tke-rep-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let paths = write_reports(&tmp, &analyzed(vec![])).unwrap();
        let html = std::fs::read_to_string(&paths.html).unwrap();
        assert!(html.contains(">A<"), "无发现应评 A");
        assert!(html.contains("未发现可确认"));
        assert!(paths.vulns.is_empty(), "无确认发现不该产 vuln 文件");
        let json = std::fs::read_to_string(&paths.json).unwrap();
        assert!(json.contains("\"outcome\": \"clean\""));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn confirmed_finding_gets_vuln_file_and_escapes() {
        let mut f = Finding::new("open-set", Severity::High, Category::DataExposure,
            "泄露 <script>", "任何人可读");
        f.confirmed = true;
        f.repro = Some("curl https://t.example/api".into());
        f.evidence.push(EvidenceRef { seq: 3, request: "evidence/step_003_req.txt".into(), response: "evidence/step_003_resp.txt".into() });

        let tmp = std::env::temp_dir().join(format!("tke-rep-vuln-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let paths = write_reports(&tmp, &analyzed(vec![f])).unwrap();
        assert_eq!(paths.vulns.len(), 1);
        let vuln = std::fs::read_to_string(&paths.vulns[0]).unwrap();
        assert!(vuln.contains("&lt;script&gt;"), "标题必须转义，防注入");
        assert!(vuln.contains("curl https://t.example/api"));
        assert!(vuln.contains("step_003"));
        let global = std::fs::read_to_string(&paths.html).unwrap();
        assert!(global.contains(">D<"), "有 high 应评 D");
        assert!(global.contains("vuln-open-set.html"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
