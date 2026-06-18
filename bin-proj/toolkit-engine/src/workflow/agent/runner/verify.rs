// 【生成后自检 + 自修复】(--verify) —— 同时是验证阶段的全景记录器
//
// harness 探索产出 .tks 后，验证它能否被 `tke run` 稳定回放：
//   1) 重启净化：关闭并重新启动目标（web 销毁会话后重开/移动 force-stop 后拉起），刷新到干净初始态
//   2) 整脚本从头回放一次（复用 ScriptRunner，即 tke run 的执行路径，**零 token**）
//   3) 失败：前 k 步可信保留，把失败步+错误+当前实时页面交回同一 AI 会话，
//      让它从失败处「续接修复」(drive spinner=false)，产出修正尾部并拼接
//   4) 成功：连续通过 NEED_PASS 次（默认 2）才算稳定，期间每次都重启净化以验证可重复性
//
// 全景记录（conversation.json）：除 AI 交互外，验证阶段还记下——
//   verify_start（初始脚本）/ verify_replay（每次回放的逐步执行轨迹+哪步失败，零 token）/
//   verify_repair（AI 介入修复的交接：保留前缀、从哪步起重导航）/ verify_end（最终结论）。
//   这样能复盘：一开始脚本哪里错了（攒经验）、AI 的介入与修正是否正确。
//
// 状态延续依据：web 会话按设备 ID 持久化（与 workarea 无关）、移动端 device 本身即状态，
// 因此回放失败后设备就停在失败页，AI 直接 perceive 即可，无需重放前缀。

use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use crate::{ControlAction, ExecutionResult, LlmSession, Params, Result, RunEvent, ScriptParser, ScriptRunner, StepResult, TksCommand, TksParam};

use super::super::execution::device::exec;
use super::super::execution::script::write_script;
use super::super::transcript::Transcript;
use super::flow::{brief, drive, friendly, paint, DriveCtx};
use super::options::VerifyReport;

const NEED_PASS: usize = 2; // 连续干净回放通过几次算「稳定」
const MAX_REPAIRS: usize = 6; // 修复尝试上限（兜底，避免反复修不好时无限循环）

