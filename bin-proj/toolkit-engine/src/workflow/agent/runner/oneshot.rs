// 【单次结构化调用】所有"单次 JSON agent"(asserter/supervisor/advisor/healer/verify/reflector)
// 的统一通道：注册**一个 report 工具**(schema=期望输出) + tool_choice 强制调它——
// 结构由供应商侧校验,arguments 直接是结构化 JSON。
// 取代旧路径"提示词求 JSON → find('{')..rfind('}') 文本手术 → 失败静默降级"。
// 模型极少数没走工具(回了文字)时带提醒重试一次;仍失败返回 None,调用方 Warn 可见化。

use crate::{AiConfig, LlmReply, LlmSession, LlmTool};

/// 单次强制工具调用。返回 (结果 JSON, prompt_tokens, completion_tokens)；None=两次都没拿到。
pub(super) async fn one_shot(
    ai: &AiConfig,
    role: &str,
    system: String,
    tool_desc: &str,
    schema: serde_json::Value,
    ask: String,
) -> (Option<serde_json::Value>, i64, i64) {
    const TOOL: &str = "report";
    let tool = LlmTool { name: TOOL.to_string(), description: tool_desc.to_string(), schema };
    let mut sess = match LlmSession::new_for_role(ai, role, system, vec![tool]) {
        Ok(s) => s.with_forced_tool(TOOL),
        Err(_) => return (None, 0, 0),
    };
    sess.user(ask);
    for attempt in 0..2 {
        match sess.next().await {
            Ok(LlmReply::ToolCalls { calls, .. }) if !calls.is_empty() => {
                let (pt, ct) = sess.total_usage();
                return (Some(calls[0].arguments.clone()), pt, ct);
            }
            // 理论上 tool_choice 强制后不该出现文字——个别供应商/模型不吃强制时提醒一次
            Ok(_) if attempt == 0 => {
                sess.user(format!("请调用工具 {} 提交结果，不要用文字回复。", TOOL));
            }
            _ => break,
        }
    }
    let (pt, ct) = sess.total_usage();
    (None, pt, ct)
}
