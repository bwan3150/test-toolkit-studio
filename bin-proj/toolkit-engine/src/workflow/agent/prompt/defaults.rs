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

/// 默认探索官系统提示词（角色 explorer——被编排官调度的 worker，非 primary）
pub const DEFAULT_EXPLORER_SYSTEM: &str = include_str!("builtin/agents/explorer.md");

/// 默认脚本医生系统提示词（角色 doctor）
pub const DEFAULT_DOCTOR_SYSTEM: &str = include_str!("builtin/agents/doctor.md");

/// 默认探索反思官系统提示词（角色 reflector）
pub const DEFAULT_REFLECTOR_SYSTEM: &str = include_str!("builtin/agents/reflector.md");

/// 默认脚本优化官系统提示词（角色 optimizer：逐步删冗余/合并盲滑）
pub const DEFAULT_OPTIMIZER_SYSTEM: &str = include_str!("builtin/agents/optimizer.md");

/// 默认探索监督官系统提示词（角色 supervisor：finish 处审查需求/步骤/终页，放行才结束）
pub const DEFAULT_SUPERVISOR_SYSTEM: &str = include_str!("builtin/agents/supervisor.md");

/// 默认踩实官系统提示词（角色 asserter：每次导航后据 diff 挑标志元素自动断言踩实）
pub const DEFAULT_ASSERTER_SYSTEM: &str = include_str!("builtin/agents/asserter.md");

/// 默认编排官系统提示词（角色 orchestrator：与用户对话、调度整条探索→验证子流程）
pub const DEFAULT_ORCHESTRATOR_SYSTEM: &str = include_str!("builtin/agents/orchestrator.md");

/// 默认操作员系统提示词（角色 operator：通用设备任务驱动 agent，非测试、不产脚本）
pub const DEFAULT_OPERATOR_SYSTEM: &str = include_str!("builtin/agents/operator.md");

/// 某角色的默认系统提示词（外部 <prompts_dir>/agents/<role>.md 可覆盖）。
/// 全角色对称：每个角色显式一支；orchestrator 是 primary（CLI --system-prompt 覆盖它），
/// explorer/doctor/… 都是被调度的 worker，地位相同。
pub fn default_role_system(role: &str) -> &'static str {
    match role {
        "orchestrator" => DEFAULT_ORCHESTRATOR_SYSTEM,
        "operator" => DEFAULT_OPERATOR_SYSTEM,
        "explorer" => DEFAULT_EXPLORER_SYSTEM,
        "doctor" => DEFAULT_DOCTOR_SYSTEM,
        "reflector" => DEFAULT_REFLECTOR_SYSTEM,
        "optimizer" => DEFAULT_OPTIMIZER_SYSTEM,
        "supervisor" => DEFAULT_SUPERVISOR_SYSTEM,
        "asserter" => DEFAULT_ASSERTER_SYSTEM,
        _ => DEFAULT_EXPLORER_SYSTEM, // 未知角色兜底（不应出现；给探索提示词避免空 system）
    }
}

/// 某角色某工具的默认 description（外部目录可覆盖：explorer→tools/<name>.md，其它→tools/<role>/<name>.md）
pub fn default_tool_description_role(role: &str, name: &str) -> &'static str {
    match role {
        "doctor" => default_doctor_tool_description(name),
        "optimizer" => default_optimizer_tool_description(name),
        "orchestrator" => default_orchestrator_tool_description(name),
        _ => default_tool_description(name),
    }
}

/// 编排官各工具默认 description（外部 <prompts_dir>/tools/orchestrator/<name>.md 可覆盖）
fn default_orchestrator_tool_description(name: &str) -> &'static str {
    match name {
        "explore" => include_str!("builtin/tools/orchestrator/explore.md"),
        "verify" => include_str!("builtin/tools/orchestrator/verify.md"),
        "finalize" => include_str!("builtin/tools/orchestrator/finalize.md"),
        "operate" => include_str!("builtin/tools/orchestrator/operate.md"),
        "save_file" => include_str!("builtin/tools/orchestrator/save_file.md"),
        "update_todos" => include_str!("builtin/tools/orchestrator/update_todos.md"),
        "ask_user" => include_str!("builtin/tools/orchestrator/ask_user.md"),
        "finish" => include_str!("builtin/tools/orchestrator/finish.md"),
        _ => "",
    }
}

