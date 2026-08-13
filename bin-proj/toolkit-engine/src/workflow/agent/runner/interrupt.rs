// 【统一中断】re-export utils 层的进程级中断（实现下沉到 utils::interrupt，让 ScriptRunner 也能查）。
// 各 runner 模块继续用 `super::interrupt::{install, aborted}`，无需改动调用点。

pub use crate::utils::interrupt::{aborted, clear_pause, install, pause_requested, request_pause};
