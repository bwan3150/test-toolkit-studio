// 【探索监督官】finish 把关：探索 agent 宣布结束时，独立审查「用户需求 + 全部步骤 + 最后一页元素」，
// 放行才真结束，否则打回继续探索（治"过早 finish/放弃"）。独立会话(无工具)，按作用域归属 "supervisor"。

use crate::{AiConfig, Platform};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;

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
    tx.log("supervisor_session", serde_json::json!({ "system_prompt": system.clone() }));

    let claim = if claimed_success { "已达成" } else { "未达成/放弃" };
    let prompt = render(
        &prompts.message("supervisor", "review"),
        &[("case", case), ("steps", steps_listing), ("page", final_page), ("claim", claim), ("reason", claimed_reason)],
    );
    tx.log("llm_message", serde_json::json!({ "content": prompt.clone() }));
    // 强制工具调用（schema 供应商侧校验），取代文字 JSON 手术
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "approved": { "type": "boolean", "description": "true=放行（结论与页面相符）；false=打回" },
            "reason": { "type": "string", "description": "一句话依据；打回时说明差在哪、该继续做什么" }
        },
        "required": ["approved", "reason"]
    });
    let (obj, pt, ct) = super::oneshot::one_shot(ai, "supervisor", system, "提交审查裁决", schema, prompt).await;
    let obj = obj?;
    tx.log("supervisor_verdict", serde_json::json!({ "content": obj.clone() }));
    let approved = obj["approved"].as_bool().unwrap_or(true);
    let reason = obj["reason"].as_str().unwrap_or("").trim().to_string();
    Some(Verdict { approved, reason, pt, ct })
}
