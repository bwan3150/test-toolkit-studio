// 【验证基础设施】目标标志(marker)推导 + 回放校验原语（page_contains / reset_state /
// strip_trailing_close / launch_spec）。诊断回放在 diagnose.rs；修复(断点续探)在 tksops。
//
// marker 两条来源（可靠性递减）：
//   ① derive_marker_from_page —— 探索成功时从**实际末页文字**原样摘取（首选，finalize 写头）；
//   ② derive_marker —— 凭 goal+步骤"猜"（仅手写/老脚本无头 marker 时兜底）。

use crate::{AiConfig, ControlAction, LlmReply, LlmSession, ScriptParser, TksCommand, TksParam};

use super::super::execution::device::exec;
use super::super::perception::Perceived;
use super::super::transcript::Transcript;
use super::super::prompt::{render, PromptSet};
use super::fmt::{friendly, parse_desc_json};



/// 探索成功收尾时：从**实际末页文字**里"摘取"目标标志（必须是末页原文的片段，机械校验
/// 包含关系——杜绝 LLM 发明页面上不存在的标志）。这是 marker 的**首选来源**：真机教训是
/// 凭 goal+步骤"猜"的 marker 可能指向中途页面（如"登录再登出"猜成登录后主页标题），
/// 错误基线会诱导医生把正确的收尾步骤当"多余流程"删掉。
/// 返回 (marker, prompt_tokens, completion_tokens)；挑不出/校验不过返回空串。
pub(super) async fn derive_marker_from_page(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    case: &str,
    template: &str, // "goal_marker_from_page"(终点) 或 "start_marker_from_page"(起始前提)
    final_page_text: &str,
) -> (String, i64, i64) {
    if final_page_text.trim().is_empty() {
        return (String::new(), 0, 0);
    }
    let system = "你帮助从任务完成后的最终页面文字里，原样摘取一段独特文字，用作回放是否到位的判据。".to_string();
    let mut sess = match LlmSession::new_for_role(ai, "verify", system, Vec::new()) {
        Ok(s) => s,
        Err(_) => return (String::new(), 0, 0),
    };
    let ask = render(&prompts.message("verify", template), &[("case", case), ("page", final_page_text)]);
    tx.log("llm_message", serde_json::json!({ "content": ask.clone() }));
    sess.user(ask);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return (String::new(), 0, 0),
    };
    let (pt, ct) = sess.total_usage();
    tx.log(template, serde_json::json!({ "content": reply.clone() }));
    let marker = parse_desc_json(&reply)
        .and_then(|o| o["goal_marker"].as_str().map(|s| s.trim().to_string()))
        .unwrap_or_default();
    // 机械校验：标志必须真实存在于末页文字（空格/大小写不敏感，与 page_contains 同规则）——
    // LLM 改写/发明的一律丢弃，宁可不校验也不要错误基线
    let norm = |s: &str| -> String { s.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect() };
    if marker.is_empty() || !norm(final_page_text).contains(&norm(&marker)) {
        return (String::new(), pt, ct);
    }
    (marker, pt, ct)
}

/// 路径化工具用：脱离探索会话，**自建一次性会话**从「目标 + 脚本步骤」推出目标标志文本。
/// 供 replay_tks / repair_tks / optimize_tks 复用——**兜底路径**：仅当脚本头没有已持久化的
/// marker（手写/老脚本）才用；探索产出的脚本 finalize 时已从真实末页摘取并写头。
pub(super) async fn derive_marker(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    lines: &[String],
    case: &str,
) -> String {
    let system = "你帮助从测试脚本步骤和目标描述里，找出一段“只有真正到达目标时最终页面才会出现”的独特文字，用作回放是否到位的判据。".to_string();
    let mut sess = match LlmSession::new_for_role(ai, "verify", system, Vec::new()) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    ask_goal_marker(prompts, &mut sess, tx, lines, case).await.unwrap_or_default()
}

/// 问会话要一段「目标标志」文本：只有真正到达目标时最终页面才会出现的独特文字。
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

/// 去掉结尾连续的"关闭"步
pub(super) fn strip_trailing_close(lines: &[String]) -> Vec<String> {
    let mut v = lines.to_vec();
    while v.last().map(|l| l.trim_start().starts_with("关闭")).unwrap_or(false) {
        v.pop();
    }
    v
}

/// 当前页面是否包含目标标志。两点都很关键，否则会把"明明已达成"误判为未达目标、
/// 进而诱导医生删掉正确步骤：
/// 1) 空格/大小写不敏感——与滚动查找 page_has_text 一致（OCR 出来的文字常多/少空格，
///    "DATASHEET-KSL00240-01072025" 在页面里可能渲染成 "DATASHEET - KSL00240 - 01072025"）；
/// 2) 兼查标签页标题与 URL——打开 PDF/新标签这类目标的独特标志(文件名/标题)常只活在 tab 上、
///    不在页面 DOM/OCR 元素里，只查 elements 会漏判（本案 marker 正是 PDF 标签页标题）。
pub(super) fn page_contains(p: &Perceived, marker: &str) -> bool {
    let norm = |s: &str| -> String { s.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect() };
    let needle = norm(marker);
    if needle.is_empty() {
        return false;
    }
    p.elements.iter().any(|e| norm(&e.to_ai_text()).contains(&needle))
        || p.tabs.iter().any(|t| norm(&t.title).contains(&needle) || norm(&t.url).contains(&needle))
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



