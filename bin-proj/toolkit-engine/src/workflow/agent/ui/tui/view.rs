// 【TUI 画屏】inline 手写渲染(dialoguer/Claude Code 模式):历史行由 mod.rs 直接 print 进
// 终端 scrollback(原生滚动/选择/复制);本文件负责两件事:
//   build_view  —— 生成底部小窗的行列表(状态行 + 输入框/choosing 列表 + 底栏)+ 光标位置,
//                  高度**动态**(选项多就高,平时 5 行),行宽裁到终端宽内;
//   line_ansi   —— ratatui Line/Span → ANSI 字符串(fg/bg/BOLD),打印交给终端,
//                  宽字符(中文)由终端原生处理——绕开 ratatui insert_before 的 CJK cell bug。

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::model::{slash_matches, ChoiceState, TuiModel};

/// 生成底部小窗:返回 (行列表, 光标位置(行号, 列显示宽))。行宽已裁到 width-1 以内。
pub(super) fn build_view(model: &TuiModel, width: u16) -> (Vec<Line<'static>>, Option<(u16, u16)>) {
    let w = (width.max(20) as usize).saturating_sub(1);
    let mut rows: Vec<Line<'static>> = Vec::new();
    // 小窗顶部空一行:内容流与状态栏/输入框之间的呼吸,不再糊在一起
    rows.push(Line::from(""));

    // ---- 状态行:进程状态 + 正在执行的步骤(反色粗体) ----
    let mut top = if let Some(q) = model.awaiting.as_ref() {
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
    if let Some(rs) = model.running_step.as_ref() {
        top.push_str(&format!(" {} ", rs));
    }
    rows.push(Line::from(Span::styled(
        top,
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));

    // ---- 输入区:choosing 列表 / 文本输入框 ----
    let cursor = if let Some(ch) = model.choosing.as_ref() {
        push_choice_rows(&mut rows, ch, w);
        None
    } else {
        Some(push_input_rows(&mut rows, model, w))
    };

    // ---- 底栏:快捷键提示 + 右对齐 token ----
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
        "Esc 停止 · 打字+Enter 指导 · / 指令 · /exit 退出"
    };
    let tok = format!("↑{} ↓{}", model.tok_up, model.tok_down);
    let pad = w.saturating_sub(disp_width(hint) + disp_width(&tok)).max(1);
    rows.push(Line::from(Span::styled(
        format!("{}{}{}", hint, " ".repeat(pad), tok),
        Style::default().fg(Color::DarkGray),
    )));

    // 行宽统一裁到 w 以内(防终端 auto-wrap 破坏相对定位)
    let rows = rows.into_iter().map(|l| clip_line(l, w)).collect();
    (rows, cursor)
}

/// 输入框(边框 3 行,slash 菜单时更多):返回光标 (行号, 列显示宽)。
fn push_input_rows(rows: &mut Vec<Line<'static>>, model: &TuiModel, w: usize) -> (u16, u16) {
    let (title, bstyle): (&str, Style) = if model.awaiting.is_some() {
        ("请输入回复", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        ("", Style::default().fg(Color::DarkGray))
    };
    rows.push(border_top(title, w, bstyle));
    // slash 菜单:输入行上方列出匹配指令(首个高亮,Tab 补全)
    if model.input.starts_with('/') {
        for (i, (name, desc)) in slash_matches(&model.input).iter().enumerate() {
            let name_style = if i == 0 {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            rows.push(boxed_line(
                Line::from(vec![
                    Span::styled(format!("{:<8}", name), name_style),
                    Span::styled(desc.to_string(), Style::default().fg(Color::DarkGray)),
                ]),
                w,
                bstyle,
            ));
        }
    }
    // 输入行:超宽时以光标为锚水平滚动(正在输入的位置必须可见)
    let inner_w = w.saturating_sub(2 + 2 + 1); // 边框2 + "> "2 + 光标1
    let chars: Vec<char> = model.input.chars().collect();
    let cur = model.cursor.min(chars.len());
    let mut start = cur;
    let mut acc = 0usize;
    while start > 0 {
        let cw = disp_width(&chars[start - 1].to_string());
        if acc + cw > inner_w {
            break;
        }
        acc += cw;
        start -= 1;
    }
    let visible: String = chars[start..].iter().collect();
    let input_row = rows.len() as u16;
    rows.push(boxed_line(Line::from(format!("> {}", visible)), w, bstyle));
    rows.push(border_bottom(w, bstyle));
    // 光标列 = 边框1 + "> "2 + 窗口内光标前子串显示宽
    let prefix: String = chars[start..cur].iter().collect();
    let col = 1 + 2 + disp_width(&prefix) as u16;
    (input_row, col)
}

/// choosing 列表(动态高度;超过上限时窗口化滚动保证选中可见)。
fn push_choice_rows(rows: &mut Vec<Line<'static>>, ch: &ChoiceState, w: usize) {
    let bstyle = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let title = if ch.prompt.trim().is_empty() {
        "下一步 · ↑↓ 选择 · Enter 确认"
    } else {
        "↑↓ 选择 · Enter 确认"
    };
    rows.push(border_top(title, w, bstyle));
    let mut body: Vec<Line<'static>> = Vec::new();
    let mut sel_row = 0usize;
    if !ch.prompt.trim().is_empty() {
        body.push(Line::from(Span::styled(
            ch.prompt.clone(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
    }
    // 选项串可用「组\t标签」编码分组:组变了先插一行小标题
    let mut last_group: Option<&str> = None;
    for (i, opt) in ch.options.iter().enumerate() {
        let (group, label) = match opt.split_once('\t') {
            Some((g, l)) => (Some(g), l),
            None => (None, opt.as_str()),
        };
        if let Some(g) = group {
            if last_group != Some(g) {
                body.push(Line::from(Span::styled(
                    g.to_string(),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                )));
                last_group = Some(g);
            }
        }
        let indent = if group.is_some() { "  " } else { "" };
        if i == ch.selected {
            sel_row = body.len();
            body.push(Line::from(Span::styled(
                format!("{}▶ {}", indent, label),
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
        } else {
            body.push(Line::from(Span::styled(
                format!("{}  {}", indent, label),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    // 内联输入行(allow_free):直接打字即输入
    if ch.allow_free {
        let on_input = ch.selected == ch.options.len();
        if on_input {
            sel_row = body.len();
        }
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
            let budget = w.saturating_sub(12);
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
        body.push(Line::from(Span::styled(format!("{}{}", prefix, text), style)));
    }
    // 窗口化:列表太长时以选中行为中心裁一段(动态高度,一般全显)
    const MAX_BODY: usize = 14;
    if body.len() > MAX_BODY {
        let start = sel_row.saturating_sub(MAX_BODY / 2).min(body.len() - MAX_BODY);
        body = body.into_iter().skip(start).take(MAX_BODY).collect();
    }
    for b in body {
        rows.push(boxed_line(b, w, bstyle));
    }
    rows.push(border_bottom(w, bstyle));
}

// ============================ 小件 ============================

/// 边框顶:┌标题────┐
fn border_top(title: &str, w: usize, style: Style) -> Line<'static> {
    let fill = w.saturating_sub(2 + disp_width(title));
    Line::from(Span::styled(format!("┌{}{}┐", title, "─".repeat(fill)), style))
}

/// 边框底:└────┘
fn border_bottom(w: usize, style: Style) -> Line<'static> {
    Line::from(Span::styled(format!("└{}┘", "─".repeat(w.saturating_sub(2))), style))
}

/// 内容行加左右边框:│内容…(右填充)│
fn boxed_line(content: Line<'static>, w: usize, bstyle: Style) -> Line<'static> {
    let inner = w.saturating_sub(2);
    let clipped = clip_line(content, inner);
    let used: usize = clipped.spans.iter().map(|s| disp_width(&s.content)).sum();
    let mut spans = vec![Span::styled("│", bstyle)];
    spans.extend(clipped.spans);
    spans.push(Span::raw(" ".repeat(inner.saturating_sub(used))));
    spans.push(Span::styled("│", bstyle));
    Line::from(spans)
}

/// 按显示宽度裁剪 Line(中文宽字符按 2 算),防终端 auto-wrap 破坏小窗定位。
fn clip_line(line: Line<'static>, max_w: usize) -> Line<'static> {
    let total: usize = line.spans.iter().map(|s| disp_width(&s.content)).sum();
    if total <= max_w {
        return line;
    }
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for sp in line.spans {
        if used >= max_w {
            break;
        }
        let sw = disp_width(&sp.content);
        if used + sw <= max_w {
            used += sw;
            out.push(sp);
        } else {
            let mut cut = String::new();
            for c in sp.content.chars() {
                let cw = disp_width(&c.to_string());
                if used + cw > max_w {
                    break;
                }
                used += cw;
                cut.push(c);
            }
            out.push(Span::styled(cut, sp.style));
            break;
        }
    }
    Line::from(out)
}

/// 字符串显示宽度(unicode width,与终端渲染一致)
pub(super) fn disp_width(s: &str) -> usize {
    Span::raw(s.to_string()).width()
}

/// 取字符串**尾部**能放进 max_w 显示宽度的一段——输入超宽时水平滚动用。
fn tail_fit(s: &str, max_w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut start = chars.len();
    let mut acc = 0usize;
    while start > 0 {
        let cw = disp_width(&chars[start - 1].to_string());
        if acc + cw > max_w {
            break;
        }
        acc += cw;
        start -= 1;
    }
    chars[start..].iter().collect()
}

/// ratatui Line → ANSI 字符串(fg/bg/BOLD)。历史行打印进 scrollback 全靠它——
/// 宽字符交给终端原生处理,绕开 ratatui insert_before 的 CJK cell bug。
pub(super) fn line_ansi(line: &Line<'_>) -> String {
    fn fg_code(c: Color) -> Option<&'static str> {
        Some(match c {
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::Gray => "37",
            Color::DarkGray => "90",
            Color::LightRed => "91",
            Color::LightGreen => "92",
            Color::LightYellow => "93",
            Color::LightBlue => "94",
            Color::LightMagenta => "95",
            Color::LightCyan => "96",
            Color::White => "97",
            _ => return None,
        })
    }
    fn bg_code(c: Color) -> Option<String> {
        fg_code(c).map(|f| {
            let n: u8 = f.parse().unwrap_or(37);
            (n + 10).to_string()
        })
    }
    let mut out = String::new();
    for sp in &line.spans {
        let mut codes: Vec<String> = Vec::new();
        if sp.style.add_modifier.contains(Modifier::BOLD) {
            codes.push("1".to_string());
        }
        if let Some(c) = sp.style.fg.and_then(fg_code) {
            codes.push(c.to_string());
        }
        if let Some(c) = sp.style.bg.and_then(bg_code) {
            codes.push(c);
        }
        if codes.is_empty() {
            out.push_str(&sp.content);
        } else {
            out.push_str(&format!("\x1b[{}m{}\x1b[0m", codes.join(";"), sp.content));
        }
    }
    out
}
