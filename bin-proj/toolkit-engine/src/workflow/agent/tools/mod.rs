// 【工具】暴露给 AI 的能力集
//   schema   各工具名 + JSON Schema
//   action   AgentAction 强类型动作
//   parse    工具调用 → AgentAction
// description 不写死在此：由 prompt::PromptSet 提供（内置默认 / 外部覆盖）。

pub mod action;
pub mod parse;
pub mod schema;

pub use action::AgentAction;
pub use parse::parse_tool_call;

use crate::LlmTool;

use super::prompt::PromptSet;

/// 组装工具集：schema 来自 schema 表，description 来自 PromptSet（可自定义）
/// 给所有"动作类"工具统一注入 comment 字段（AI 每步的思考），finish/ask_user/
/// request_screenshot 本身已有 reason/question，不注入。
pub fn build_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    const NO_COMMENT: [&str; 3] = ["finish", "ask_user", "request_screenshot"];
    schema::tool_schemas()
        .into_iter()
        .map(|mut t| {
            if !NO_COMMENT.contains(&t.name) {
                inject_comment(&mut t.schema);
            }
            LlmTool::new(t.name, prompts.tool_description(t.name), t.schema)
        })
        .collect()
}

/// 给工具 schema 的 properties 注入可选 comment 字段
fn inject_comment(schema: &mut serde_json::Value) {
    if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
        props.insert(
            "comment".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "这一步你的思考：结合已做过的步骤，你现在在这个页面想做什么、为什么这样做。会实时显示给用户，也会留给后续步骤参考。"
            }),
        );
    }
}
