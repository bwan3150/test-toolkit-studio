// 【生成后自检 + 自修复】(--verify) —— 脚本医生(编辑器 agent)的入口 + 验证基础设施
//
// harness 探索产出 .tks 后，验证它能否被 `tke run` 稳定回放、并把它修到「稳定到达目标」：
//   ① 目标标志：问探索会话要一段"只有真到达目标才出现的独特文字"(marker)，用于自动校验是否真达成；
//   ② 脚本医生(doctor.rs)：诊断回放(每步留页面)→ 医生编辑脚本任意行(删/改/插 + 活体重探)→ 重跑 →
//      直到稳定到达目标并提炼最短(详见 doctor.rs)；
//   ③ 稳定性：连续 need_pass 次(默认 2)重启净化后回放都到达目标才算稳定，期间不稳就再请医生修。
//
// 全景记录（conversation.json）：除 AI 交互外还记 verify_start / doctor_* / verify_end，可复盘
// 一开始脚本哪里错、医生怎么改、最终结论。
//
// 本文件保留「回放 / 目标校验」基础设施(goal_probe / do_replay / reset_state / strip_trailing_close /
// page_contains / launch_spec)，供 doctor.rs 复用；真正的「编辑→跑→看→改」主循环在 doctor.rs。

use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use crate::{AiConfig, ControlAction, ExecutionResult, LlmReply, LlmSession, Params, Result, RunEvent, ScriptParser, ScriptRunner, TksCommand, TksParam};

use super::super::execution::device::exec;
use super::super::execution::script::write_script;
use super::super::perception::Perceived;
use super::super::transcript::Transcript;
use super::super::prompt::{render, PromptSet};
use super::doctor;
use super::flow::{brief, friendly, paint, parse_desc_json, DriveCtx};
use super::options::VerifyReport;

// 稳定性通过次数 / 修复(活体重探)上限均来自 config [harness]（params.harness），不再写死。

