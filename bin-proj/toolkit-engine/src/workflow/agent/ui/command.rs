// 【UI 命令】用户/前端 → 引擎 的指令（与 UiEvent 反向）。
//
// TUI 把键盘动作翻译成 UiCommand；JSON 前端从 stdin 读一行 JSON 反序列化成 UiCommand。
// 引擎在安全点 drain_commands() 取走并应用（Guidance 注入会话、Abort 优雅停、Answer 回 ask_user）。

use serde::Deserialize;

/// 前端 → 引擎 命令。serde 外部标签 → NDJSON `{"type":"guidance","text":"..."}`。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiCommand {
    /// 中途指导：注入当前会话，下一轮生效
    Guidance { text: String },
    /// 终止（= 现 Ctrl+C 语义；优雅停在下一个安全点）
    Abort,
    /// 回答 ask_user（AwaitingInput 的配对）
    Answer { text: String },
    /// 暂停（引擎在安全点空转等 Resume）
    Pause,
    /// 继续
    Resume,
}
