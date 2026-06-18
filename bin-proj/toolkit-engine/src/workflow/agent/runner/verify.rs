// 【生成后自检 + 自修复】(--verify)
//
// harness 探索产出 .tks 后，验证它能否被 `tke run` 稳定回放：
//   1) 重启净化：关掉目标（web 销毁会话 / 移动 force-stop），让脚本的「启动」从零开始
//   2) 整脚本从头回放一次（复用 ScriptRunner，即 tke run 的执行路径）
//   3) 失败：前 k 步可信保留，把失败步+错误+当前实时页面交回同一 AI 会话，
//      让它从失败处「续接修复」(drive report=false)，产出修正尾部并拼接
//   4) 成功：连续通过 NEED_PASS 次（默认 2）才算稳定，期间每次都重启净化以验证可重复性
//
// 状态延续依据：web 会话按设备 ID 持久化（与 workarea 无关）、移动端 device 本身即状态，
// 因此回放失败后设备就停在失败页，AI 直接 perceive 即可，无需重放前缀。

use std::io::IsTerminal;
use std::path::Path;
use std::sync::Arc;

use crate::{ControlAction, ExecutionResult, LlmSession, Params, Result, RunEvent, ScriptParser, ScriptRunner, TksCommand, TksParam};

use super::super::execution::device::exec;
use super::super::execution::script::write_script;
use super::super::transcript::Transcript;
use super::flow::{brief, drive, friendly, paint, DriveCtx};
use super::options::VerifyReport;

const NEED_PASS: usize = 2; // 连续干净回放通过几次算「稳定」
const MAX_REPAIRS: usize = 6; // 修复尝试上限（兜底，避免反复修不好时无限循环）

/// 自检并尝试修复，返回（最终脚本步骤, 自检报告）。会把最终版本写回 script_path。
pub async fn verify_and_repair(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    log_root: Option<&Path>,
    mut lines: Vec<String>,
) -> (Vec<String>, VerifyReport) {
    let tty = std::io::stderr().is_terminal();
    let mut report = VerifyReport { ran: true, passed: false, repairs: 0 };

    if lines.is_empty() {
        eprintln!("{}", paint(tty, "33", "自检跳过：没有生成可回放的步骤"));
        return (lines, report);
    }

    eprintln!("{}", paint(tty, "1", "╭─ 自检回放 ──────────────────────────"));

    let mut streak = 0usize; // 连续干净通过次数
    let mut replay_no = 0usize;
    loop {
        // 1) 重启净化
        reset_state(ctx.device, &lines).await;
        // 2) 写回当前版本并整脚本回放
        let _ = write_script(script_path, case, &lines);
        replay_no += 1;
        let result = replay(params, script_path, log_root).await;

        match result {
            // —— 干净通过 ——
            Ok(r) if r.success => {
                streak += 1;
                tx.log(
                    "verify_replay",
                    serde_json::json!({ "replay": replay_no, "success": true, "streak": streak, "steps": r.steps.len() }),
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
                    serde_json::json!({ "replay": replay_no, "success": false, "failed_step": k + 1, "failed_line": failed, "error": err }),
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
                    serde_json::json!({ "replay": replay_no, "success": false, "error": e.to_string() }),
                );
                eprintln!("  第 {} 次回放：{} 无法执行 — {}", replay_no, paint(tty, "31", "✗"), e);
                break;
            }
        }
    }

    // 落盘最终版本
    let _ = write_script(script_path, case, &lines);

    let summary = if report.passed {
        paint(tty, "32", &format!("✓ 稳定通过（连续 {} 次干净回放，修复 {} 次）", NEED_PASS, report.repairs))
    } else {
        paint(tty, "33", &format!("■ 未达稳定（修复 {} 次后仍未连续通过，已保留当前最好版本）", report.repairs))
    };
    eprintln!("  {}   {}", paint(tty, "2", "结果"), summary);
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    (lines, report)
}

/// 重启净化：关掉目标，让脚本的「启动」从零开始。web=销毁会话；移动=force-stop 包名。
/// 仅当脚本自身有「启动」步时才关——否则关了就没法从头起（脚本假定目标已开着）。
async fn reset_state(device: &str, lines: &[String]) {
    let Some(package) = launch_target(lines) else { return };
    let _ = exec(device, ControlAction::Close { package }).await;
    // 给关闭/进程退出留点时间，避免下一步「启动」抢在旧会话销毁前
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
}

/// 从脚本里解析首个「启动」步骤的目标（包名 / URL）；web 用不上（Close 忽略包）。
fn launch_target(lines: &[String]) -> Option<String> {
    let content = format!("步骤:\n{}", lines.join("\n"));
    let script = ScriptParser::new().parse(&content).ok()?;
    for step in &script.steps {
        if step.command == TksCommand::Launch {
            for p in &step.params {
                if let TksParam::Text(t) = p {
                    return Some(t.clone());
                }
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
