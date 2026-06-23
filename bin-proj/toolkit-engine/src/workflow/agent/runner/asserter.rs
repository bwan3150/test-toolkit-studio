// 【踩实官】子 agent（单次调用、工具型）：探索主 agent 每做一次"让页面变化了"的导航点击/切换后，
// 由它根据页面 diff（新出现的元素）挑一个**能证明这次点击有效、确实进到了预期新页面**的独有标志元素，
// 系统据此自动插入一条断言把这步踩实——把"断言"从主 agent 手里拿走，避免主 agent 漏断言。
// 独立会话(无工具)，按作用域归属 "asserter"；token 单独计、并入总量。

use crate::{AiConfig, LlmReply, LlmSession, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::flow::parse_desc_json;

/// 踩实官的选择结果
pub(super) struct Pick {
    /// 选中的元素序号（当前页元素列表里的 index）；None=这页找不到可踩实的标志（可能上一步点错了）
    pub index: Option<usize>,
    /// 依据：选中=为什么这是该页独有标志；None=为什么找不到/疑似点错
    pub reason: String,
    pub pt: i64,
    pub ct: i64,
}

/// 让踩实官从"新出现的元素"里挑一个标志元素断言。会话/解析失败时返回 index=None（上层据此跳过、不插断言）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn pick_checkpoint(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    device: &str,
    case: &str,
    action_desc: &str,
    delta_listing: &str,
) -> Pick {
    let none = |reason: String| Pick { index: None, reason, pt: 0, ct: 0 };
    let platform = Platform::from_device(Some(device));
    let mut scope = tx.scoped("asserter");
    let tx = &mut *scope;

    let system = prompts.role_system("asserter", device, platform.name());
    let mut sess = match LlmSession::new(ai, system.clone(), Vec::new()) {
        Ok(s) => s,
        Err(e) => return none(format!("踩实官会话创建失败：{}", e)),
    };
    tx.log("asserter_session", serde_json::json!({ "system_prompt": system }));

    let prompt = render(
        &prompts.message("asserter", "pick"),
        &[("case", case), ("action", action_desc), ("delta", delta_listing)],
    );
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    sess.user(prompt);

    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return none("踩实官无文本回复".into()),
    };
    let (pt, ct) = sess.total_usage();
    tx.log("asserter_pick", serde_json::json!({ "content": reply.clone() }));

    let obj = match parse_desc_json(&reply) {
        Some(o) => o,
        None => return Pick { index: None, reason: "踩实官回复非 JSON".into(), pt, ct },
    };
    let index = obj["index"].as_u64().map(|n| n as usize);
    let reason = obj["reason"].as_str().unwrap_or("").trim().to_string();
    Pick { index, reason, pt, ct }
}
