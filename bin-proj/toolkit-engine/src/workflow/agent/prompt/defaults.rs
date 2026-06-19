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

/// 默认脚本医生系统提示词（角色 doctor）
pub const DEFAULT_DOCTOR_SYSTEM: &str = include_str!("builtin/agents/doctor.md");

/// 某角色的默认系统提示词（外部 <prompts_dir>/agents/<role>.md 可覆盖）
pub fn default_role_system(role: &str) -> &'static str {
    match role {
        "doctor" => DEFAULT_DOCTOR_SYSTEM,
        _ => DEFAULT_SYSTEM, // explorer 及未知角色回落主提示词
    }
}

/// 某角色某工具的默认 description（外部目录可覆盖：explorer→tools/<name>.md，其它→tools/<role>/<name>.md）
pub fn default_tool_description_role(role: &str, name: &str) -> &'static str {
    match role {
        "doctor" => default_doctor_tool_description(name),
        _ => default_tool_description(name),
    }
}

/// 某角色某运行时消息模板的内置默认（外部 <prompts_dir>/messages/<role>/<name>.md 可覆盖）。
/// 这些是**运行时喂给 AI 的消息**（每轮页面、各类提示、诊断 trace、活体重探开场白等），含 {占位符}。
pub fn default_message(role: &str, name: &str) -> &'static str {
    match (role, name) {
        // —— explorer：探索 agent 每轮 / 控制类消息 ——
        ("explorer", "case_intro") => include_str!("builtin/messages/explorer/case_intro.md"),
        ("explorer", "element_tag") => include_str!("builtin/messages/explorer/element_tag.md"),
        ("explorer", "page_round") => include_str!("builtin/messages/explorer/page_round.md"),
        ("explorer", "page_round_visual") => include_str!("builtin/messages/explorer/page_round_visual.md"),
        ("explorer", "hint_perceive_error") => include_str!("builtin/messages/explorer/hint_perceive_error.md"),
        ("explorer", "hint_no_progress") => include_str!("builtin/messages/explorer/hint_no_progress.md"),
        ("explorer", "hint_revisits") => include_str!("builtin/messages/explorer/hint_revisits.md"),
        ("explorer", "nudge_use_tool") => include_str!("builtin/messages/explorer/nudge_use_tool.md"),
        ("explorer", "screenshot_provided") => include_str!("builtin/messages/explorer/screenshot_provided.md"),
        ("explorer", "screenshot_failed") => include_str!("builtin/messages/explorer/screenshot_failed.md"),
        ("explorer", "desc_pass") => include_str!("builtin/messages/explorer/desc_pass.md"),
        // —— doctor：脚本医生消息 ——
        ("doctor", "trace") => include_str!("builtin/messages/doctor/trace.md"),
        ("doctor", "trace_objective_minimize") => include_str!("builtin/messages/doctor/trace_objective_minimize.md"),
        ("doctor", "trace_objective_fix") => include_str!("builtin/messages/doctor/trace_objective_fix.md"),
        ("doctor", "reexplore_preamble") => include_str!("builtin/messages/doctor/reexplore_preamble.md"),
        ("doctor", "auto_revert") => include_str!("builtin/messages/doctor/auto_revert.md"),
        ("doctor", "nudge_use_tool") => include_str!("builtin/messages/doctor/nudge_use_tool.md"),
        ("doctor", "finish_pushback") => include_str!("builtin/messages/doctor/finish_pushback.md"),
        // —— verify：验证编排消息 ——
        ("verify", "goal_marker") => include_str!("builtin/messages/verify/goal_marker.md"),
        _ => "",
    }
}

/// 脚本医生（编辑器 agent）各工具默认 description
fn default_doctor_tool_description(name: &str) -> &'static str {
    match name {
        "delete_lines" => include_str!("builtin/tools/doctor/delete_lines.md"),
        "replace_line" => include_str!("builtin/tools/doctor/replace_line.md"),
        "insert_after" => include_str!("builtin/tools/doctor/insert_after.md"),
        "reexplore" => include_str!("builtin/tools/doctor/reexplore.md"),
        "run" => include_str!("builtin/tools/doctor/run.md"),
        "finish" => include_str!("builtin/tools/doctor/finish.md"),
        _ => "",
    }
}

/// 各工具默认 description（外部 <prompts_dir>/tools/<name>.md 可覆盖）
pub fn default_tool_description(name: &str) -> &'static str {
    match name {
        "launch" => include_str!("builtin/tools/launch.md"),
        "close" => include_str!("builtin/tools/close.md"),
        "click" => include_str!("builtin/tools/click.md"),
        "input" => include_str!("builtin/tools/input.md"),
        "long_press" => include_str!("builtin/tools/long_press.md"),
        "clear" => include_str!("builtin/tools/clear.md"),
        "assert" => include_str!("builtin/tools/assert.md"),
        "click_visual" => include_str!("builtin/tools/click_visual.md"),
        "swipe_direction" => include_str!("builtin/tools/swipe_direction.md"),
        "back" => include_str!("builtin/tools/back.md"),
        "switch" => include_str!("builtin/tools/switch.md"),
        "hide_keyboard" => include_str!("builtin/tools/hide_keyboard.md"),
        "wait" => include_str!("builtin/tools/wait.md"),
        "request_screenshot" => include_str!("builtin/tools/request_screenshot.md"),
        "ask_user" => include_str!("builtin/tools/ask_user.md"),
        "rename_element" => include_str!("builtin/tools/rename_element.md"),
        "finish" => include_str!("builtin/tools/finish.md"),
        _ => "",
    }
}
