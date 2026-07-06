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
    ElementItem, Level, NotReady, Phase, StatusLine, StepState, SubAgent, TodoItem, TodoStatus,
    Tokens, UiEvent,
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

    /// 是否是可与用户多轮对话的交互式前端（真 TTY 的 TUI）。
    /// 编排官据此决定：交互式 → 跑完一条用例后等用户下一句（REPL）；
    /// 非交互式（管道/CI/被 app spawn 的 JSON）→ 跑完即结束，绝不阻塞等输入。
    fn is_interactive(&self) -> bool {
        false
    }

    /// 是否能把问题/授权请求送达用户并等到回复：TUI（弹窗）与 JSON（NDJSON 事件由 app 代问代答）
    /// 为 true；Plain（管道/CI，stdin 不可等）为 false。ask_user / ask_permission 据此决定
    /// 「真问」还是「不阻塞地走兜底」——与 is_interactive（是否开 REPL）是两个独立维度。
    fn supports_prompts(&self) -> bool {
        false
    }

    /// 安全点非阻塞取命令（每轮开始/LLM 调用前/步骤后调），返回本次累积的所有命令。
    fn drain_commands(&self) -> Vec<UiCommand>;

    /// ask_user：发 AwaitingInput 后阻塞等一个 Answer。返回 None = 期间收到 Abort。
    async fn await_answer(&self, round: usize, question: String) -> Option<String>;

    /// 从候选里选一个（设备选择等）。默认：把选项编号拼进问题文本走 await_answer + 解析序号；
    /// TUI 可 override 成方向键列表选择。返回选中下标(0-based)；None=放弃/中断。
    async fn await_choice(&self, prompt: String, options: Vec<String>) -> Option<usize> {
        let mut q = prompt;
        for (i, o) in options.iter().enumerate() {
            q.push_str(&format!("\n  [{}] {}", i + 1, o));
        }
        let ans = self.await_answer(0, q).await?;
        ans.trim()
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1 && *n <= options.len())
            .map(|n| n - 1)
    }

    /// 「从候选里选 **或** 直接输入」（ask_user 带 options 用）：TUI 渲染成列表+内联输入行；
    /// 默认实现走 await_answer（编号选择或直接作答），覆盖 Plain。
    async fn await_choice_or_text(&self, prompt: String, options: Vec<String>) -> Option<ChoiceReply> {
        let mut q = prompt;
        for (i, o) in options.iter().enumerate() {
            q.push_str(&format!("\n  [{}] {}", i + 1, o));
        }
        q.push_str("\n（输入序号选择，或直接输入你的回答）");
        let ans = self.await_answer(0, q).await?;
        let t = ans.trim();
        if let Ok(n) = t.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return Some(ChoiceReply::Pick(n - 1));
            }
        }
        if t.is_empty() {
            return None;
        }
        Some(ChoiceReply::Text(ans))
    }

    /// 引擎结束时调用：让前端收尾（TUI 退出 alt-screen / JSON flush / 线程 join）。
    async fn shutdown(self: Box<Self>);
}

/// 「选项 或 自由输入」的答复
pub enum ChoiceReply {
    /// 选中了第 n 个候选项（0-based）
    Pick(usize),
    /// 用户直接输入了文本
    Text(String),
}

/// 带候选项的提问（opencode 式）：ask_user 带 options 时统一走这里。
/// TUI 渲染成"方向键选择 + 末尾可**直接打字**的输入行"（不必先选『其他』再弹输入框）；
/// Plain/JSON 由 await_choice_or_text 的各自实现兜底。None = 用户放弃/中断。
pub async fn ask_with_options(ui: &dyn Frontend, _round: usize, question: &str, options: &[String]) -> Option<String> {
    match ui.await_choice_or_text(question.to_string(), options.to_vec()).await? {
        ChoiceReply::Pick(i) => options.get(i).cloned(),
        ChoiceReply::Text(t) => Some(t),
    }
}
