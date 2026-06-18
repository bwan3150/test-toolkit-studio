// AI 探索测试（tke harness 的内部实现，替代已废弃的 tester-ai 子项目）
// 闭环：感知页面 → AI 决策(toolcall) → 执行+落库+记 .tks → 再感知 → … → AI 判定结束
//
// 多层子模块，单文件单职责（自顶向下）：
//   provider     【对接】统一多家大模型(genai)：types / client / session
//   prompt       【提示词】主/角色(subagent)/工具提示词，全部可自定义：defaults / source
//   tools        【工具】schema / action / parse（description 来自 prompt）
//   perception   【感知】capture(采集) / render(元素列表)
//   execution    【执行】device(执行) / library(落库) / script(.tks)
//   knowledge    【记忆/知识库】mem0 + RAG（本期留口子）
//   transcript   【对话日志】conversation.jsonl
//   interaction  【用户交互】ask_user 通道
//   runner       【编排】options / flow(主循环) / mod(装配)

pub mod execution;
pub mod interaction;
pub mod knowledge;
pub mod perception;
pub mod prompt;
pub mod provider;
pub mod runner;
pub mod tools;
pub mod transcript;

pub use prompt::{PromptSet, PromptSpec};
pub use provider::{LlmReply, LlmSession, LlmTool, LlmToolCall};
pub use runner::{AgentResult, AgentRunOptions, AgentRunner};
