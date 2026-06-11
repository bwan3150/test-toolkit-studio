// Workflow 模块 - 工作流类（综合使用原子方法）
//   script  执行单个 .tks 脚本：逐行实时输出 + 完整产物（log/标注截图/结构文件）
//   flow    依次执行一组 .tks 脚本，产物同上
//   ai      AI 探索生成 .tks（透传 tester-ai，见 handlers）

pub mod events;
pub mod artifacts;
pub mod script_runner;
pub mod flow;

pub use events::RunEvent;
pub use artifacts::RunArtifacts;
pub use script_runner::ScriptRunner;
pub use flow::{FlowDef, FlowRunner, FlowResult};
