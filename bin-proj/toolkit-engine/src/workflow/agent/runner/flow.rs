// 主循环：perceive → decide → apply → record（只编排，具体能力委托各子模块）
// 产物经 RunArtifacts 统一保存：每个执行步存 screenshots/step_NNN + page/step_NNN，
// 与 tke run 同构；conversation.jsonl（AI 原始对话）由 transcript 写在同一运行目录。

use std::io::IsTerminal;
use std::path::Path;

use crate::{Fetcher, LlmReply, LlmSession, Result, RunArtifacts, StepResult, Workarea};

/// 终端着色：仅当 stderr 是 TTY 时输出 ANSI，管道/重定向为纯文本（与 tke run 一致）
fn paint(tty: bool, code: &str, s: &str) -> String {
    if tty {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

/// 取首个非空行并按字符数截断，避免模型长篇刷屏
fn brief(s: &str, max: usize) -> String {
    let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.chars().count() > max {
        let head: String = line.chars().take(max).collect();
        format!("{}…", head)
    } else {
        line.to_string()
    }
}

use super::super::execution;
use super::super::interaction::read_user_line;
use super::super::perception::{capture, render_element_list, Perceived};
use super::super::tools::{parse_tool_call, AgentAction};
use super::super::transcript::Transcript;

/// 循环所需的只读上下文
pub struct DriveCtx<'a> {
    pub device: &'a str,
    pub element_path: &'a Path,
    pub workarea: &'a Workarea,
    pub fetcher: &'a Fetcher,
    pub artifacts: &'a RunArtifacts,
    pub max_rounds: usize,
}

/// 循环结果
pub struct DriveOutcome {
    pub success: bool,
    pub reason: String,
    pub lines: Vec<String>,
    pub steps: Vec<StepResult>,
    pub rounds: usize,
}

/// 驱动探索循环
pub async fn drive(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
) -> Result<DriveOutcome> {
    let tty = std::io::stderr().is_terminal();
    let max_llm_calls = ctx.max_rounds * 4 + 10; // 含要图/反问等不前进的轮次
    let mut lines: Vec<String> = Vec::new();
    let mut steps: Vec<StepResult> = Vec::new();
    let mut round = 0usize;
    let mut llm_calls = 0usize;
    let mut finish: Option<(bool, String)> = None;

    'outer: while round < ctx.max_rounds {
        round += 1;

        // 1) 采集页面
        //    web/iOS 冷启动时尚无会话，采集会失败——此时降级为"空页面 + 提示先 launch"，
        //    不中断循环；AI 调 launch 建会话后，下一轮采集即正常。
        let (p, perceive_err) = match capture(ctx.device, ctx.workarea, ctx.fetcher).await {
            Ok(p) => (p, None),
            Err(e) => {
                tx.log("perceive_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                (
                    Perceived {
                        elements: Vec::new(),
                        shot_path: ctx.workarea.screenshot_path(),
                        xml_path: ctx.workarea.ui_tree_path(),
                    },
                    Some(e.to_string()),
                )
            }
        };
        let list_text = render_element_list(&p.elements);
        tx.log(
            "page_snapshot",
            serde_json::json!({
                "round": round,
                "element_count": p.elements.len(),
                "elements": list_text.clone(),
                "xml": p.xml_path.to_string_lossy(),
                "perceive_error": perceive_err.clone(),
            }),
        );
        let hint = if perceive_err.is_some() {
            "\n（注意：未能采集到页面——若是 web/iOS 且尚未打开目标，请先调用 launch 打开应用/网址）"
        } else {
            ""
        };
        sess.user(format!(
            "【第 {} 轮】当前页面元素（[序号] 描述 @(中心坐标)）：\n{}{}\n请调用一个工具决定下一步。",
            round, list_text, hint
        ));
        eprintln!(
            "{}  {} 个元素{}",
            paint(tty, "1", &format!("第 {} 轮", round)),
            p.elements.len(),
            if perceive_err.is_some() { "  (页面未就绪，待 launch)" } else { "" }
        );

        // 2) 内层：持续问 AI，直到产生一个"改变页面的动作"或结束
        loop {
            if llm_calls >= max_llm_calls {
                finish = Some((false, format!("达到 LLM 调用上限({})，强制结束", max_llm_calls)));
                break 'outer;
            }
            llm_calls += 1;
            tx.log("llm_request", serde_json::json!({ "round": round, "seq": llm_calls }));

            let reply = match sess.next().await {
                Ok(r) => r,
                Err(e) => {
                    tx.log("llm_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                    return Err(e);
                }
            };

            let (thinking, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 200);
                    if !b.is_empty() {
                        eprintln!("  {}  {}", paint(tty, "2", "AI"), b);
                    }
                    eprintln!("  {}", paint(tty, "2", "（未调用工具，已提示其用工具或 finish）"));
                    tx.log("llm_text", serde_json::json!({ "round": round, "content": t }));
                    sess.user("请只通过调用工具来操作；若已完成或无法继续，请调用 finish。");
                    continue;
                }
                LlmReply::ToolCalls { text, calls } => (text, calls),
            };

            // 协议要求：本轮所有 tool_call 都要有 tool_result。仅执行第一个，其余回执"已忽略"
            let primary = calls[0].clone();
            for extra in &calls[1..] {
                sess.tool_result(extra.call_id.as_str(), "已忽略：每轮仅处理第一个工具调用");
            }

            let action = match parse_tool_call(&primary) {
                Ok(a) => a,
                Err(e) => {
                    tx.log("tool_parse_error", serde_json::json!({ "round": round, "tool": primary.name.clone(), "error": e.to_string() }));
                    sess.tool_result(primary.call_id.as_str(), format!("参数错误: {}", e));
                    continue;
                }
            };
            tx.log(
                "llm_decision",
                serde_json::json!({ "round": round, "tool": primary.name.clone(), "arguments": primary.arguments.clone() }),
            );

            // 展示 AI 显式思考文字（模型在调工具时同时给的）；缺省时各分支用动作意图说明补
            let thought = thinking.as_deref().map(|s| brief(s, 200)).filter(|s| !s.is_empty());
            if let Some(s) = &thought {
                eprintln!("  {}  {}", paint(tty, "2", "AI"), s);
            }

            match action {
                AgentAction::Finish { success, reason } => {
                    eprintln!(
                        "{}  {}  {}",
                        paint(tty, "1", "结束"),
                        if success { paint(tty, "32", "达成") } else { paint(tty, "31", "未达成") },
                        reason
                    );
                    tx.log("finish", serde_json::json!({ "success": success, "reason": reason.clone() }));
                    finish = Some((success, reason));
                    break 'outer;
                }
                AgentAction::RequestScreenshot { reason } => {
                    eprintln!("  {}  {}", paint(tty, "2", "请求截图"), reason);
                    tx.log("screenshot_requested", serde_json::json!({ "round": round, "reason": reason }));
                    sess.tool_result(primary.call_id.as_str(), "已附上当前页面截图（见下一条消息）");
                    match sess.user_with_image("当前页面截图：", &p.shot_path) {
                        Ok(()) => tx.log(
                            "screenshot_sent",
                            serde_json::json!({ "round": round, "screenshot": p.shot_path.to_string_lossy() }),
                        ),
                        Err(e) => {
                            tx.log("screenshot_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                            sess.user("（截图附加失败，请基于元素列表继续判断）");
                        }
                    }
                    continue; // 不前进，重新询问
                }
                AgentAction::AskUser { question } => {
                    tx.log("ask_user", serde_json::json!({ "round": round, "question": question.clone() }));
                    let answer = read_user_line(&question).await;
                    tx.log("user_reply", serde_json::json!({ "round": round, "answer": answer.clone() }));
                    sess.tool_result(primary.call_id.as_str(), format!("用户答复：{}", answer));
                    continue; // 不前进，重新询问
                }
                device_action => {
                    // 模型没给显式思考时，用动作的意图说明补上"AI 想干啥"
                    if thought.is_none() {
                        if let Some(s) = device_action.intent() {
                            let b = brief(s, 200);
                            if !b.is_empty() {
                                eprintln!("  {}  {}", paint(tty, "2", "AI"), b);
                            }
                        }
                    }
                    match execution::apply(
                        ctx.device,
                        ctx.element_path,
                        &device_action,
                        &p.elements,
                        &p.shot_path,
                        tx,
                        round,
                    )
                    .await
                    {
                        Ok((line, detail, trace)) => {
                            eprintln!("  {}  {}", line, paint(tty, "32", "✓"));
                            // 存本步产物（screenshots/step_NNN + page/step_NNN），与 run 同构
                            // trace 带点击点+元素 bounds → 截图标注红框+蓝点，可核对 AI 实际点到哪
                            let step_index = steps.len();
                            let (screenshot, xml) =
                                ctx.artifacts.save_step(ctx.workarea, step_index, &trace, &line, true);
                            tx.log(
                                "tks_step",
                                serde_json::json!({ "round": round, "step": step_index + 1, "line": line.clone(), "exec": detail.clone(), "screenshot": screenshot.clone(), "xml": xml.clone() }),
                            );
                            steps.push(StepResult {
                                index: step_index,
                                command: line.clone(),
                                success: true,
                                error: None,
                                duration_ms: 0,
                                line: None,
                                screenshot,
                                xml,
                            });
                            lines.push(line.clone());
                            sess.tool_result(primary.call_id.as_str(), format!("已执行：{}（.tks: {}）", detail, line));
                        }
                        Err(e) => {
                            eprintln!("  {} {}", paint(tty, "31", "✗ 执行失败:"), e);
                            tx.log("exec_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                            sess.tool_result(primary.call_id.as_str(), format!("执行失败: {}", e));
                            continue; // 同一页面重试
                        }
                    }
                    break; // 页面已变 → 重新采集
                }
            }
        }
    }

    let (success, reason) = finish.unwrap_or((false, format!("达到最大轮数({})未结束", ctx.max_rounds)));
    Ok(DriveOutcome { success, reason, lines, steps, rounds: round })
}