/// 脚本优化官各工具默认 description（外部 <prompts_dir>/tools/optimizer/<name>.md 可覆盖）
fn default_optimizer_tool_description(name: &str) -> &'static str {
    match name {
        "delete_step" => include_str!("builtin/tools/optimizer/delete_step.md"),
        "merge_swipes" => include_str!("builtin/tools/optimizer/merge_swipes.md"),
        "done" => include_str!("builtin/tools/optimizer/done.md"),
        _ => "",
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
        ("explorer", "hint_after_close") => include_str!("builtin/messages/explorer/hint_after_close.md"),
        ("explorer", "hint_no_progress") => include_str!("builtin/messages/explorer/hint_no_progress.md"),
        ("explorer", "hint_revisits") => include_str!("builtin/messages/explorer/hint_revisits.md"),
        ("explorer", "nudge_use_tool") => include_str!("builtin/messages/explorer/nudge_use_tool.md"),
        ("explorer", "screenshot_provided") => include_str!("builtin/messages/explorer/screenshot_provided.md"),
        ("explorer", "screenshot_failed") => include_str!("builtin/messages/explorer/screenshot_failed.md"),
        ("explorer", "desc_pass") => include_str!("builtin/messages/explorer/desc_pass.md"),
        ("explorer", "finish_check") => include_str!("builtin/messages/explorer/finish_check.md"),
        ("explorer", "finish_recheck_fail") => include_str!("builtin/messages/explorer/finish_recheck_fail.md"),
        // —— supervisor：探索监督官（finish 把关）——
        ("supervisor", "review") => include_str!("builtin/messages/supervisor/review.md"),
        ("supervisor", "pushback") => include_str!("builtin/messages/supervisor/pushback.md"),
        // —— asserter：踩实官（每次导航后自动断言）——
        ("asserter", "pick") => include_str!("builtin/messages/asserter/pick.md"),
        // —— doctor：脚本医生消息 ——
        ("doctor", "trace") => include_str!("builtin/messages/doctor/trace.md"),
        ("doctor", "trace_objective_minimize") => include_str!("builtin/messages/doctor/trace_objective_minimize.md"),
        ("doctor", "trace_objective_fix") => include_str!("builtin/messages/doctor/trace_objective_fix.md"),
        ("doctor", "reexplore_preamble") => include_str!("builtin/messages/doctor/reexplore_preamble.md"),
        ("doctor", "auto_revert") => include_str!("builtin/messages/doctor/auto_revert.md"),
        ("doctor", "nudge_use_tool") => include_str!("builtin/messages/doctor/nudge_use_tool.md"),
        ("doctor", "finish_pushback") => include_str!("builtin/messages/doctor/finish_pushback.md"),
        ("doctor", "finish_check") => include_str!("builtin/messages/doctor/finish_check.md"),
        // —— verify：验证编排消息 ——
        ("verify", "goal_marker") => include_str!("builtin/messages/verify/goal_marker.md"),
        // —— reflector：探索反思官 ——
        ("reflector", "analyze_failure") => include_str!("builtin/messages/reflector/analyze_failure.md"),
        ("reflector", "analyze_success") => include_str!("builtin/messages/reflector/analyze_success.md"),
        ("reflector", "optimize") => include_str!("builtin/messages/reflector/optimize.md"),
        ("reflector", "finalize") => include_str!("builtin/messages/reflector/finalize.md"),
        ("optimizer", "intro") => include_str!("builtin/messages/optimizer/intro.md"),
        ("optimizer", "nudge") => include_str!("builtin/messages/optimizer/nudge.md"),
        ("optimizer", "fb_step_oob") => include_str!("builtin/messages/optimizer/fb_step_oob.md"),
        ("optimizer", "fb_step_dup") => include_str!("builtin/messages/optimizer/fb_step_dup.md"),
        ("optimizer", "fb_step_loadbearing") => include_str!("builtin/messages/optimizer/fb_step_loadbearing.md"),
        ("optimizer", "fb_step_deleted") => include_str!("builtin/messages/optimizer/fb_step_deleted.md"),
        ("optimizer", "fb_merge_invalid") => include_str!("builtin/messages/optimizer/fb_merge_invalid.md"),
        ("optimizer", "fb_merge_notswipe") => include_str!("builtin/messages/optimizer/fb_merge_notswipe.md"),
        ("optimizer", "fb_merge_overlap") => include_str!("builtin/messages/optimizer/fb_merge_overlap.md"),
        ("optimizer", "fb_merge_done") => include_str!("builtin/messages/optimizer/fb_merge_done.md"),
        ("optimizer", "fb_bad_action") => include_str!("builtin/messages/optimizer/fb_bad_action.md"),
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
        "pick" => include_str!("builtin/tools/doctor/pick.md"),
        "pick_visual" => include_str!("builtin/tools/doctor/pick_visual.md"),
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
        "hover" => include_str!("builtin/tools/hover.md"),
        "input" => include_str!("builtin/tools/input.md"),
        "long_press" => include_str!("builtin/tools/long_press.md"),
        "clear" => include_str!("builtin/tools/clear.md"),
        "assert" => include_str!("builtin/tools/assert.md"),
        "assert_visual" => include_str!("builtin/tools/assert_visual.md"),
        "click_visual" => include_str!("builtin/tools/click_visual.md"),
        "swipe_direction" => include_str!("builtin/tools/swipe_direction.md"),
        "swipe_to_find" => include_str!("builtin/tools/swipe_to_find.md"),
        "swipe_element" => include_str!("builtin/tools/swipe_element.md"),
        "drag" => include_str!("builtin/tools/drag.md"),
        "press_key" => include_str!("builtin/tools/press_key.md"),
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
