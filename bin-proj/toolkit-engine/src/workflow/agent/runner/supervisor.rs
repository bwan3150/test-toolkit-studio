// 【探索监督官】finish 把关：探索 agent 宣布结束时，独立审查「用户需求 + 全部步骤 + 最后一页元素」，
// 放行才真结束，否则打回继续探索（治"过早 finish/放弃"）。独立会话(无工具)，按作用域归属 "supervisor"。

use crate::{AiConfig, LlmReply, LlmSession, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::fmt::parse_desc_json;

/// 监督官裁决
pub(super) struct Verdict {
    /// 是否放行结束（true=可结束；false=打回继续）
    pub approved: bool,
    /// 依据：放行=为什么算达成；打回=下一步该往哪走
    pub reason: String,
    pub pt: i64,
    pub ct: i64,
}

/// 让监督官审查一次 finish。返回 None 表示会话/解析失败（上层按"放行"兜底，不卡死探索）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn supervise(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    device: &str,
    case: &str,
    steps_listing: &str,
    final_page: &str,
    claimed_success: bool,
    claimed_reason: &str,
) -> Option<Verdict> {
    let platform = Platform::from_device(Some(device));
    let mut scope = tx.scoped("supervisor");
    let tx = &mut *scope;

    let system = prompts.role_system("supervisor", device, platform.name());
    let mut sess = LlmSession::new_for_role(ai, "supervisor", system.clone(), Vec::new()).ok()?;
    tx.log("supervisor_session", serde_json::json!({ "system_prompt": system }));

    let claim = if claimed_success { "已达成" } else { "未达成/放弃" };
    let prompt = render(
        &prompts.message("supervisor", "review"),
        &[("case", case), ("steps", steps_listing), ("page", final_page), ("claim", claim), ("reason", claimed_reason)],
    );
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    sess.user(prompt);

    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return None,
    };
    let (pt, ct) = sess.total_usage();
    tx.log("supervisor_verdict", serde_json::json!({ "content": reply.clone() }));

    let obj = parse_desc_json(&reply)?;
    let approved = obj["approved"].as_bool().unwrap_or(true);
    let reason = obj["reason"].as_str().unwrap_or("").trim().to_string();
    Some(Verdict { approved, reason, pt, ct })
}
