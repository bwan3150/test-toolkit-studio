// 【TUI 画屏】布局 + 各区渲染：顶栏(状态) / 消息区(自动换行+贴底+窗口化) / 输入区(输入框或
// choosing 列表) / 底栏(快捷键+token)。渲染状态在 model.rs。

use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};


use super::model::{slash_matches, ChoiceState, TuiModel};

/// 四段布局：顶栏(1) / 消息区(min) / 输入框(动态) / 底栏(1)
pub(super) fn view(frame: &mut Frame, model: &TuiModel) {
    // choosing 模式时输入区要容纳 prompt + 选项列表，按需放大；否则固定 3 行单行输入框。
    let input_h: u16 = if let Some(ch) = model.choosing.as_ref() {
        // 边框 2 + prompt 1 + 每项 1 + 每个分组小标题 1（「组\t标签」渲染时组变插一行标题）
        let mut groups: u16 = 0;
        let mut last: Option<&str> = None;
        for o in &ch.options {
            if let Some((g, _)) = o.split_once('\t') {
                if last != Some(g) {
                    groups += 1;
                    last = Some(g);
                }
            }
        }
        (ch.options.len() as u16 + groups + 3 + u16::from(ch.allow_free)).clamp(4, 17)
    } else if model.input.starts_with('/') {
        // slash 指令菜单：匹配项 + 输入行 + 边框
        (slash_matches(&model.input).len() as u16 + 3).clamp(3, 10)
    } else {
        3
    };
    let chunks = Layout::vertical([
        Constraint::Length(1),       // 顶栏
        Constraint::Min(0),          // 消息区
        Constraint::Length(1),       // 空行，隔开消息区与输入框
        Constraint::Length(input_h), // 输入框
        Constraint::Length(1),       // 底栏
    ])
    .split(frame.area());

    // ---- 顶栏：随当前进程状态变化（反色粗体）----
    // 准备中 / 等待回复 / 已暂停 / 探索中 / 诊断修复中 / 稳定性验证中 …
    let top = if let Some(q) = model.awaiting.as_ref() {
        if q.starts_with("已暂停") {
            " ● 已暂停 · 等待指令 ".to_string()
        } else {
            " ● 等待回复 ".to_string()
        }
    } else if let Some((p, n)) = model.phase {
        let label = match n {
            Some(k) => format!("{}#{}", p.label(), k),
            None => p.label().to_string(),
        };
        format!(" ● {}中 · 第{}轮 ", label, model.round)
    } else {
        " ● 准备中 ".to_string()
    };
    frame.render_widget(
        Paragraph::new(Line::from(top)).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );

    // ---- 消息区：自动换行 + 准确贴底 + **窗口化渲染** ----
    // ratatui 0.29 的 Paragraph::line_count 是 unstable 私有方法，不能用；
    // 退而按宽度逐行估算 wrap 后行数累加（每条按显示宽度 ceil 折行），比按 lines.len() 估算准。
    // 只克隆可视窗口覆盖到的那几行（≤ 屏高 + 1 行）交给 Paragraph——此前整段 clone 全部历史，
    // 每帧 O(历史) 的克隆/布局成本随会话线性上涨。行数统计用 usize（u16 在长会话 6.5 万行处会溢出）。
    let msg_area = chunks[1];
    let height = msg_area.height as usize;
    let width = msg_area.width.max(1) as usize;
    // 每条 Line 折行后的行数（空行算 1 行；否则 ceil(显示宽度 / 区宽)）
    let heights: Vec<usize> = model
        .lines
        .iter()
        .map(|l| {
            let w = l.width();
            (w / width + usize::from(w % width != 0)).max(1)
        })
        .collect();
    let total_wrapped: usize = heights.iter().sum();
    let max_top = total_wrapped.saturating_sub(height);
    let y = if model.follow {
        max_top
    } else {
        max_top.saturating_sub(model.scroll as usize)
    };
    // 定位覆盖 [y, y+height) 的行区间：start = 首条进入视口的行，skip = 它折行后要跳过的前几行
    let mut acc = 0usize;
    let mut start = model.lines.len();
    let mut skip = 0usize;
    for (i, h) in heights.iter().enumerate() {
        if acc + h > y {
            start = i;
            skip = y - acc;
            break;
        }
        acc += h;
    }
    let mut visible: Vec<Line<'static>> = Vec::new();
    let mut covered = 0usize;
    for (i, l) in model.lines.iter().enumerate().skip(start) {
        visible.push(l.clone());
        covered += heights[i];
        if covered >= skip + height {
            break;
        }
    }
    frame.render_widget(
        Paragraph::new(Text::from(visible))
            .wrap(Wrap { trim: false })
            .scroll((skip as u16, 0)),
        msg_area,
    );

    // ---- 输入区：choosing 列表 / awaiting 文本输入 / 普通指导 ----
    let input_area = chunks[3];
    if let Some(ch) = model.choosing.as_ref() {
        render_choice(frame, input_area, ch);
    } else {
        render_input(frame, input_area, model);
    }

    // ---- 底栏：快捷键提示（暗灰，随状态变化）----
    let hint = if model.choosing.as_ref().map(|c| c.allow_free).unwrap_or(false) {
        "↑↓ 选择 · 直接打字=自行输入 · Enter 确认 · Esc 放弃"
    } else if model.choosing.is_some() {
        "↑↓ 选择 · Enter 确认 · 1-9 直选 · Esc 放弃"
    } else if model.awaiting.is_some() {
        "Enter 提交 · 输入指导让 AI 继续 · /exit 退出"
    } else if model.finished {
        "输入 /exit 退出"
    } else if model.input.starts_with('/') {
        "Tab 补全 · Enter 执行 · 输入 / 看指令"
    } else {
        "Esc 停止 · 打字+Enter 指导 · / 指令 · ↑↓ 滚动 · /exit 退出"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );
    // 总 token 显示在底栏右侧（与提示同一行，不再内嵌输入框）
    frame.render_widget(
        Paragraph::new(format!("↑{} ↓{} ", model.tok_up, model.tok_down))
            .right_aligned()
            .style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );
}