/// 自检并尝试修复，返回（最终脚本步骤, 自检报告）。会把最终版本写回 script_path。
/// `ai`：用于新建独立的「脚本医生」会话（与探索会话分开，独立工具集）。
/// `verify_log`：保留参数（诊断回放产物落点由 doctor 内部统一管理，此处不再单独用）。
#[allow(clippy::too_many_arguments)]
pub async fn verify_and_repair(
    sess: &mut LlmSession,
    ai: &AiConfig,
    prompts: &PromptSet,
    reflection: Option<&str>,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    verify_log: Option<&Path>,
    lines: Vec<String>,
) -> (Vec<String>, VerifyReport) {
    let tty = std::io::stderr().is_terminal();
    let mut report = VerifyReport { ran: true, ..Default::default() };
    // 次数上限来自 config [harness]（缺省 stability=2 / repairs=6）
    let need_pass = params.harness.stability;
    let max_repairs = params.harness.repairs;

    if lines.is_empty() {
        eprintln!("{}", paint(tty, "33", "自检跳过：没有生成可回放的步骤"));
        return (lines, report);
    }
    let _ = verify_log; // 诊断回放会反复进行，产物落点由 doctor 内部管理

    // 目标标志：问探索会话(它见过目标页)要一段"只有真到达目标才出现的独特文字"
    let marker = ask_goal_marker(prompts, sess, tx, &lines, case).await.unwrap_or_default();

    tx.log(
        "verify_start",
        serde_json::json!({ "need_pass": need_pass, "max_repairs": max_repairs, "marker": marker, "script_lines": lines.clone() }),
    );
    // ============ 诊断优化 ============
    // 脚本医生把脚本修到「能跑通并到达目标」并提炼最短（删冗余/改坏自动还原，详见 doctor.rs）。
    eprintln!();
    eprintln!("{}", paint(tty, "1", "╭─ 诊断优化（脚本医生：编辑→跑→看→改，修到稳定回放到目标并提炼最短）─"));
    let fixed = doctor::doctor_repair(ai, prompts, reflection, tx, ctx, params, script_path, case, &marker, lines, &mut report).await;
    let mut lines = match fixed {
        Some(l) => {
            report.reached = true;
            report.final_steps = l.len();
            let opt = if report.hit_iter_limit {
                paint(tty, "33", &format!("■ 优化达上限（仍可跑、未必最短）· 修复 {} 次 · 最终 {} 步", report.repairs, l.len()))
            } else {
                paint(tty, "32", &format!("✓ 优化完成 · 修复 {} 次 · 最终 {} 步", report.repairs, l.len()))
            };
            eprintln!("  {}   {}", paint(tty, "2", "诊断"), opt);
            eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
            l
        }
        None => {
            report.reached = false;
            tx.log("verify_end", serde_json::json!({ "passed": false, "reached": false, "repairs": report.repairs }));
            eprintln!("  {}   {}", paint(tty, "2", "诊断"), paint(tty, "31", "✗ 修复失败（脚本仍跑不到目标，已保留当前最好版本）"));
            eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
            return (Vec::new(), report);
        }
    };

    // ============ 稳定性测试 ============
    // 连续 need_pass 次重启净化后回放都到达目标才算稳定；中途不稳就再请医生修一轮。
    eprintln!();
    eprintln!("{}", paint(tty, "1", &format!("╭─ 稳定性测试（连续 {} 次重启净化后回放都须到达目标）─", need_pass)));
    let mut streak = 0usize;
    let mut replay_no = 0usize;
    loop {
        if super::interrupt::aborted() {
            eprintln!("  {}", paint(tty, "33", "已中断（Ctrl+C），停止稳定性测试"));
            break;
        }
        replay_no += 1;
        eprintln!();
        eprintln!("  {}", paint(tty, "1;36", &format!("▶ 第 {} 次稳定回放（重启净化中…）", replay_no)));
        // 用 diagnose 跑（与诊断同格式、同样把逐步成败/截图/页面结构记进 conversation.json）
        let reached = doctor::diagnose(tx, ctx, params, script_path, case, &lines, &marker, "verify_stability", replay_no, true).await.reached;
        if reached {
            streak += 1;
            report.stability_passes = streak;
            eprintln!("  {} 到达目标（连续 {}/{}）", paint(tty, "32", "✓"), streak, need_pass);
            if streak >= need_pass {
                report.passed = true;
                break;
            }
            continue;
        }
        streak = 0;
        report.stability_passes = 0;
        if report.repairs >= max_repairs {
            eprintln!("  {}", paint(tty, "33", &format!("已达修复上限 {} 次，停止", max_repairs)));
            break;
        }
        // 不稳 → 再请医生修一轮（克隆传入：医生放弃时仍保留当前最好版本供落盘）
        match doctor::doctor_repair(ai, prompts, reflection, tx, ctx, params, script_path, case, &marker, lines.clone(), &mut report).await {
            Some(l) => lines = l,
            None => break,
        }
    }

    // 落盘最终版本（含结尾关闭）
    report.final_steps = lines.len();
    let _ = write_script(script_path, case, &lines);
    tx.log(
        "verify_end",
        serde_json::json!({
            "passed": report.passed,
            "reached": report.reached,
            "repairs": report.repairs,
            "final_steps": report.final_steps,
            "stability_passes": report.stability_passes,
            "final_script": lines.clone(),
        }),
    );

    let summary = if report.passed {
        paint(tty, "32", &format!("✓ 验证通过（连续 {} 次到达目标）", report.stability_passes))
    } else {
        paint(tty, "33", &format!("✗ 验证未通过（连续到达 {}/{} 次，已保留当前最好版本）", report.stability_passes, need_pass))
    };
    eprintln!("  {}   {}", paint(tty, "2", "验证"), summary);
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    (lines, report)
}

