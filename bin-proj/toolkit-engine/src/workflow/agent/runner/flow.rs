// 主循环：perceive → decide → apply → record（只编排，具体能力委托各子模块）
// 产物经 RunArtifacts 统一保存：每个执行步存 screenshots/step_NNN + page/step_NNN，
// 与 tke run 同构；conversation.jsonl（AI 原始对话）由 transcript 写在同一运行目录。

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::engines::ocr::OcrSource;
use crate::{Fetcher, LlmReply, LlmSession, Result, RunArtifacts, StepResult, Workarea};

/// 终端着色：仅当 stderr 是 TTY 时输出 ANSI，管道/重定向为纯文本（与 tke run 一致）
fn paint(tty: bool, code: &str, s: &str) -> String {
    if tty {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

/// 页面签名哈希（用元素列表文本），检测"原地打转"用
fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// 从模型回复里抽出 JSON 对象（容忍前后多余文字 / ```json 围栏）
fn parse_desc_json(s: &str) -> Option<serde_json::Value> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    serde_json::from_str(s.get(start..=end)?).ok()
}

/// 给 .tks 指令行着色：命令(青) + 元素/参数(黄)。`命令 [{元素}]` / `命令 [参数]`
fn paint_line(tty: bool, line: &str) -> String {
    match line.find(" [") {
        Some(i) => {
            let (verb, rest) = line.split_at(i);
            format!("{} {}", paint(tty, "36", verb), paint(tty, "33", rest.trim_start()))
        }
        None => paint(tty, "36", line),
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

use crate::Platform;

use super::super::execution;
use super::super::interaction::read_user_line;
use super::super::perception::{capture, match_known, render_element_list, Perceived};
use super::super::tools::{parse_tool_call, AgentAction};
use super::super::transcript::Transcript;

/// 循环所需的只读上下文
pub struct DriveCtx<'a> {
    pub device: &'a str,
    pub element_path: &'a Path,
    pub workarea: &'a Workarea,
    pub fetcher: &'a Fetcher,
    pub artifacts: &'a RunArtifacts,
    /// OCR 增强来源（None=不跑 OCR，行为同此前）
    pub ocr: Option<&'a OcrSource>,
    pub max_rounds: usize,
}

/// 循环结果
pub struct DriveOutcome {
    pub success: bool,
    pub reason: String,
    pub lines: Vec<String>,
    pub steps: Vec<StepResult>,
    pub rounds: usize,
    /// 本轮新增的元素名（人工审核用）
    pub created: Vec<String>,
    /// 本轮更新了描述的元素名（人工审核用）
    pub updated: Vec<String>,
    /// 是否被用户中断（Ctrl+C）
    pub aborted: bool,
}

/// 驱动探索循环
pub async fn drive(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
) -> Result<DriveOutcome> {
    let tty = std::io::stderr().is_terminal();

    // 用户中断（Ctrl+C）：监听到则在下一个决策点优雅停止、照常出总结
    let abort = Arc::new(AtomicBool::new(false));
    {
        let a = abort.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                a.store(true, Ordering::Relaxed);
            }
        });
    }

    // 启动加载动画：采集首屏前有数秒空窗，转个 spinner 让用户知道在跑（仅 TTY）
    let spin_stop = Arc::new(AtomicBool::new(false));
    let mut spin_handle = if tty {
        let stop = spin_stop.clone();
        Some(tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0usize;
            while !stop.load(Ordering::Relaxed) {
                eprint!("\r{} 正在启动…", frames[i % frames.len()]);
                let _ = std::io::stderr().flush();
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            }
            eprint!("\r\x1b[K"); // 清除 spinner 行
            let _ = std::io::stderr().flush();
        }))
    } else {
        None
    };

    let mut aborted = false;
    let mut page_sigs: Vec<u64> = Vec::new(); // 各轮结构签名，检测原地打转
    let mut no_progress = 0usize; // 连续多少轮页面无变化（上一步操作没生效）
    let max_llm_calls = ctx.max_rounds * 4 + 10; // 含要图/反问等不前进的轮次
    let mut lines: Vec<String> = Vec::new();
    let mut steps: Vec<StepResult> = Vec::new();
    let mut round = 0usize;
    let mut llm_calls = 0usize;
    let mut finish: Option<(bool, String)> = None;
    // 本轮测试对元素库的变更（供结束时人工审核）
    let mut created: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new(); // 格式化的差异行
    let mut updated_names: Vec<String> = Vec::new(); // 去重用

    'outer: while round < ctx.max_rounds {
        if abort.load(Ordering::Relaxed) {
            aborted = true;
            finish = Some((false, "已终止（用户中断 Ctrl+C）".to_string()));
            break 'outer;
        }
        round += 1;

        // 1) 采集页面
        //    web/iOS 冷启动时尚无会话，采集会失败——此时降级为"空页面 + 提示先 launch"，
        //    不中断循环；AI 调 launch 建会话后，下一轮采集即正常。
        let (p, perceive_err) = match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
            Ok(p) => (p, None),
            Err(e) => {
                tx.log("perceive_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                (
                    Perceived {
                        elements: Vec::new(),
                        shot_path: ctx.workarea.screenshot_path(),
                        xml_path: ctx.workarea.ui_tree_path(),
                        ocr_filled: 0,
                        ocr_added: 0,
                        tabs: Vec::new(),
                    },
                    Some(e.to_string()),
                )
            }
        };
        // 首屏采集完毕，停掉加载动画
        if let Some(h) = spin_handle.take() {
            spin_stop.store(true, Ordering::Relaxed);
            let _ = h.await;
        }

        // 对照元素库：命中(结构/ocr)的标"已知"，让 AI 复用库名、不重复造
        let platform = Platform::from_device(Some(ctx.device));
        let known = match_known(&p.elements, platform, ctx.element_path);
        let n_known = known.iter().filter(|k| k.is_some()).count();
        let n_unknown = p.elements.len() - n_known;
        // 仅 name 的视图，供 apply 强制复用库名
        let known_names: Vec<Option<String>> =
            known.iter().map(|k| k.as_ref().map(|h| h.name.clone())).collect();

        let list_text = render_element_list(&p.elements, &known);

        // 结构签名：只用真实元素的结构文本（排除易变的 OCR 伪元素如时钟、坐标），
        // 用于判断"上一步操作有没有让页面变化"以及是否在多页间打转。
        let struct_sig = hash_str(
            &p.elements
                .iter()
                .filter(|e| e.class_name != "OcrText")
                .map(|e| e.to_ai_text())
                .collect::<Vec<_>>()
                .join("|"),
        );
        let unchanged = page_sigs.last() == Some(&struct_sig); // 与上一轮完全相同 → 上一步没生效
        let revisits = page_sigs.iter().rev().take(8).filter(|&&h| h == struct_sig).count();
        page_sigs.push(struct_sig);
        if unchanged {
            no_progress += 1;
        } else {
            no_progress = 0;
        }
        let stuck = (no_progress >= 1 || revisits >= 2) && !p.elements.is_empty();

        tx.log(
            "page_snapshot",
            serde_json::json!({
                "round": round,
                "element_count": p.elements.len(),
                "known": n_known,
                "unknown": n_unknown,
                "revisits": revisits,
                "elements": list_text.clone(),
                "xml": p.xml_path.to_string_lossy(),
                "perceive_error": perceive_err.clone(),
            }),
        );
        let hint = if perceive_err.is_some() {
            "\n（注意：未能采集到页面——若是 web/iOS 且尚未打开目标，请先调用 launch 打开应用/网址）".to_string()
        } else if no_progress >= 1 {
            format!(
                "\n（重要：你上一步操作后，页面与操作前**完全相同**（已连续 {} 次）。\
                 **不要再重复同一个操作**！可能原因：①它触发了下载（下载不会改变页面，做一次就够，别再点）；\
                 ②它无效或需要换元素/换方式；③在弹层中打开了。\
                 请据此换一个**完全不同**的目标/做法，或 request_screenshot 看截图，或如果该步已完成就继续下一步/finish。）",
                no_progress
            )
        } else if revisits >= 2 {
            "\n（系统提示：你回到了之前出现过的页面、在多页间打转。请停止重复无效操作，换一条**完全不同**的路径。）".to_string()
        } else {
            String::new()
        };
        // 标签页信息（web 多标签时），人和 AI 对称可见
        let tabs_text = crate::format_tabs(&p.tabs);
        let tabs_block = if tabs_text.is_empty() { String::new() } else { format!("{}\n", tabs_text) };
        sess.user_page(format!(
            "{}【第 {} 轮】当前页面元素（[序号] 描述 @(中心坐标)）：\n{}{}\n标有「已知元素」的请复用其 name；请调用一个工具决定下一步。",
            tabs_block, round, list_text, hint
        ));
        // 卡得更死（连续 2 次没变 / 多页打转）：主动把截图塞给 AI，强制触发多模态读图
        if (no_progress >= 2 || revisits >= 3) && perceive_err.is_none() {
            if let Err(e) = sess.user_with_image("（系统：你似乎卡住了，附上当前页面截图，请据图判断下一步该点哪里）", &p.shot_path) {
                tx.log("auto_screenshot_error", serde_json::json!({ "round": round, "error": e.to_string() }));
            } else {
                tx.log("auto_screenshot", serde_json::json!({ "round": round }));
            }
        }
        // OCR 贡献提示（回填/新增），让用户看出 OCR 是否在起作用
        let ocr_tag = if p.ocr_filled + p.ocr_added > 0 {
            paint(tty, "35", &format!("  OCR +{}", p.ocr_filled + p.ocr_added))
        } else {
            String::new()
        };
        let tab_tag = if p.tabs.len() > 1 {
            paint(tty, "34", &format!("  {}标签页", p.tabs.len()))
        } else {
            String::new()
        };
        eprintln!(
            "{}  {} · {}{}{}{}",
            paint(tty, "1", &format!("第 {} 轮", round)),
            paint(tty, "32", &format!("{} 个已知元素", n_known)),
            paint(tty, "2", &format!("{} 个未知元素", n_unknown)),
            ocr_tag,
            tab_tag,
            if perceive_err.is_some() { paint(tty, "31", "  (页面未就绪，待 launch)") } else { String::new() }
        );
        if stuck {
            let msg = if no_progress >= 2 || revisits >= 3 {
                "  ⚠ 上一步没生效/原地打转，已强制附截图让 AI 看图"
            } else if no_progress >= 1 {
                "  ⚠ 上一步操作后页面无变化（可能没生效），已提示 AI 换做法"
            } else {
                "  ⚠ 在多页间打转，已提示 AI 换路径"
            };
            eprintln!("{}", paint(tty, "33", msg));
        }

        // 2) 内层：持续问 AI，直到产生一个"改变页面的动作"或结束
        loop {
            if abort.load(Ordering::Relaxed) {
                aborted = true;
                finish = Some((false, "已终止（用户中断 Ctrl+C）".to_string()));
                break 'outer;
            }
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

            // 本次 LLM 调用的 token 用量（上行/输入、下行/输出），着色、显示在 comment 之后
            let (pt, ct) = sess.last_usage();
            let toks = paint(tty, "34", &format!("↑{} ↓{}", pt, ct));
            tx.log("llm_usage", serde_json::json!({ "round": round, "seq": llm_calls, "prompt_tokens": pt, "completion_tokens": ct }));

            let (thinking, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 200);
                    let say = if b.is_empty() { "（未调用工具，已提示其用工具或 finish）".to_string() } else { b };
                    eprintln!("  {}  {}", say, toks);
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

            let (action, comment) = match parse_tool_call(&primary) {
                Ok(v) => v,
                Err(e) => {
                    tx.log("tool_parse_error", serde_json::json!({ "round": round, "tool": primary.name.clone(), "error": e.to_string() }));
                    sess.tool_result(primary.call_id.as_str(), format!("参数错误: {}", e));
                    continue;
                }
            };
            tx.log(
                "llm_decision",
                serde_json::json!({ "round": round, "tool": primary.name.clone(), "comment": comment.clone(), "arguments": primary.arguments.clone() }),
            );

            // 每次 LLM 调用打印一行思考（comment 优先，其次模型文字，再次 reason，最后工具名）
            // + token 着色在后；不显示"AI"标签
            let say = comment
                .as_deref()
                .map(|s| brief(s, 200))
                .filter(|s| !s.is_empty())
                .or_else(|| thinking.as_deref().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .or_else(|| action.intent().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| format!("调用 {}", primary.name));
            eprintln!("  {}  {}", say, toks);

            match action {
                AgentAction::Finish { success, reason } => {
                    // 回执 finish 的 tool_result，避免后续 desc 生成轮因悬空 tool_call 被 API 拒
                    sess.tool_result(primary.call_id.as_str(), "已结束探索");
                    // 结果在末尾统一 section 展示，这里只记录与收尾
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
                    match execution::apply(
                        ctx.device,
                        ctx.element_path,
                        &device_action,
                        &p.elements,
                        &known_names,
                        &p.shot_path,
                        tx,
                        round,
                    )
                    .await
                    {
                        Ok((line, detail, trace, saved)) => {
                            eprintln!("  {} {}", paint(tty, "32", "✓"), paint_line(tty, &line));
                            // 记录元素库变更（去重），供结束时人工审核
                            if let Some(s) = saved {
                                if s.created {
                                    if !created.contains(&s.name) {
                                        created.push(s.name.clone());
                                    }
                                } else if s.desc_updated
                                    && !created.contains(&s.name)
                                    && !updated_names.contains(&s.name)
                                {
                                    // 写出"哪个元素的 desc 从什么改成什么"
                                    updated.push(format!(
                                        "{} · desc：「{}」→「{}」",
                                        s.name,
                                        s.old_desc.as_deref().unwrap_or("（空）"),
                                        s.new_desc.as_deref().unwrap_or("（空）"),
                                    ));
                                    updated_names.push(s.name.clone());
                                }
                            }
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
                    // settle：等页面切换动画/加载稳定再重新采集，避免抓到上一页或半截动画
                    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    break; // 页面已变 → 重新采集
                }
            }
        }
    }

    // 收尾：确保加载动画已停（如中断发生在首屏采集之前）
    if let Some(h) = spin_handle.take() {
        spin_stop.store(true, Ordering::Relaxed);
        let _ = h.await;
    }

    let (success, reason) = finish.unwrap_or((false, format!("达到最大轮数({})未结束", ctx.max_rounds)));

    // approach A：探索结束后，据本轮各新建元素的实际作用统一生成 desc（创建时不写、不想当然）。
    // 复用同一会话——它已知道每个元素被点击后发生了什么。中断时也生成（元素已被点过，丢了可惜），
    // 给"退出中"提示；再按一次 Ctrl+C 会命中默认信号、强制退出。
    let mut generated: Vec<(String, String)> = Vec::new();
    if !created.is_empty() {
        if aborted {
            eprintln!("{}", paint(tty, "33", "退出中：正在为新建元素生成描述…（再次 Ctrl+C 强制退出）"));
        }
        sess.user(format!(
            "探索结束。请根据本次探索中这些元素被操作后的实际效果，为每个元素写一句准确的 desc\
             （描述该元素本身是什么、点了会怎样，与本次测试过程无关）。\
             只返回一个 JSON 对象，键为元素名、值为其 desc；不要调用工具、不要 ``` 围栏或多余文字。\n元素：{}",
            created.join("、")
        ));
        if let Ok(LlmReply::Text(t)) = sess.next().await {
            tx.log("desc_pass", serde_json::json!({ "content": t.clone() }));
            if let Some(obj) = parse_desc_json(&t) {
                for name in &created {
                    if let Some(d) = obj.get(name).and_then(|v| v.as_str()).map(str::trim) {
                        if !d.is_empty()
                            && crate::tools::element::set_element_desc(ctx.element_path, name, d).is_ok()
                        {
                            generated.push((name.clone(), d.to_string()));
                        }
                    }
                }
            }
        }
    }

    let (tp, tc) = sess.total_usage();

    // —— 统一结果 section（带框）——
    let status = if aborted {
        paint(tty, "33", "■ 已终止")
    } else if success {
        paint(tty, "32", "✓ 达成")
    } else {
        paint(tty, "31", "✗ 未达成")
    };
    // —— section 1：结果 ——
    eprintln!("{}", paint(tty, "1", "╭─ 结果 ──────────────────────────────"));
    eprintln!("  {}   {}（{} 轮）", paint(tty, "2", "状态"), status, round);
    eprintln!("  {}   {}", paint(tty, "2", "依据"), brief(&reason, 200));
    eprintln!("  {}   {}", paint(tty, "2", "模型"), sess.model());
    eprintln!(
        "  {}  {}",
        paint(tty, "2", "Token"),
        paint(tty, "2", &format!("↑{} ↓{} · 合计 {}", tp, tc, tp + tc))
    );
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    // —— section 2：元素库更新 ——
    eprintln!("{}", paint(tty, "1", "╭─ 元素库更新 ────────────────────────"));
    if created.is_empty() {
        eprintln!("  {}   {}", paint(tty, "2", "新增"), paint(tty, "2", "（无）"));
    } else {
        // 每个新建元素单独一行，附带据实际作用生成的 desc（若有）
        let line_for = |c: &String| match generated.iter().find(|(k, _)| k == c) {
            Some((_, d)) => format!("{} · {}", c, brief(d, 80)),
            None => c.clone(),
        };
        eprintln!("  {}   {}", paint(tty, "2", "新增"), paint(tty, "32", &line_for(&created[0])));
        for c in &created[1..] {
            eprintln!("         {}", paint(tty, "32", &line_for(c)));
        }
        eprintln!("  {}", paint(tty, "2", "（已新增，desc 据实际作用生成，请人工二次审核）"));
    }
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    Ok(DriveOutcome { success, reason, lines, steps, rounds: round, created, updated, aborted })
}