/// 自检并尝试修复，返回（最终脚本步骤, 自检报告）。会把最终版本写回 script_path。
/// `verify_log`：回放产物（标注截图/页面结构）落点；None 则只记文本轨迹、不存图。
pub async fn verify_and_repair(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    verify_log: Option<&Path>,
    mut lines: Vec<String>,
) -> (Vec<String>, VerifyReport) {
    let tty = std::io::stderr().is_terminal();
    let mut report = VerifyReport { ran: true, passed: false, repairs: 0, created: Vec::new(), updated: Vec::new() };

    if lines.is_empty() {
        eprintln!("{}", paint(tty, "33", "自检跳过：没有生成可回放的步骤"));
        return (lines, report);
    }

    // 全景记录：初始脚本
    tx.log(
        "verify_start",
        serde_json::json!({ "need_pass": NEED_PASS, "max_repairs": MAX_REPAIRS, "script_lines": lines.clone() }),
    );

    eprintln!("{}", paint(tty, "1", "╭─ 自检回放 ──────────────────────────"));

    let mut streak = 0usize; // 连续干净通过次数
    let mut replay_no = 0usize;
    loop {
        // 1) 重启净化
        reset_state(ctx.device, &lines).await;
        // 2) 写回当前版本并整脚本回放
        let _ = write_script(script_path, case, &lines);
        replay_no += 1;
        let result = replay(params, script_path, verify_log).await;

        match result {
            // —— 干净通过 ——
            Ok(r) if r.success => {
                streak += 1;
                tx.log(
                    "verify_replay",
                    serde_json::json!({
                        "replay": replay_no, "engine": "tke run", "tokens": 0,
                        "success": true, "streak": streak, "total_steps": r.steps.len(),
                        "steps": steps_json(&r.steps),
                    }),
                );
                eprintln!(
                    "  第 {} 次回放：{} 全部 {} 步通过（连续 {}/{}）",
                    replay_no,
                    paint(tty, "32", "✓"),
                    r.steps.len(),
                    streak,
                    NEED_PASS
                );
                if streak >= NEED_PASS {
                    report.passed = true;
                    break;
                }
                continue; // 再跑一遍确认稳定
            }
            // —— 回放执行到某步失败 ——
            Ok(r) => {
                streak = 0;
                let k = r
                    .steps
                    .iter()
                    .position(|s| !s.success)
                    .unwrap_or_else(|| r.steps.len().saturating_sub(1));
                let failed = lines.get(k).cloned().unwrap_or_default();
                let err = r.error.clone().unwrap_or_else(|| "未知错误".into());
                tx.log(
                    "verify_replay",
                    serde_json::json!({
                        "replay": replay_no, "engine": "tke run", "tokens": 0,
                        "success": false, "failed_step": k + 1, "failed_line": failed, "error": err,
                        "total_steps": r.steps.len(), "steps": steps_json(&r.steps),
                    }),
                );
                eprintln!(
                    "  第 {} 次回放：{} 第 {} 步失败 {} — {}",
                    replay_no,
                    paint(tty, "31", "✗"),
                    k + 1,
                    friendly(&failed),
                    brief(&err, 120)
                );

                if report.repairs >= MAX_REPAIRS {
                    eprintln!("  {}", paint(tty, "33", &format!("已达修复上限 {} 次，停止自检", MAX_REPAIRS)));
                    break;
                }
                report.repairs += 1;

                // 3) 失败续接：前 k 步可信保留，AI 从当前实时页面续接修复
                let prefix: Vec<String> = lines[..k].to_vec();
                // 全景记录：AI 介入修复的交接点
                tx.log(
                    "verify_repair",
                    serde_json::json!({
                        "repair": report.repairs, "from_step": k + 1, "failed_line": failed,
                        "error": err, "kept_prefix_steps": k,
                    }),
                );
                eprintln!(
                    "  {}",
                    paint(tty, "2", &format!("→ 让 AI 从第 {} 步修复（第 {} 次修复）…", k + 1, report.repairs))
                );
                let preamble = format!(
                    "现在进入【回放修复】。我把你刚生成的脚本从头跑了一遍，前 {} 步都成功了，\
                     但第 {} 步「{}」失败：{}。设备此刻就停在那一步该发生的页面上。\
                     请从当前页面出发重新找到正确做法、把剩下的测试目标走完，最后 finish。\
                     （只需产出从这一步往后的操作，前面成功的步骤会自动保留。）整体测试目标：{}",
                    k,
                    k + 1,
                    friendly(&failed),
                    err,
                    case
                );
                sess.user(preamble);
                let tail = match drive(sess, tx, ctx, false).await {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("  {}", paint(tty, "31", &format!("修复过程出错：{}", e)));
                        break;
                    }
                };
                // 累计修复阶段的元素变更，供最终「元素库更新」框合并展示
                for c in tail.created {
                    if !report.created.contains(&c) {
                        report.created.push(c);
                    }
                }
                for u in tail.updated {
                    if !report.updated.contains(&u) {
                        report.updated.push(u);
                    }
                }
                if tail.aborted {
                    eprintln!("  {}", paint(tty, "33", "已终止（用户中断）"));
                    break;
                }
                // 拼接：可信前缀 + 修复尾部
                lines = prefix;
                lines.extend(tail.lines);
                if lines.is_empty() {
                    eprintln!("  {}", paint(tty, "33", "修复未产出有效步骤，停止自检"));
                    break;
                }
            }
            // —— 回放根本起不来（解析/设备错误）——
            Err(e) => {
                tx.log(
                    "verify_replay",
                    serde_json::json!({ "replay": replay_no, "engine": "tke run", "tokens": 0, "success": false, "error": e.to_string() }),
                );
                eprintln!("  第 {} 次回放：{} 无法执行 — {}", replay_no, paint(tty, "31", "✗"), e);
                break;
            }
        }
    }

    // 落盘最终版本
    let _ = write_script(script_path, case, &lines);
    tx.log(
        "verify_end",
        serde_json::json!({ "passed": report.passed, "repairs": report.repairs, "replays": replay_no }),
    );

    let summary = if report.passed {
        paint(tty, "32", &format!("✓ 稳定通过（连续 {} 次干净回放，修复 {} 次）", NEED_PASS, report.repairs))
    } else {
        paint(tty, "33", &format!("■ 未达稳定（修复 {} 次后仍未连续通过，已保留当前最好版本）", report.repairs))
    };
    eprintln!("  {}   {}", paint(tty, "2", "结果"), summary);
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    (lines, report)
}

/// 把回放的逐步执行结果转成全景记录用的 JSON 数组（命令/成败/错误/截图，零 token）。
fn steps_json(steps: &[StepResult]) -> serde_json::Value {
    serde_json::Value::Array(
        steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step": s.index + 1,
                    "command": s.command,
                    "success": s.success,
                    "error": s.error,
                    "duration_ms": s.duration_ms,
                    "screenshot": s.screenshot,
                })
            })
            .collect(),
    )
}

/// 重启净化：关掉目标后**重新启动**，把状态刷新到干净初始态，再开始回放。
/// web=销毁会话后重开并导航；移动=force-stop 后重新拉起。
/// 这样脚本里即便没有「启动」步也没关系——这里已经重启过了；脚本若自带「启动」，
/// 那只是再导航/拉起一次（幂等，仍是干净态）。仅当连启动目标都解析不到时才跳过。
async fn reset_state(device: &str, lines: &[String]) {
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

/// 整脚本从头回放一次（复用 tke run 的 ScriptRunner）。事件吞掉，只取最终结果。
async fn replay(params: &Arc<Params>, script_path: &Path, log_root: Option<&Path>) -> Result<ExecutionResult> {
    let runner = ScriptRunner::new(params.clone());
    let mut sink = |_: &RunEvent| {};
    runner.run(script_path, log_root, &mut sink).await
}
