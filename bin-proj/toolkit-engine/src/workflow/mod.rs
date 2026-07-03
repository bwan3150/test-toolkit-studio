// Workflow 模块 - 工作流类（综合使用原子方法）
//   script  执行单个 .tks 脚本：逐行实时输出 + 完整产物（log/标注截图/结构文件）
//   flow    依次执行一组 .tks 脚本，产物同上
//   agent   AI 探索生成 .tks（内置 AI 闭环，已替代废弃的 tester-ai 子项目）

pub mod events;
pub mod artifacts;
pub mod script_runner;
pub mod flow;

// .tks 脚本引擎（解析器 + 解释器 + 单步执行器）——与 agent/runner(AI 编排)是两回事，故不叫 runner
pub mod tks;

// AI 探索测试（tke harness）：内置 AI 闭环，逐轮决策生成 .tks
pub mod agent;

pub use events::RunEvent;
pub use tks::{TksRunner, ScriptParser, ScriptInterpreter, ActionTrace, script_to_source, step_to_source};
pub use artifacts::RunArtifacts;
pub use script_runner::ScriptRunner;
pub use flow::{FlowDef, FlowRunner, FlowResult};
pub use agent::{
    LlmSession, LlmTool, LlmToolCall, LlmReply,
    AgentRunner, AgentRunOptions, AgentResult, PromptSet, PromptSpec,
};
