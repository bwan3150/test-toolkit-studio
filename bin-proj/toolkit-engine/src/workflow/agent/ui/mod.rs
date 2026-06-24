// 【UI 前端层】引擎/渲染解耦：引擎只 emit(UiEvent) / drain_commands()，
// 前端可插拔（Plain 行式回归 / JSON NDJSON 双向 / TUI 全屏交互）。
//
//   event    UiEvent（引擎 → 前端，面向渲染）
//   command  UiCommand（前端 → 引擎，指导/中断/答复）
//   plain    PlainFrontend：等价现有 eprintln，非 TTY/无 --json 时用（回归锚点）
//   json     JsonFrontend：NDJSON 双向，被 Electron app spawn 时用
//   tui      TuiFrontend：ratatui 全屏交互，真 TTY 时用

pub mod command;
pub mod event;
pub mod json;
pub mod plain;
pub mod tui;

pub use command::UiCommand;
pub use event::{
    ElementItem, Level, NotReady, Phase, StatusLine, StepState, SubAgent, Tokens, UiEvent,
};
pub use json::JsonFrontend;
pub use plain::PlainFrontend;
pub use tui::TuiFrontend;

/// 引擎面向的前端句柄：发事件 + 在安全点取命令 + 阻塞等答复。
/// 必须 Send（引擎在 tokio task 里持有）；emit/drain 同步（打印点是同步语境），
/// await_answer/shutdown 为 async（替代阻塞 read_line / TUI 线程 join）。
#[async_trait::async_trait]
pub trait Frontend: Send + Sync {
    /// 单向：发一个渲染事件（永不阻塞引擎）
    fn emit(&self, ev: UiEvent);

    /// 安全点非阻塞取命令（每轮开始/LLM 调用前/步骤后调），返回本次累积的所有命令。
    fn drain_commands(&self) -> Vec<UiCommand>;

    /// ask_user：发 AwaitingInput 后阻塞等一个 Answer。返回 None = 期间收到 Abort。
    async fn await_answer(&self, round: usize, question: String) -> Option<String>;

    /// 引擎结束时调用：让前端收尾（TUI 退出 alt-screen / JSON flush / 线程 join）。
    async fn shutdown(self: Box<Self>);
}
