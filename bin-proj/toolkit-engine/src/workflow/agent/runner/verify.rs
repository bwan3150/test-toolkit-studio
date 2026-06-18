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

use crate::{ControlAction, ExecutionResult, LlmReply, LlmSession, Params, Result, RunEvent, ScriptParser, ScriptRunner, StepResult, TksCommand, TksParam};

use super::super::execution::device::exec;
use super::super::execution::script::write_script;
use super::super::perception::{capture, Perceived};
use super::super::transcript::Transcript;
use super::flow::{brief, drive, friendly, paint, parse_desc_json, DriveCtx};
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

    // —— 先提炼：逐个删候选步、每删一步都重启重跑验证是否仍达成目标。删了仍达标才真删，
    //    删了到不了目标就还原。靠重跑判定，不对着文本想当然，AI 删错也会被挡回。——
    //    marker=目标标志文本，后面稳定性验证也用它强制确认"真到达目标"，杜绝"瞎跑也算过"。
    let marker;
    (lines, marker) = minimize(sess, tx, ctx, params, script_path, case, lines).await;

    // 全景记录：初始脚本（提炼后）
    tx.log(
        "verify_start",
        serde_json::json!({ "need_pass": NEED_PASS, "max_repairs": MAX_REPAIRS, "script_lines": lines.clone() }),
    );

    eprintln!();
    eprintln!("{}", paint(tty, "1", "╭─ 自检回放 ──────────────────────────"));

    let mut streak = 0usize; // 连续"到达目标"的次数
    let mut replay_no = 0usize;
    loop {
        // 回放时去掉结尾"关闭"步（关了就看不到目标页，没法校验目标标志）；最终落盘仍含关闭。
        let check = strip_trailing_close(&lines);
        // 1) 重启净化
        reset_state(ctx.device, &check).await;
        // 2) 写回并逐步实时回放（像 tke run，看得到第几步、哪步红了）
        let _ = write_script(script_path, case, &check);
        replay_no += 1;
        eprintln!();
        eprintln!("  {}", paint(tty, "1;36", &format!("第 {} 次验证回放", replay_no)));
        let result = {
            let mut total = 0usize;
            let mut sink = |e: &RunEvent| match e {
                RunEvent::RunStart { total_steps, .. } => total = *total_steps,
                RunEvent::StepEnd { index, command, success, error, .. } => {
                    if *success {
                        eprintln!(
                            "    {}  {}  {}",
                            paint(tty, "32", "✓"),
                            paint(tty, "2", &format!("步 {}/{}", index + 1, total)),
                            friendly(command)
                        );
                    } else {
                        eprintln!(
                            "    {}  {}",
                            paint(tty, "31", &format!("✗ 步 {}/{}  {}", index + 1, total, friendly(command))),
                            paint(tty, "31", &format!("— {}", brief(error.as_deref().unwrap_or(""), 100)))
                        );
                    }
                }
                _ => {}
            };
            ScriptRunner::new(params.clone()).run(script_path, verify_log, &mut sink).await
        };

        // 判定本次回放结果：reached=真到达目标；否则给出失败步 k 供修复
        let (reached, k, failed, err) = match &result {
            Ok(r) if r.success => {
                // 全部步骤无报错，但还要**确认真到达目标**（目标标志出现），否则"瞎跑也算过"
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let goal_ok = marker.is_empty()
                    || matches!(capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await, Ok(p) if page_contains(&p, &marker));
                if goal_ok {
                    (true, 0usize, String::new(), String::new())
                } else {
                    // 跑完没到目标 → 从最后一步起让 AI 续接，把目标走完
                    eprintln!("  {}  目标标志未出现：{}", paint(tty, "31", "✗ 未达目标"), brief(&marker, 40));
                    (false, check.len(), "（目标标志未出现）".to_string(), format!("脚本跑完但未到达目标标志「{}」", marker))
                }
            }
            Ok(r) => {
                let k = r.steps.iter().position(|s| !s.success).unwrap_or_else(|| r.steps.len().saturating_sub(1));
                (false, k, check.get(k).cloned().unwrap_or_default(), r.error.clone().unwrap_or_else(|| "未知错误".into()))
            }
            Err(e) => {
                tx.log("verify_replay", serde_json::json!({ "replay": replay_no, "engine": "tke run", "tokens": 0, "success": false, "error": e.to_string() }));
                eprintln!("  第 {} 次回放：{} 无法执行 — {}", replay_no, paint(tty, "31", "✗"), e);
                break;
            }
        };
        let total_steps = result.as_ref().map(|r| r.steps.len()).unwrap_or(0);
        let steps_log = result.as_ref().map(|r| steps_json(&r.steps)).unwrap_or(serde_json::Value::Null);

        if reached {
            streak += 1;
            tx.log("verify_replay", serde_json::json!({ "replay": replay_no, "engine": "tke run", "tokens": 0, "success": true, "reached_goal": true, "streak": streak, "total_steps": total_steps, "steps": steps_log }));
            eprintln!("  {} 全部 {} 步通过且到达目标（连续 {}/{}）", paint(tty, "32", "✓"), total_steps, streak, NEED_PASS);
            if streak >= NEED_PASS {
                report.passed = true;
                break;
            }
            continue; // 再跑一遍确认稳定
        }

        // —— 未到达目标（某步失败 或 跑完没到目标）：交 AI 续接修复 ——
        {
            streak = 0;
            tx.log("verify_replay", serde_json::json!({ "replay": replay_no, "engine": "tke run", "tokens": 0, "success": false, "failed_step": k + 1, "failed_line": failed, "error": err, "total_steps": total_steps, "steps": steps_log }));
            if report.repairs >= MAX_REPAIRS {
                    eprintln!("  {}", paint(tty, "33", &format!("已达修复上限 {} 次，停止自检", MAX_REPAIRS)));
                    break;
                }
                report.repairs += 1;

                // 3) 失败续接：前 k 步可信保留，AI 从当前实时页面续接修复
                let mut prefix: Vec<String> = lines[..k].to_vec();
                // 全景记录：AI 介入修复的交接点
                tx.log(
                    "verify_repair",
                    serde_json::json!({
                        "repair": report.repairs, "from_step": k + 1, "failed_line": failed,
                        "error": err, "kept_prefix_steps": k,
                    }),
                );
                eprintln!();
                eprintln!(
                    "  {}",
                    paint(tty, "1;33", &format!("◆ AI 接管修复（第 {} 次）：从第 {} 步起重新导航，保留前 {} 步", report.repairs, k + 1, k))
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
                let tail = match drive(sess, tx, ctx, false, "修复·").await {
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
                // 修复本身没达成目标（AI 卡死/超轮，自己 success=false）：别把"能跑通但没真的
                // 完成目标"的步骤拼进去当通过——那是假阳性。直接判验证失败。
                if !tail.success {
                    tx.log("verify_repair_failed", serde_json::json!({ "repair": report.repairs, "reason": tail.reason }));
                    eprintln!("  {}", paint(tty, "33", "修复未能达成测试目标（AI 也没走通），判定验证失败"));
                    break;
                }
                // 修复阶段若改了某元素名，前缀里的引用也要同步（库 key 已变，否则回放找不到）
                for (old, new) in &tail.renames {
                    let (from, to) = (format!("{{{}}}", old), format!("{{{}}}", new));
                    for l in prefix.iter_mut() {
                        if l.contains(&from) {
                            *l = l.replace(&from, &to);
                        }
                    }
                }
                // 拼接：可信前缀 + 修复尾部
                lines = prefix;
                lines.extend(tail.lines);
                if lines.is_empty() {
                    eprintln!("  {}", paint(tty, "33", "修复未产出有效步骤，停止自检"));
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
    eprintln!();
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

/// 【脚本提炼】逐个删候选步、每删一步都重跑验证是否仍达成目标，删了仍达标才真删。
/// 不对着 .tks 文本想当然——靠"删了再重跑"判定，AI 删错也会被重跑挡回、自动还原。
async fn minimize(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    lines: Vec<String>,
) -> (Vec<String>, String) {
    let tty = std::io::stderr().is_terminal();
    if lines.len() < 3 {
        return (lines, String::new());
    }
    // 1) 问 AI：目标标志文本(自动校验用) + 它认为可删的候选步(大胆列，逐个验证)
    let (marker, candidates) = match ask_minimize_plan(sess, tx, &lines, case).await {
        Some(v) => v,
        None => return (lines, String::new()),
    };
    if marker.is_empty() {
        let _ = write_script(script_path, case, &lines);
        return (lines, String::new());
    }

    eprintln!();
    eprintln!(
        "{}",
        paint(tty, "1;36", &format!("▶ 脚本提炼：逐步删冗余并重跑验证（目标标志「{}」）", brief(&marker, 24)))
    );

    // 2) 基线：完整脚本回放后仍能到达目标？**逐步可见**，让人能看到脚本怎么跑、有没有真到目标。
    //    到不了就不提炼（保留全量，交给后面的验证；若标志是 AI 写错，验证也会暴露）。
    eprintln!("  {}", paint(tty, "2", "基线回放（验证完整脚本能到达目标）："));
    if !reaches_goal(ctx, params, script_path, case, &lines, &marker, true).await {
        eprintln!("  {}", paint(tty, "33", "完整脚本回放未到达目标标志，跳过提炼（保留全量，交验证）"));
        tx.log("minimize_skip", serde_json::json!({ "reason": "baseline_miss", "marker": marker }));
        let _ = write_script(script_path, case, &lines);
        return (lines, marker);
    }
    eprintln!("  {}", paint(tty, "32", "✓ 基线达标，开始逐步删冗余（删一步→重跑→仍达标才真删）"));

    if candidates.is_empty() {
        let _ = write_script(script_path, case, &lines);
        return (lines, marker);
    }

    // 3) 先一次性删掉所有候选——能到目标就全删（省去逐个重跑）
    let cand_set: std::collections::HashSet<usize> = candidates.iter().copied().collect();
    let all_trial: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !cand_set.contains(&(i + 1)))
        .map(|(_, l)| l.clone())
        .collect();
    if !all_trial.is_empty() && all_trial.len() < lines.len() && reaches_goal(ctx, params, script_path, case, &all_trial, &marker, false).await {
        eprintln!("  {}", paint(tty, "2", &format!("一次性删除 {} 个冗余步后仍达标", lines.len() - all_trial.len())));
        eprintln!("  {}", paint(tty, "1;36", &format!("提炼完成：{} 步 → {} 步", lines.len(), all_trial.len())));
        tx.log("minimize_result", serde_json::json!({ "from": lines.len(), "to": all_trial.len(), "mode": "batch" }));
        let _ = write_script(script_path, case, &all_trial);
        return (all_trial, marker);
    }

    // 4) 批量删不行 → 逐个删（编号从大到小，删除不影响更小编号），删了仍达标才真删
    let mut current = lines.clone();
    let mut cand: Vec<usize> = candidates.into_iter().filter(|&i| i >= 1 && i <= lines.len()).collect();
    cand.sort_unstable_by(|a, b| b.cmp(a));
    cand.dedup();
    for idx in cand {
        if idx > current.len() {
            continue;
        }
        let mut trial = current.clone();
        let dropped = trial.remove(idx - 1);
        if reaches_goal(ctx, params, script_path, case, &trial, &marker, false).await {
            eprintln!("  {}  {}", paint(tty, "32", "✓ 删"), paint(tty, "2", &friendly(&dropped)));
            current = trial;
        } else {
            eprintln!("  {}  {}", paint(tty, "33", "· 留"), paint(tty, "2", &friendly(&dropped)));
        }
    }
    eprintln!("  {}", paint(tty, "1;36", &format!("提炼完成：{} 步 → {} 步", lines.len(), current.len())));
    tx.log("minimize_result", serde_json::json!({ "from": lines.len(), "to": current.len(), "mode": "one-by-one" }));
    let _ = write_script(script_path, case, &current);
    (current, marker)
}

/// 问 AI 要：目标标志文本 + 可删候选步编号
async fn ask_minimize_plan(sess: &mut LlmSession, tx: &mut Transcript, lines: &[String], case: &str) -> Option<(String, Vec<usize>)> {
    let listing = lines
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, friendly(l)))
        .collect::<Vec<_>>()
        .join("\n");
    sess.user(format!(
        "探索已达成目标。我要把脚本提炼成最短可复用版本，办法是**逐个删候选步、每删一步都重启重跑、\
         验证是否仍达成目标**——删了仍达标才真删，删了到不了就还原保留。所以你可以**大胆**列出所有你\
         觉得多余的步，删错了会被自动挡回，不用怕。\n已执行步骤：\n{}\n\n\
         只返回 JSON：{{\"goal_marker\":\"...\", \"removable\":[编号,...]}}\n\
         - goal_marker：一段**只有真正达成目标时最终页面上才会出现**的独特文字（从你看到的目标页里挑，\
         如文档标题/编号/SKU），用于自动校验是否仍达成目标，**务必准确、独特**。\n\
         - removable：你认为多余、可删的步骤编号（重复点击、走错绕回、点了没用的空步）。\n\
         不要调用工具、不要 ``` 围栏或多余文字。整体测试目标：{}",
        listing, case
    ));
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    tx.log("minimize_plan", serde_json::json!({ "content": reply.clone() }));
    let obj = parse_desc_json(&reply)?;
    let marker = obj["goal_marker"].as_str().unwrap_or("").trim().to_string();
    let removable: Vec<usize> = obj["removable"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).map(|n| n as usize).collect())
        .unwrap_or_default();
    Some((marker, removable))
}

/// 回放 lines（去掉结尾"关闭"步，否则关了就看不到目标页），看最终页面是否出现目标标志文本。
/// verbose=true 时逐步打印（像 tke run），让人看得到怎么跑、有没有真到目标。
async fn reaches_goal(
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    lines: &[String],
    marker: &str,
    verbose: bool,
) -> bool {
    let check = strip_trailing_close(lines);
    if check.is_empty() {
        return false;
    }
    let _ = write_script(script_path, case, &check);
    reset_state(ctx.device, &check).await;
    let ok = if verbose {
        replay_streamed(params, script_path).await
    } else {
        matches!(replay_quiet(params, script_path).await, Ok(r) if r.success)
    };
    if !ok {
        return false;
    }
    // settle 后感知最终页面，匹配目标标志（含 OCR 文本）
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let reached = match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
        Ok(p) => page_contains(&p, marker),
        Err(_) => false,
    };
    if verbose {
        let tty = std::io::stderr().is_terminal();
        if reached {
            eprintln!("    {}  目标标志已出现：{}", paint(tty, "32", "✓ 到达目标"), brief(marker, 40));
        } else {
            eprintln!("    {}  跑完但未出现目标标志：{}", paint(tty, "31", "✗ 未达目标"), brief(marker, 40));
        }
    }
    reached
}

