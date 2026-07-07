// 【参谋】explorer 提问的中转站（单次 JSON 调用,不接地任务的标准形状）：
//   托管模式(Auto) —— 代答:站在全局目标+当前页面视角直接给行动指示,不打扰用户;
//   交互模式(Ask) —— 出选项:把问题提炼成 2~4 个候选答案,渲染成混合选择列表(选或输入)。
// 这样 explorer 的提问不再是"直通但没人加工"：主 AI 侧的全局视角先过一道,
// 用户要么完全不被打扰,要么看到的是已经带候选项的好问题。

use crate::{AiConfig, LlmReply, LlmSession};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::fmt::parse_desc_json;

/// 参谋答复：Auto 模式取 answer；Ask 模式取 options（空=退化为自由输入）。
pub(super) struct AdvisorReply {
    pub(super) answer: String,
    pub(super) options: Vec<String>,
    pub(super) pt: i64,
    pub(super) ct: i64,
}

/// 请参谋出主意。`need_answer=true`=托管代答；false=为用户生成候选选项。
/// 失败/解析不出时返回空（调用方走兜底：代答退化为"请自行决定"、选项退化为自由输入）。
#[allow(clippy::too_many_arguments)]
pub(super) async fn consult(
    ai: &AiConfig,
    prompts: &PromptSet,
    tx: &mut Transcript,
    goal: &str,
    question: &str,
    explorer_options: &[String],
    steps: &[String],
    page_text: &str,
    need_answer: bool,
) -> AdvisorReply {
    let empty = |pt, ct| AdvisorReply { answer: String::new(), options: Vec::new(), pt, ct };
    let system = prompts.role_system("advisor", "", "");
    let mut sess = match LlmSession::new_for_role(ai, "advisor", system, Vec::new()) {
        Ok(s) => s,
        Err(_) => return empty(0, 0),
    };
    let steps_text = if steps.is_empty() {
        "（还没有执行任何步骤）".to_string()
    } else {
        steps.iter().enumerate().map(|(i, l)| format!("{}. {}", i + 1, super::fmt::friendly(l))).collect::<Vec<_>>().join("\n")
    };
    // explorer 自带的候选（如果有）作为参考一并给参谋
    let extra = if explorer_options.is_empty() {
        String::new()
    } else {
        format!("探索 agent 自己拟的候选（可参考/可替换）：{}", explorer_options.join("；"))
    };
    let mode_key = if need_answer { "mode_answer" } else { "mode_options" };
    let ask = render(
        &prompts.message("advisor", "consult"),
        &[
            ("goal", goal),
            ("steps", &steps_text),
            ("page", if page_text.trim().is_empty() { "（页面为空/未采集）" } else { page_text }),
            ("question", question),
            ("extra", &extra),
            ("mode_instruction", &prompts.message("advisor", mode_key)),
        ],
    );
    tx.log("advisor_consult", serde_json::json!({ "question": question, "need_answer": need_answer }));
    sess.user(ask);
    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return empty(0, 0),
    };
    let (pt, ct) = sess.total_usage();
    tx.log("advisor_reply", serde_json::json!({ "content": reply.clone() }));
    let Some(obj) = parse_desc_json(&reply) else { return empty(pt, ct) };
    AdvisorReply {
        answer: obj["answer"].as_str().unwrap_or("").trim().to_string(),
        options: obj["options"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default(),
        pt,
        ct,
    }
}
