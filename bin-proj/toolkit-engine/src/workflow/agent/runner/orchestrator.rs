// 【编排官 orchestrator】与用户对话、统筹整场测试的薄 LLM 外壳。
//
// 形态（对话式会话外壳，类 claude code）：编排官是最顶层的长寿会话，用户与它对话；
// 它通过工具 `run_testcase` 调度成熟的探索→验证子流程（= run_one_testcase，复用 explorer/doctor/verify，
// 行为不变），把结果讲给用户，并据此决定再测/追问/结束。explorer/doctor/verify 降为被调度的 worker。
//
// slice-1a（本刀）：工具 run_testcase / ask_user / finish；半自动——给了用例就直接跑，
// 跑完汇报、问下一步；用户随时插话当硬约束（Guidance）。
// 后续刀：把 run_one_testcase 内部的 explore/verify/diagnose 拆成独立工具，让编排官调度子阶段。

use serde_json::json;

use crate::models::Platform;
use crate::{Frontend, LlmReply, LlmSession, LlmTool, Result, UiCommand, UiEvent};

use super::super::prompt::PromptSet;
use super::super::ui::{Level, Tokens};
use super::options::{AgentResult, AgentRunOptions};
use super::{interrupt, run_one_testcase};

/// 编排官工具集（手写 schema；description 走 PromptSet 角色化，可外部覆盖）
fn orch_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    vec![
        LlmTool::new(
            "run_testcase",
            prompts.role_tool_description("orchestrator", "run_testcase"),
            json!({
                "type": "object",
                "properties": {
                    "testcase": { "type": "string", "description": "要执行的完整测试用例描述" },
                    "note": { "type": "string", "description": "可选：给探索的额外约束/提示（用户纠偏、已否定路径、需留意的细节）" }
                },
                "required": ["testcase"]
            }),
        ),
        LlmTool::new(
            "ask_user",
            prompts.role_tool_description("orchestrator", "ask_user"),
            json!({
                "type": "object",
                "properties": { "question": { "type": "string", "description": "要问用户的问题" } },
                "required": ["question"]
            }),
        ),
        LlmTool::new(
            "finish",
            prompts.role_tool_description("orchestrator", "finish"),
            json!({
                "type": "object",
                "properties": { "reason": { "type": "string", "description": "结束小结：测了什么、结果如何" } },
                "required": ["reason"]
            }),
        ),
    ]
}

