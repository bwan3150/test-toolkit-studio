// 【展示/文本工具】驱动循环与各 agent 共用的纯函数：着色、截断、时长/token 格式化、
// .tks 行的友好显示、动作预览等。从 flow.rs 拆出（flow 只留驱动循环本体）。

use super::super::execution;
use super::super::tools::AgentAction;

/// 终端着色：仅当 stderr 是 TTY 时输出 ANSI，管道/重定向为纯文本（与 tke run 一致）
pub(crate) fn paint(tty: bool, code: &str, s: &str) -> String {
    if tty {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

/// 从模型回复里抽出 JSON 对象（容忍前后多余文字 / ```json 围栏）
pub(crate) fn parse_desc_json(s: &str) -> Option<serde_json::Value> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    serde_json::from_str(s.get(start..=end)?).ok()
}

/// 动作行的友好显示：去掉 .tks 的 `[{ }]` 括号噪声，纯文本不上色（仅前面的 ✓/✗ 带色）。
/// 如 `点击 [{登录按钮}]` → `点击 登录按钮`、`定向滑动 [{640, 406}, 上, 406]` → `定向滑动 640, 406, 上, 406`
pub(crate) fn friendly(line: &str) -> String {
    line.replace("[{", "").replace("}]", "").replace(['[', ']', '{', '}'], "")
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 是否「启动」步(打开浏览器/拉起 App)：脚本的结构性入口。不可被 reexplore/pick 覆盖成点击、
/// 也不可删除——否则后续「重启净化」(reset_state)因解析不到启动目标而空转、整条脚本再也起不来。
pub(crate) fn is_launch_line(line: &str) -> bool {
    line.trim_start().starts_with("启动")
}

/// 是否「断言」步(踩实校验)：它不改变页面(必然被判为空操作)，但它是脚本自检的承重点——
/// 优化阶段绝不能当冗余删掉，否则回放时「走错页」不会在断言处暴露、问题定位变难。
pub(crate) fn is_assert_line(line: &str) -> bool {
    line.trim_start().starts_with("断言")
}

/// 紧凑 token 显示：4443→4.4k、148693→149k、<1000 原样，避免大数字刷屏
pub(crate) fn fmt_tokens(n: i64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 100_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        format!("{}k", (n + 500) / 1000)
    }
}

/// 时长格式化（与 tke run 的 EventPrinter 对齐）：320ms / 3.7s / 1m12s
pub(crate) fn fmt_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m{:02}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

/// 取首个非空行并按字符数截断，避免模型长篇刷屏
pub(crate) fn brief(s: &str, max: usize) -> String {
    let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.chars().count() > max {
        let head: String = line.chars().take(max).collect();
        format!("{}…", head)
    } else {
        line.to_string()
    }
}

/// 「即将执行」的人类可读预览：用 AI 选中元素的文字（执行前即有），让 CLI 先于设备显示，
/// 用户能对上 agent 这步要点啥。返回 None = 非设备动作、不预览。
pub(super) fn preview_action(action: &AgentAction, elements: &[crate::UIElement]) -> Option<String> {
    // 用 auto_name（与 apply 落库同源）拼出和最终 .tks 行一致的展示，如「点击 Products@410_57」
    let name = |id: usize| -> String {
        elements.get(id).map(execution::auto_name).unwrap_or_else(|| format!("元素#{}", id))
    };
    Some(match action {
        AgentAction::Launch { target, .. } => format!("启动 {}", brief(target, 60)),
        AgentAction::Close { target } => format!("关闭 {}", brief(target, 60)),
        AgentAction::Click { element_id, .. } => format!("点击 {}", name(*element_id)),
        AgentAction::Hover { element_id, .. } => format!("悬停 {}", name(*element_id)),
        AgentAction::Input { element_id, text, .. } => format!("输入 {} \"{}\"", name(*element_id), brief(text, 30)),
        AgentAction::LongPress { element_id, duration_ms, .. } => format!("按压 {} {}ms", name(*element_id), duration_ms),
        AgentAction::Clear { element_id, .. } => format!("清空 {}", name(*element_id)),
        AgentAction::Assert { element_id, exist, .. } => {
            format!("断言 {} {}", name(*element_id), if *exist { "存在" } else { "不存在" })
        }
        AgentAction::ClickVisual { .. } => "视觉点击（看图框选）".to_string(),
        AgentAction::AssertVisual { exist, .. } => format!("视觉断言（看图框选）{}", if *exist { "存在" } else { "不存在" }),
        AgentAction::SwipeDir { direction, .. } => format!("定向滑动 {}", dir_cn(direction)),
        AgentAction::SwipeToFind { target, direction } => format!("滚动查找 \"{}\", {}", brief(target, 30), dir_cn(direction)),
        AgentAction::SwipeElement { element_id, direction, .. } => format!("在 {} 上滑 {}", name(*element_id), dir_cn(direction)),
        AgentAction::Drag { from_id, to_id } => format!("拖 {} → {}", name(*from_id), name(*to_id)),
        AgentAction::PressKey { key } => format!("按键 {}", key.trim().to_uppercase()),
        AgentAction::Switch { target } => format!("切换 {}", brief(target, 40)),
        AgentAction::Back => "返回".to_string(),
        AgentAction::HideKeyboard => "隐藏键盘".to_string(),
        AgentAction::Wait { ms: Some(ms), .. } => format!("等待 {}ms", ms),
        AgentAction::Wait { element: Some(e), .. } => format!("等待元素 {}", brief(e, 30)),
        AgentAction::Wait { .. } => "等待".to_string(),
        _ => return None, // Finish/RequestScreenshot/AskUser/Rename 非设备动作，不在此预览
    })
}

/// 方向英文 → 中文（预览用）
fn dir_cn(d: &str) -> &str {
    match d {
        "up" => "上",
        "down" => "下",
        "left" => "左",
        "right" => "右",
        other => other,
    }
}