/// 问探索会话要一段「目标标志」文本：只有真正到达目标时最终页面才会出现的独特文字。
async fn ask_goal_marker(prompts: &PromptSet, sess: &mut LlmSession, tx: &mut Transcript, lines: &[String], case: &str) -> Option<String> {
    let listing = lines
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, friendly(l)))
        .collect::<Vec<_>>()
        .join("\n");
    let ask = render(&prompts.message("verify", "goal_marker"), &[("listing", &listing), ("case", case)]);
    tx.log("llm_message", serde_json::json!({ "content": ask.clone() }));
    sess.user(ask);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    tx.log("goal_marker", serde_json::json!({ "content": reply.clone() }));
    let obj = parse_desc_json(&reply)?;
    Some(obj["goal_marker"].as_str().unwrap_or("").trim().to_string())
}

/// 回放一遍：verbose=true 逐步流式打印（像 tke run）。
pub(super) async fn do_replay(params: &Arc<Params>, script_path: &Path, verbose: bool) -> Result<ExecutionResult> {
    let tty = std::io::stderr().is_terminal();
    let mut total = 0usize;
    let mut sink = |e: &RunEvent| {
        if !verbose {
            return;
        }
        match e {
            RunEvent::RunStart { total_steps, .. } => total = *total_steps,
            RunEvent::StepEnd { index, command, success, error, .. } => {
                if *success {
                    eprintln!("    {}  {}  {}", paint(tty, "32", "✓"), paint(tty, "2", &format!("步 {}/{}", index + 1, total)), friendly(command));
                } else {
                    eprintln!(
                        "    {}  {}",
                        paint(tty, "31", &format!("✗ 步 {}/{}  {}", index + 1, total, friendly(command))),
                        paint(tty, "31", &format!("— {}", brief(error.as_deref().unwrap_or(""), 80))),
                    );
                }
            }
            _ => {}
        }
    };
    ScriptRunner::new(params.clone()).run(script_path, None, &mut sink).await
}

/// 去掉结尾连续的"关闭"步
pub(super) fn strip_trailing_close(lines: &[String]) -> Vec<String> {
    let mut v = lines.to_vec();
    while v.last().map(|l| l.trim_start().starts_with("关闭")).unwrap_or(false) {
        v.pop();
    }
    v
}

/// 当前页面（含 OCR 伪元素）的文本里是否包含目标标志（不分大小写）
pub(super) fn page_contains(p: &Perceived, marker: &str) -> bool {
    let m = marker.to_lowercase();
    p.elements.iter().any(|e| e.to_ai_text().to_lowercase().contains(&m))
}

/// 重启净化：关掉目标后**重新启动**，把状态刷新到干净初始态，再开始回放。
/// web=销毁会话后重开并导航；移动=force-stop 后重新拉起。
/// 这样脚本里即便没有「启动」步也没关系——这里已经重启过了；脚本若自带「启动」，
/// 那只是再导航/拉起一次（幂等，仍是干净态）。仅当连启动目标都解析不到时才跳过。
pub(super) async fn reset_state(device: &str, lines: &[String]) {
    let Some((target, activity)) = launch_spec(lines) else { return };
    // 1) 关闭：清掉旧会话/进程
    let _ = exec(device, ControlAction::Close { package: target.clone() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // 2) 重新启动：刷新到干净初始状态
    let _ = exec(device, ControlAction::Launch { package: target, activity }).await;
    // 给启动 / 页面加载留时间，再开始回放
    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
}

/// 解析脚本首个「启动」步骤的 (目标, activity)。目标=包名/URL；activity 缺省空串。
fn launch_spec(lines: &[String]) -> Option<(String, String)> {
    let content = format!("步骤:\n{}", lines.join("\n"));
    let script = ScriptParser::new().parse(&content).ok()?;
    for step in &script.steps {
        if step.command == TksCommand::Launch {
            let texts: Vec<String> = step
                .params
                .iter()
                .filter_map(|p| match p {
                    TksParam::Text(t) => Some(t.clone()),
                    _ => None,
                })
                .collect();
            if let Some(target) = texts.first().cloned() {
                return Some((target, texts.get(1).cloned().unwrap_or_default()));
            }
        }
    }
    None
}