/// 取字符串**尾部**能放进 max_w 显示宽度的一段（中文等宽字符按显示宽度算）——
/// 输入超宽时水平滚动用：光标在末尾，保证末尾可见。
fn tail_fit(s: &str, max_w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut start = chars.len();
    let mut w = 0usize;
    while start > 0 {
        let cw = Span::raw(chars[start - 1].to_string()).width();
        if w + cw > max_w {
            break;
        }
        w += cw;
        start -= 1;
    }
    chars[start..].iter().collect()
}

/// choosing 模式：在输入区渲染 prompt + 高亮选项列表（方向键选择）。
fn render_choice(frame: &mut Frame, area: ratatui::layout::Rect, ch: &ChoiceState) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        ch.prompt.clone(),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    // 选项串可用「组\t标签」编码分组：组变了就先插一行小标题，选项缩进列在其下。
    let mut last_group: Option<&str> = None;
    for (i, opt) in ch.options.iter().enumerate() {
        let (group, label) = match opt.split_once('\t') {
            Some((g, l)) => (Some(g), l),
            None => (None, opt.as_str()),
        };
        if let Some(g) = group {
            if last_group != Some(g) {
                lines.push(Line::from(Span::styled(
                    g.to_string(),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                )));
                last_group = Some(g);
            }
        }
        let indent = if group.is_some() { "  " } else { "" };
        if i == ch.selected {
            // 选中项：反白高亮 + ▶ 前导
            lines.push(Line::from(Span::styled(
                format!("{}▶ {}", indent, label),
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("{}  {}", indent, label),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    // 内联输入行（allow_free）：直接打字即输入，不必先选"其他"再等输入框
    if ch.allow_free {
        let on_input = ch.selected == ch.options.len();
        let (text, style) = if ch.free_input.is_empty() {
            (
                "其他（直接打字…）".to_string(),
                if on_input {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            )
        } else {
            // 超宽时只显示尾部（正在输入的位置必须可见）
            let budget = (area.width as usize).saturating_sub(12); // 边框2+前缀2+"其他："+光标
            (
                format!("其他：{}▎", tail_fit(&ch.free_input, budget)),
                if on_input {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )
        };
        let prefix = if on_input { "▶ " } else { "  " };
        lines.push(Line::from(Span::styled(format!("{}{}", prefix, text), style)));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title("↑↓ 选择 · Enter 确认")
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ),
        area,
    );
}

/// 普通/awaiting 文本输入框 + 可见光标。
fn render_input(frame: &mut Frame, area: ratatui::layout::Rect, model: &TuiModel) {
    let (title, border_style): (String, Style) = if model.awaiting.is_some() {
        // 问题本体已推进消息流（`? …` 行）——输入框只给固定短提示，不再重复长问题
        ("请输入回复".to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        // 普通状态：输入框左上角不显示提示文字（提示统一在底栏）
        (String::new(), Style::default().fg(Color::DarkGray))
    };
    // slash 菜单：input 以 / 开头时，输入行上方列出匹配指令（首个高亮，Tab 补全）
    let mut body: Vec<Line<'static>> = Vec::new();
    if model.input.starts_with('/') {
        for (i, (name, desc)) in slash_matches(&model.input).iter().enumerate() {
            let name_style = if i == 0 {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            body.push(Line::from(vec![
                Span::styled(format!("{:<8}", name), name_style),
                Span::styled(desc.to_string(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    // 输入超宽时**水平滚动**：以光标为锚截取可见窗口——此前整行塞进去，超宽部分被裁剪、
    // 光标被钳在边缘，正在输入的内容看不见（真机反馈的 bug）
    let inner_w = (area.width as usize).saturating_sub(5); // 边框2 + "> "2 + 光标占位1
    let chars: Vec<char> = model.input.chars().collect();
    let cur = model.cursor.min(chars.len());
    let mut start = cur;
    let mut w = 0usize;
    while start > 0 {
        let cw = Span::raw(chars[start - 1].to_string()).width();
        if w + cw > inner_w {
            break;
        }
        w += cw;
        start -= 1;
    }
    let visible: String = chars[start..].iter().collect();
    body.push(Line::from(format!("> {}", visible)));
    frame.render_widget(
        Paragraph::new(body).block(
            Block::bordered()
                .title(Line::from(title))
                .border_style(border_style),
        ),
        area,
    );
    // 光标定位（输入行是块内最后一行）：x = 边框1 + "> "2 + 窗口内光标前子串宽度
    let prefix: String = chars[start..cur].iter().collect();
    let pre_w = Span::raw(prefix).width() as u16;
    let cursor_x = area.x + 1 + 2 + pre_w;
    let cursor_y = area.y + area.height.saturating_sub(2);
    // 夹在区域内，避免越界
    let cx = cursor_x.min(area.x + area.width.saturating_sub(1));
    frame.set_cursor_position(Position::new(cx, cursor_y));
}
