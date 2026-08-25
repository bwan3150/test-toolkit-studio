//! 生成一份 tke security 报告样例（确定性 reporter，无需 LLM）。
//! 用法：cargo run --example security_report_sample --no-default-features -- <输出目录>
//! 造几条不同严重度的发现，渲染 security-report.html + findings.json + vuln-*.html。

use tke::workflow::security::analyst::AnalyzedReport;
use tke::workflow::security::evidence::EvidenceRef;
use tke::workflow::security::finding::{Category, Finding, Severity};
use tke::workflow::security::report::write_reports;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "/tmp/tke-sec-sample".to_string());

    let mut f1 = Finding::new(
        "login-no-ratelimit", Severity::Medium, Category::Auth,
        "登录接口无速率限制，可账户爆破", "登录端点对失败尝试不限流不锁定，配合可枚举用户名可离线爆破。",
    );
    f1.confirmed = true;
    f1.repro = Some("for i in $(seq 100); do\n  curl -s -o /dev/null -w \"%{http_code} \" \\\n    -X POST https://shop.example/api/login -d \"u=demo&p=wrong-$i\"\ndone\n# 实测 100 次全 401，无一次 429".into());
    f1.evidence.push(EvidenceRef { seq: 12, request: "evidence/step_012_req.txt".into(), response: "evidence/step_012_resp.txt".into() });

    let mut f2 = Finding::new(
        "missing-headers", Severity::Low, Category::Config,
        "缺少 CSP / X-Frame-Options", "无内容安全策略与点击劫持防护。",
    );
    f2.confirmed = true;
    f2.evidence.push(EvidenceRef { seq: 2, request: "evidence/step_002_req.txt".into(), response: "evidence/step_002_resp.txt".into() });

    let mut f3 = Finding::new(
        "suspected-idor", Severity::High, Category::Auth,
        "疑似越权（IDOR，待验证）", "改 id 似乎能读到他人订单，但样本不足以坐实。",
    );
    f3.confirmed = false; // 疑似 → 只进全局清单，不出单独报告

    let report = AnalyzedReport {
        usage: Default::default(),
        target: "https://shop.acme-demo.example/".into(),
        mode: "safe".into(),
        findings: vec![f1, f2, f3],
        summary: "2 项已确认（1 中危 1 低危）、1 项疑似越权待验证。".into(),
        dropped: 1,
    };

    let paths = write_reports(std::path::Path::new(&dir), &report).expect("写报告失败");
    println!("HTML : {}", paths.html.display());
    println!("JSON : {}", paths.json.display());
    for v in &paths.vulns {
        println!("VULN : {}", v.display());
    }
}
