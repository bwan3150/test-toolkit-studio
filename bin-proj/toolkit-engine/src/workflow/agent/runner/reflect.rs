// 【探索反思官】两类职责：
//  ① 探索失败 → `reflect()` 出「重探指导」，上层带指导重探（治"绕大圈没找到"，探索阶段安全网）；
//  ② 脚本已正确 → `optimize()` 做**软优化**：看正确脚本的 trace，指出可删的绕路/冗余步并删掉
//     （大胆删、不逐条验证；删坏了由医生在下一轮复检兜底——医生⇄反思官交替收敛）。
// 独立会话(无工具)，按作用域归属 "reflector"；token 单独计、并入总量。

use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use crate::{AiConfig, Fetcher, LlmReply, LlmSession, Params, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::flow::{brief, friendly, paint, parse_desc_json, DriveCtx, DriveOutcome};
use super::options::VerifyReport;

/// 反思产物
pub(super) struct Reflection {
    /// 报告文本（重探指导 / 绕路报告）
    pub report: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

/// 把探索全程拼成 journey 文本：步号 · 动作 · 当时的理由 · 该步所在页面(结构元素前几项)。
/// 页面取探索阶段已落盘的每步 xml（离线解析，不重放、不跑 OCR，求快）。
fn build_journey(outcome: &DriveOutcome, fetcher: &Fetcher, run_dir: &Path) -> String {
    let mut s = String::new();
    for (i, line) in outcome.lines.iter().enumerate() {
        let why = outcome
            .step_comments
            .get(i)
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .unwrap_or("（无）");
        let page = outcome
            .steps
            .get(i)
            .and_then(|st| st.xml.as_ref())
            .map(|rel| run_dir.join(rel))
            .and_then(|p| fetcher.fetch_elements_from_file(&p).ok())
            .map(|els| {
                els.iter()
                    .take(6)
                    .map(|e| e.to_ai_text())
                    .filter(|t| !t.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        s.push_str(&format!("{}. {}　理由：{}", i + 1, friendly(line), brief(why, 80)));
        if !page.trim().is_empty() {
            s.push_str(&format!("　页面：{}", brief(&page, 120)));
        }
        s.push('\n');
    }
    s
}

/// 复盘一轮探索。success=true 用 analyze_success(给 doctor 的绕路报告)，否则 analyze_failure(重探指导)。
/// 返回 None 表示无产物（空脚本 / 会话创建失败 / LLM 没给文本）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn reflect(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    device: &str,
    fetcher: &Fetcher,
    run_dir: &Path,
    case: &str,
    outcome: &DriveOutcome,
    success: bool,
) -> Option<Reflection> {
    if outcome.lines.is_empty() {
        return None;
    }
    let platform = Platform::from_device(Some(device));
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;

    let system = prompts.role_system("reflector", device, platform.name());
    let mut sess = LlmSession::new(ai, system.clone(), Vec::new()).ok()?;
    tx.log("reflector_session", serde_json::json!({ "system_prompt": system }));

    let journey = build_journey(outcome, fetcher, run_dir);
    let template = if success { "analyze_success" } else { "analyze_failure" };
    let prompt = render(&prompts.message("reflector", template), &[("case", case), ("journey", &journey)]);
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    sess.user(prompt);

    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    let (prompt_tokens, completion_tokens) = sess.total_usage();
    tx.log(
        "reflector_report",
        serde_json::json!({ "mode": if success { "success" } else { "failure" }, "content": reply.clone() }),
    );
    Some(Reflection { report: reply, prompt_tokens, completion_tokens })
}

/// 软优化：脚本已正确时，看其逐步 trace 找出可删的绕路/冗余步并删掉（大胆删、不逐条验证；
/// 删坏了由医生下一轮复检兜底）。返回优化后的脚本（有改动）或 None（无可优化/失败）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn optimize(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    marker: &str,
    lines: &[String],
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    let tty = std::io::stderr().is_terminal();
    let mut scope = tx.scoped("reflector");
    let tx = &mut *scope;

    // 诊断当前脚本拿逐步 trace（调用前应已正确；不正确则不优化）
    let diag = super::doctor::diagnose(tx, ctx, params, script_path, case, lines, marker, "reflect_diagnose", 0, false).await;
    if !diag.reached {
        return None;
    }
    let trace = diag.trace_lines();
    let pages = diag.page_groups(); // 页面访问/重复分析：同一页反复=绕路，助判冗余
    let numbered: String = lines.iter().enumerate().map(|(i, l)| format!("{}. {}", i + 1, friendly(l))).collect::<Vec<_>>().join("\n");

    let platform = Platform::from_device(Some(ctx.device));
    let system = prompts.role_system("reflector", ctx.device, platform.name());
    let mut sess = LlmSession::new(ai, system, Vec::new()).ok()?;
    let prompt = render(
        &prompts.message("reflector", "optimize"),
        &[("case", case), ("marker", marker), ("trace", &trace), ("pages", &pages), ("script", &numbered)],
    );
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    sess.user(prompt);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    let (pt, ct) = sess.total_usage();
    report.extra_prompt += pt;
    report.extra_completion += ct;
    tx.log("reflector_optimize", serde_json::json!({ "content": reply.clone() }));

    // 解析 {"removable":[步号,...]}
    let obj = parse_desc_json(&reply)?;
    let mut removable: Vec<usize> = obj["removable"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).map(|n| n as usize).collect())
        .unwrap_or_default();
    removable.retain(|&i| i >= 1 && i <= lines.len());
    removable.sort_unstable_by(|a, b| b.cmp(a)); // 从大到小删，不影响更小编号
    removable.dedup();
    if removable.is_empty() {
        eprintln!("  {}", paint(tty, "2", "反思官：当前路径已无可删冗余"));
        return None;
    }

    let mut new = lines.to_vec();
    for idx in &removable {
        new.remove(idx - 1);
    }
    if new == lines {
        return None;
    }
    let mut nums: Vec<usize> = removable.iter().copied().collect();
    nums.sort_unstable();
    eprintln!(
        "  {}  删第 {} 步（{} 步 → {} 步），交医生复检",
        paint(tty, "1;36", "◇ 反思官优化路径"),
        nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("/"),
        lines.len(),
        new.len()
    );
    Some(new)
}
