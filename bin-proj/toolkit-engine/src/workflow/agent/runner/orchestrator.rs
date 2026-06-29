// 【编排官 orchestrator】= 测试领域专精的「主 AI」，最顶层长寿会话，用户与它对话。
//
// 形态（类 claude code）：它能对话、讨论、定方案；通过**细粒度工具**调度被它当 worker 的
// explorer/doctor/verify——不再把整条流水线当黑盒，而是把"探索 / 验证 / 收尾"当**独立步骤**
// 由主 AI 决定顺序（验证三段是 todo、非线性）。用户随时插话当硬约束（Guidance）。
//
// slice 2b：工具 explore / verify / finalize / ask_user / finish，主 AI 持一个「当前运行」TestRun，
// 在它上面推进各阶段。常规节奏 explore → verify → finalize，但主 AI 可停下讨论、跳过验证、
// 或针对问题重新 explore。

use serde_json::json;

use crate::models::Platform;
use crate::{Frontend, LlmReply, LlmSession, LlmTool, Result, UiCommand, UiEvent};

use super::super::prompt::PromptSet;
use super::super::ui::{Level, TodoItem, TodoStatus, Tokens};
use super::options::{AgentResult, AgentRunOptions};
use super::testrun::TestRun;
use super::interrupt;

/// 编排官工具集（手写 schema；description 走 PromptSet 角色化，可外部覆盖）。
fn orch_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    let desc = |name: &str| prompts.role_tool_description("orchestrator", name);
    let empty = || json!({ "type": "object", "properties": {}, "required": [] });
    vec![
        LlmTool::new(
            "explore",
            desc("explore"),
            json!({
                "type": "object",
                "properties": {
                    "goal": { "type": "string", "description": "要在设备上完成的任务目标——测试用例 或 一般任务（如『进入隐私政策页』『找到并截取用户头像』）" },
                    "note": { "type": "string", "description": "可选：额外约束/提示（用户纠偏、已否定路径、需留意的细节）" },
                    "make_test": { "type": "boolean", "description": "要产出可回放、可验证的**测试脚本**时设 true（开每步自动断言+完成把关，之后可 verify）；一般任务/不需校验留 false（默认，更轻、脚本只有设备动作）" },
                    "read_full": { "type": "boolean", "description": "任务是读**长内容**（整页 policy 等）时设 true：到达目标后滚动逐屏收集全文。截图/找元素留 false（默认，不乱滚）" }
                },
                "required": ["goal"]
            }),
        ),
        LlmTool::new("verify", desc("verify"), empty()),
        LlmTool::new("finalize", desc("finalize"), empty()),
        LlmTool::new(
            "save_file",
            desc("save_file"),
            json!({
                "type": "object",
                "properties": {
                    "filename": { "type": "string", "description": "文件名（如 policy.md；只取文件名部分，存到脚本输出目录）" },
                    "content": { "type": "string", "description": "要写入的完整内容" }
                },
                "required": ["filename", "content"]
            }),
        ),
        LlmTool::new(
            "update_todos",
            desc("update_todos"),
            json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "完整的当前计划（每次替换整张清单，不是增量）",
                        "items": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "这一步要做什么" },
                                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"], "description": "状态：pending 待办 / in_progress 进行中 / completed 已完成" }
                            },
                            "required": ["text", "status"]
                        }
                    }
                },
                "required": ["todos"]
            }),
        ),
        LlmTool::new(
            "ask_user",
            desc("ask_user"),
            json!({
                "type": "object",
                "properties": { "question": { "type": "string", "description": "要问用户的问题" } },
                "required": ["question"]
            }),
        ),
        LlmTool::new(
            "finish",
            desc("finish"),
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
/// 返回最近一次 finalize 的结果（供 harness 决定退出码 / Done）；一条都没跑则返回兜底结果。
pub(crate) async fn serve(opts: &AgentRunOptions, ui: &dyn Frontend) -> Result<AgentResult> {
    let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
    let platform = opts.platform.unwrap_or_else(|| Platform::from_device(Some(&device)));
    let prompts = PromptSet::resolve(&opts.prompt)?;
    // system() = primary = 编排官；CLI --system-prompt / --system-prompt-file 覆盖的就是这份
    let system = prompts.system(&device, platform.name());
    let mut sess = LlmSession::new(&opts.ai, system, orch_tools(&prompts))?;

    // 开场：把用户给的初始用例交给编排官（半自动：有用例就让它直接安排）
    let opening = opts.case.trim();
    if opening.is_empty() {
        sess.user("会话已开始，用户还没有说要做什么。请先用 ask_user 简短地问用户想让你在这台设备上做什么。".to_string());
    } else {
        sess.user(format!(
            "用户给的任务：\n{}\n\n请先判断这是「要可回放脚本的测试」还是「一般任务」，再选对工具安排执行。",
            opening
        ));
    }

    // 当前进行中的运行（一条用例的 TestRun，跨工具调用存活）；最近一次 finalize 的结果。
    let mut current: Option<TestRun> = None;
    let mut last_result: Option<AgentResult> = None;
    let mut ran_any = false;
    // 非交互模式下「还没探索过就只用文字回复」的催促次数（防空转结束、一条没跑）
    let mut nudge = 0;

    loop {
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
                if !ui.is_interactive() && !ran_any && nudge < 2 {
                    nudge += 1;
                    sess.user("请直接用对应工具开始（测试用 explore，一般任务用 operate），不要只用文字回复。".to_string());
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
                        // —— 探索：开一条新用例的 TestRun（若上条还没收尾，先收尾、不丢产物）——
                        "explore" => {
                            let goal = arg_str(&call.arguments, "goal");
                            let note = call.arguments.get("note").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let make_test = call.arguments.get("make_test").and_then(|v| v.as_bool()).unwrap_or(false);
                            let read_full = call.arguments.get("read_full").and_then(|v| v.as_bool()).unwrap_or(false);
                            if goal.trim().is_empty() {
                                sess.tool_result(call.call_id, "未提供 goal，无法开始。");
                                continue;
                            }
                            if let Some(prev) = current.take() {
                                if let Ok(r) = prev.finalize(opts, ui).await {
                                    last_result = Some(r);
                                }
                            }
                            emit_orch(ui, &sess, &format!("开始：{}", first_line(&goal)));
                            let run = TestRun::explore(opts, ui, &goal, note.as_deref(), make_test, read_full).await?;
                            ran_any = true;
                            // 工具结果 = 摘要 + 末页可读文字(截断) + 截图路径——通用任务靠它答问/存文件
                            let mut brief = run.explore_brief();
                            let text = run.final_text().trim();
                            if !text.is_empty() {
                                let snippet: String = text.chars().take(4000).collect();
                                brief.push_str(&format!("\n- 末页可读文字（截断 4000 字）：\n{}", snippet));
                            }
                            brief.push_str(&format!("\n- 末页截图：{}", run.final_shot().display()));
                            current = Some(run);
                            sess.tool_result(call.call_id, brief);
                        }
                        // —— 验证：回放→医生修复→稳定性（是否验证由主 AI 决定，不再被 --verify 门控）——
                        "verify" => match current.as_mut() {
                            Some(run) if run.explore_succeeded() => {
                                run.verify(opts, ui).await;
                                let brief = run.verify_brief();
                                sess.tool_result(call.call_id, brief);
                            }
                            Some(_) => {
                                sess.tool_result(call.call_id, "当前探索未达成/被中断，无脚本可验证。可重新 explore，或直接 finalize。");
                            }
                            None => {
                                sess.tool_result(call.call_id, "还没有进行中的探索。请先 explore，再 verify。");
                            }
                        },
                        // —— 收尾：定稿命名 + 提交元素库 + 落日志 + 结果框；消费当前运行 ——
                        "finalize" => match current.take() {
                            Some(run) => {
                                let r = run.finalize(opts, ui).await?;
                                let summary = summarize(&r);
                                last_result = Some(r);
                                sess.tool_result(call.call_id, summary);
                            }
                            None => {
                                sess.tool_result(call.call_id, "没有进行中的运行可收尾（请先 explore）。");
                            }
                        },
                        // —— 把内容写成文件交付（如 policy.md）。非设备动作→给当前 .tks 追加一行注释留痕 ——
                        "save_file" => {
                            let filename = arg_str(&call.arguments, "filename");
                            let content = arg_str(&call.arguments, "content");
                            // 只取文件名部分，防目录穿越；存到脚本输出目录
                            let safe = std::path::Path::new(&filename)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if safe.is_empty() {
                                sess.tool_result(call.call_id, "未提供有效 filename。");
                                continue;
                            }
                            let path = opts.script_dir.join(&safe);
                            match std::fs::write(&path, content.as_bytes()) {
                                Ok(()) => {
                                    // 解析成**绝对路径**再报（script_dir 可能相对/临时目录，相对路径用户找不到）
                                    let abs = std::fs::canonicalize(&path)
                                        .unwrap_or_else(|_| {
                                            std::env::current_dir().map(|d| d.join(&path)).unwrap_or_else(|_| path.clone())
                                        });
                                    // 非设备动作→给当前 .tks 追加注释留痕（如果有进行中的运行）
                                    if let Some(run) = current.as_mut() {
                                        run.note(&format!("保存文件 {}", safe));
                                    }
                                    emit_orch(ui, &sess, &format!("已保存文件：{}", abs.display()));
                                    // 明确叮嘱主 AI：把这个完整路径**原样告诉用户**（别只说"已保存"）
                                    sess.tool_result(
                                        call.call_id,
                                        format!("已保存。文件完整路径（请原样转告用户，这是用户能找到文件的唯一位置）：\n{}", abs.display()),
                                    );
                                }
                                Err(e) => {
                                    sess.tool_result(call.call_id, format!("保存失败：{}", e));
                                }
                            }
                        }
                        // —— 更新计划清单（把探索/验证/收尾等列成可见可勾选的 todo）——
                        "update_todos" => {
                            let items = parse_todos(&call.arguments);
                            let n = items.len();
                            ui.emit(UiEvent::Todo { items });
                            sess.tool_result(call.call_id, format!("计划已更新（{} 项）。继续执行下一步。", n));
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
                                if ans.trim().is_empty() { "（用户未回答）".to_string() } else { ans },
                            );
                        }
                        "finish" => {
                            // 收尾任何未结的运行，避免丢产物
                            if let Some(run) = current.take() {
                                if let Ok(r) = run.finalize(opts, ui).await {
                                    last_result = Some(r);
                                }
                            }
                            let reason = arg_str(&call.arguments, "reason");
                            let reason = if reason.trim().is_empty() { "会话结束".to_string() } else { reason };
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

    // 退出前：收尾任何未结的运行（用户中断/关会话时也别丢产物）
    if let Some(run) = current.take() {
        if let Ok(r) = run.finalize(opts, ui).await {
            last_result = Some(r);
        }
    }
    Ok(finalize(last_result, ran_any, "会话结束"))
}

/// 取字符串参数（缺省空串）
fn arg_str(args: &serde_json::Value, key: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// 取一段文字的首行（用于事件里简短展示用例）
fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("").trim()
}

/// 解析 update_todos 的 todos 数组 → Vec<TodoItem>（空 text 跳过；未知状态当 pending）
fn parse_todos(args: &serde_json::Value) -> Vec<TodoItem> {
    args.get("todos")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let text = t.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if text.is_empty() {
                        return None;
                    }
                    let status = match t.get("status").and_then(|v| v.as_str()).unwrap_or("pending") {
                        "in_progress" => TodoStatus::InProgress,
                        "completed" | "done" => TodoStatus::Done,
                        _ => TodoStatus::Pending,
                    };
                    Some(TodoItem { text, status })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 主 AI（编排官）对用户说的一句话：作为助手本体发 Assistant 事件（前端渲染成纯文本、无名无色）。
fn emit_orch(ui: &dyn Frontend, sess: &LlmSession, text: &str) {
    let (pt, ct) = sess.last_usage();
    ui.emit(UiEvent::Assistant { text: text.to_string(), tokens: Tokens::new(pt, ct) });
}

/// 把一次 finalize 的结果摘成给编排官看的工具结果文本
fn summarize(r: &AgentResult) -> String {
    let head = if r.aborted {
        "被中断（用户终止）"
    } else if r.success {
        "目标达成、脚本已定稿并提交"
    } else {
        "未达成（脚本未稳定，已不保留）"
    };
    format!(
        "收尾完成：{}。\n- 探索轮数：{}\n- 脚本：{}\n- 结束依据：{}",
        head,
        r.rounds,
        if r.script.as_os_str().is_empty() { "（未生成）".to_string() } else { r.script.display().to_string() },
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
