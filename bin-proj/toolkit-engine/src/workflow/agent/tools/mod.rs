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
pub fn build_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    schema::tool_schemas()
        .into_iter()
        .map(|t| LlmTool::new(t.name, prompts.tool_description(t.name), t.schema))
        .collect()
}
