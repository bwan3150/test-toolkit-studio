// 主循环：perceive → decide → apply → record（只编排，具体能力委托各子模块）

use std::path::Path;

use crate::{Fetcher, LlmReply, LlmSession, Result, Workarea};

use super::super::execution;
use super::super::interaction::read_user_line;
use super::super::perception::{capture, render_element_list};
use super::super::tools::{parse_tool_call, AgentAction};
use super::super::transcript::Transcript;

/// 循环所需的只读上下文
pub struct DriveCtx<'a> {
    pub device: &'a str,
    pub element_path: &'a Path,
    pub workarea: &'a Workarea,
    pub fetcher: &'a Fetcher,
    pub screens_dir: &'a Path,
    pub max_rounds: usize,
}

/// 循环结果
pub struct DriveOutcome {
    pub success: bool,
    pub reason: String,
    pub lines: Vec<String>,
    pub rounds: usize,
}

/// 驱动探索循环
pub async fn drive(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
) -> Result<DriveOutcome> {
    let max_llm_calls = ctx.max_rounds * 4 + 10; // 含要图/反问等不前进的轮次
    let mut lines: Vec<String> = Vec::new();
    let mut round = 0usize;
    let mut llm_calls = 0usize;
    let mut finish: Option<(bool, String)> = None;

    'outer: while round < ctx.max_rounds {
        round += 1;

        // 1) 采集页面
        let p = match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.screens_dir, round).await {
            Ok(p) => p,
            Err(e) => {
                tx.log("perceive_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                return Err(e);
            }
        };
        let list_text = render_element_list(&p.elements);
        tx.log(
            "page_snapshot",
            serde_json::json!({
                "round": round,
                "element_count": p.elements.len(),
                "elements": list_text.clone(),
                "screenshot": p.saved_shot.as_ref().map(|p| p.to_string_lossy().to_string()),
                "xml": p.xml_path.to_string_lossy(),
            }),
        );
        sess.user(format!(
            "【第 {} 轮】当前页面元素（[序号] 描述 @(中心坐标)）：\n{}\n请调用一个工具决定下一步。",
            round, list_text
        ));

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

            let calls = match reply {
                LlmReply::Text(t) => {
                    tx.log("llm_text", serde_json::json!({ "round": round, "content": t }));
                    sess.user("请只通过调用工具来操作；若已完成或无法继续，请调用 finish。");
                    continue;
                }
                LlmReply::ToolCalls(calls) => calls,
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

            match action {
                AgentAction::Finish { success, reason } => {
                    tx.log("finish", serde_json::json!({ "success": success, "reason": reason.clone() }));
                    finish = Some((success, reason));
                    break 'outer;
                }
                AgentAction::RequestScreenshot { reason } => {
                    tx.log("screenshot_requested", serde_json::json!({ "round": round, "reason": reason }));
                    sess.tool_result(primary.call_id.as_str(), "已附上当前页面截图（见下一条消息）");
                    match sess.user_with_image("当前页面截图：", &p.shot_path) {
                        Ok(()) => tx.log(
                            "screenshot_sent",
                            serde_json::json!({ "round": round, "screenshot": p.saved_shot.as_ref().map(|p| p.to_string_lossy().to_string()) }),
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
                        Ok((line, detail)) => {
                            lines.push(line.clone());
                            tx.log("tks_step", serde_json::json!({ "round": round, "line": line.clone(), "exec": detail.clone() }));
                            sess.tool_result(primary.call_id.as_str(), format!("已执行：{}（.tks: {}）", detail, line));
                        }
                        Err(e) => {
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
    Ok(DriveOutcome { success, reason, lines, rounds: round })
}
