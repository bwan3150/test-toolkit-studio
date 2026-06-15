// 内置默认提示词 —— 以**文件**形式放在 `builtin/`，编译期由 include_str! 嵌入二进制
// （tke 单二进制自包含，运行时不依赖外部文件）。
//
// 布局：一提示词/一工具各一个文件，与用户 `--prompts-dir` 的目录布局**完全一致**，
//   方便直接拷贝 builtin/ 出去改写：
//     builtin/agents/explorer.md     主/角色(subagent)系统提示词
//     builtin/tools/<name>.md        每个工具的 description
//
// 回落链：CLI 注入文本 > CLI .md 文件 > --prompts-dir 目录文件 > 这里的内置文件。
// 占位 {device}/{platform} 由 PromptSet::system 渲染替换。

/// 默认主系统提示词（角色 explorer）
pub const DEFAULT_SYSTEM: &str = include_str!("builtin/agents/explorer.md");

/// 各工具默认 description（外部 <prompts_dir>/tools/<name>.md 可覆盖）
pub fn default_tool_description(name: &str) -> &'static str {
    match name {
        "launch" => include_str!("builtin/tools/launch.md"),
        "close" => include_str!("builtin/tools/close.md"),
        "click" => include_str!("builtin/tools/click.md"),
        "input" => include_str!("builtin/tools/input.md"),
        "long_press" => include_str!("builtin/tools/long_press.md"),
        "clear" => include_str!("builtin/tools/clear.md"),
        "swipe_direction" => include_str!("builtin/tools/swipe_direction.md"),
        "back" => include_str!("builtin/tools/back.md"),
        "hide_keyboard" => include_str!("builtin/tools/hide_keyboard.md"),
        "wait" => include_str!("builtin/tools/wait.md"),
        "request_screenshot" => include_str!("builtin/tools/request_screenshot.md"),
        "ask_user" => include_str!("builtin/tools/ask_user.md"),
        "finish" => include_str!("builtin/tools/finish.md"),
        _ => "",
    }
}