/// 逐步流式回放（像 tke run），返回是否全部成功
async fn replay_streamed(params: &Arc<Params>, script_path: &Path) -> bool {
    let tty = std::io::stderr().is_terminal();
    let mut total = 0usize;
    let mut sink = |e: &RunEvent| match e {
        RunEvent::RunStart { total_steps, .. } => total = *total_steps,
        RunEvent::StepEnd { index, command, success, error, .. } => {
            if *success {
                eprintln!("    {}  {}  {}", paint(tty, "32", "✓"), paint(tty, "2", &format!("步 {}/{}", index + 1, total)), friendly(command));
            } else {
                eprintln!(
                    "    {}  {}",
                    paint(tty, "31", &format!("✗ 步 {}/{}  {}", index + 1, total, friendly(command))),
                    paint(tty, "31", &format!("— {}", brief(error.as_deref().unwrap_or(""), 80)))
                );
            }
        }
        _ => {}
    };
    matches!(ScriptRunner::new(params.clone()).run(script_path, None, &mut sink).await, Ok(r) if r.success)
}

/// 去掉结尾连续的"关闭"步
fn strip_trailing_close(lines: &[String]) -> Vec<String> {
    let mut v = lines.to_vec();
    while v.last().map(|l| l.trim_start().starts_with("关闭")).unwrap_or(false) {
        v.pop();
    }
    v
}

/// 当前页面（含 OCR 伪元素）的文本里是否包含目标标志（不分大小写）
fn page_contains(p: &Perceived, marker: &str) -> bool {
    let m = marker.to_lowercase();
    p.elements.iter().any(|e| e.to_ai_text().to_lowercase().contains(&m))
}

/// 安静回放（不逐步打印），只取最终结果——提炼阶段会反复回放，不刷屏
async fn replay_quiet(params: &Arc<Params>, script_path: &Path) -> Result<ExecutionResult> {
    let runner = ScriptRunner::new(params.clone());
    let mut sink = |_: &RunEvent| {};
    runner.run(script_path, None, &mut sink).await
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

