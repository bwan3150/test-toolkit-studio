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
//   runner       【编排】主 AI(orchestrator) 与用户对话、自由调度颗粒化工具（无固定流水线）：
//                  orchestrator 主 AI 会话循环 + 工具调度 + 文件操作 + 授权
//                  testrun      explore：驱动设备 → .tks 写工作区 + .tklib 元素包（自收尾，无常驻态）
//                  tksops       路径化 replay_tks / repair_tks / optimize_tks（对工作区已有 .tks 操作）
//                  flow         驱动循环（看页面→动作→再看），explore 与 tksops 复用
//                  doctor       脚本医生（回放→修复），供 repair_tks
//                  reflect      反思官（失败重探计划 / 定稿命名）+ optimize（删冗余），供 optimize_tks
//                  verify       回放/目标校验基础设施（do_replay / marker）
//                  supervisor · asserter  make_test 时的 finish 把关 / 每步自动断言
//                  options / interrupt / mod(装配)

pub mod execution;
pub mod interaction;
pub mod knowledge;
pub mod perception;
pub mod prompt;
pub mod provider;
pub mod runner;
pub mod tools;
pub mod transcript;
pub mod ui;

pub use prompt::{PromptSet, PromptSpec};
pub use provider::{model_supports_reasoning, LlmReply, LlmSession, LlmTool, LlmToolCall};
pub use runner::{AgentResult, AgentRunOptions, AgentRunner};
pub use ui::{Frontend, JsonFrontend, Level, PlainFrontend, TuiFrontend, UiCommand, UiEvent};