/// 开一场编排官会话：以 opts.case 为开场用例（可空——无 --testcase 进对话），
/// 循环 LLM 决策 → 调度工具 → 回填结果，直到 finish / 用户结束 / 中断。
/// 返回最近一次 run_testcase 的结果（供 harness 决定退出码 / Done）；一条都没跑则返回兜底结果。
pub(crate) async fn serve(opts: &AgentRunOptions, ui: &dyn Frontend) -> Result<AgentResult> {
    let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
    let platform = opts.platform.unwrap_or_else(|| Platform::from_device(Some(&device)));
    let prompts = PromptSet::resolve(&opts.prompt)?;
    // system() = primary = 编排官；CLI --system-prompt / --system-prompt-file 覆盖的就是这份
    let system = prompts.system(&device, platform.name());
    let mut sess = LlmSession::new(&opts.ai, system, orch_tools(&prompts))?;

    // 开场：把用户给的初始用例交给编排官（半自动：有用例就让它直接安排执行）
    let opening = opts.case.trim();
    if opening.is_empty() {
        sess.user(
            "会话已开始，用户还没有给出测试用例。请先用 ask_user 简短地问用户想测什么。".to_string(),
        );
    } else {
        sess.user(format!("用户要测的用例：\n{}\n\n请安排执行。", opening));
    }

    // 整场会话的返回值 = 最近一次 run_testcase 的结果
    let mut last_result: Option<AgentResult> = None;
    let mut ran_any = false;
    // 非交互模式下「还没跑过任何用例就只用文字回复」的催促次数（防空转结束、一条没跑）
    let mut nudge = 0;

    loop {
        // 进程级中断（Ctrl+C）：优雅停
        if interrupt::aborted() {
            break;
        }
        // 用户中途指令：插话当硬约束、Abort 当结束
        let mut user_abort = false;
        for cmd in ui.drain_commands() {
            match cmd {
                UiCommand::Guidance { text } => {
                    sess.user(format!("【用户中途指示，请当作硬约束遵守】\n{}", text));
                }
                UiCommand::Abort => user_abort = true,
                _ => {}
            }
        }
        if user_abort {
            break;
        }

        let reply = match sess.next().await {
            Ok(r) => r,
            Err(e) => {
                ui.emit(UiEvent::Notice { level: Level::Err, text: format!("编排官出错：{}", e) });
                break;
            }
        };

        match reply {
            // 编排官纯文本（没有更多工具动作）：展示。
            // 交互式 TUI → 让位给用户等下一句（REPL，空回车/中断=结束）；
            // 非交互（管道/CI/被 app spawn 的 JSON）→ 结束会话，绝不阻塞等输入。
            LlmReply::Text(text) => {
                if !text.trim().is_empty() {
                    emit_orch(ui, &sess, &text);
                }
                // 非交互且还没跑过任何用例：催它用 run_testcase，别只说话就空转结束
                if !ui.is_interactive() && !ran_any && nudge < 2 {
                    nudge += 1;
                    sess.user(
                        "请直接调用 run_testcase 开始执行用户给的用例（不要只用文字回复）。".to_string(),
                    );
                    continue;
                }
                if !ui.is_interactive() {
                    break;
                }
                match ui.await_answer(0, "你的回复（直接回车结束会话）".to_string()).await {
                    Some(s) if !s.trim().is_empty() => sess.user(s),
                    _ => break,
                }
            }
            // 工具调用：逐个调度
            LlmReply::ToolCalls { text, calls } => {
                if let Some(t) = &text {
                    if !t.trim().is_empty() {
                        emit_orch(ui, &sess, t);
                    }
                }
                for call in calls {
                    match call.name.as_str() {
                        "run_testcase" => {
                            let tc = call
                                .arguments
                                .get("testcase")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let note = call
                                .arguments
                                .get("note")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            if tc.trim().is_empty() {
                                sess.tool_result(call.call_id, "未提供 testcase，无法执行。");
                                continue;
                            }
                            emit_orch(
                                ui,
                                &sess,
                                &format!("调度执行用例：{}", tc.lines().next().unwrap_or("").trim()),
                            );
                            // 调度成熟子流程（探索→重探→落脚本→验证修复→收尾）
                            let res = run_one_testcase(opts, ui, &tc, note.as_deref()).await?;
                            let summary = summarize(&res);
                            ran_any = true;
                            last_result = Some(res);
                            sess.tool_result(call.call_id, summary);
                        }
                        "ask_user" => {
                            let q = call
                                .arguments
                                .get("question")
                                .and_then(|v| v.as_str())
                                .unwrap_or("请提供更多信息")
                                .to_string();
                            let ans = ui.await_answer(0, q).await.unwrap_or_default();
                            sess.tool_result(
                                call.call_id,
                                if ans.trim().is_empty() {
                                    "（用户未回答）".to_string()
                                } else {
                                    ans
                                },
                            );
                        }
                        "finish" => {
                            let reason = call
                                .arguments
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .unwrap_or("会话结束")
                                .to_string();
                            emit_orch(ui, &sess, &reason);
                            return Ok(finalize(last_result, ran_any, &reason));
                        }
                        other => {
                            sess.tool_result(call.call_id, format!("未知工具：{}", other));
                        }
                    }
                }
            }
        }
    }

    Ok(finalize(last_result, ran_any, "会话结束"))
}

/// 主 AI（编排官）对用户说的一句话：作为助手本体发 Assistant 事件（前端渲染成纯文本、无名无色）。
fn emit_orch(ui: &dyn Frontend, sess: &LlmSession, text: &str) {
    let (pt, ct) = sess.last_usage();
    ui.emit(UiEvent::Assistant {
        text: text.to_string(),
        tokens: Tokens::new(pt, ct),
    });
}

/// 把一次 run_testcase 的结果摘成给编排官看的工具结果文本
fn summarize(r: &AgentResult) -> String {
    let head = if r.aborted {
        "被中断（用户终止）"
    } else if r.success {
        "目标达成、脚本已生成并（若开启）验证稳定"
    } else {
        "未达成（脚本未稳定，已不保留）"
    };
    format!(
        "执行完成：{}。\n- 探索轮数：{}\n- 脚本：{}\n- 结束依据：{}",
        head,
        r.rounds,
        if r.script.as_os_str().is_empty() {
            "（未生成）".to_string()
        } else {
            r.script.display().to_string()
        },
        r.finish_reason
    )
}

/// 收束整场会话的返回值：有跑过就返回最近一次结果；一条都没跑给兜底结果。
fn finalize(last: Option<AgentResult>, ran_any: bool, reason: &str) -> AgentResult {
    if let Some(r) = last {
        return r;
    }
    AgentResult {
        success: ran_any,
        rounds: 0,
        script: std::path::PathBuf::new(),
        conversation: std::path::PathBuf::new(),
        finish_reason: reason.to_string(),
        aborted: interrupt::aborted(),
    }
}
