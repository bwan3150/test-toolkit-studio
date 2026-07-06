// 【TUI 渲染状态】TuiModel：apply 把 UiEvent 转成彩色 Line 累积；handle_key/handle_paste
// 处理输入（含 /指令、choosing 列表、中途指导）。画屏在 view.rs。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::workflow::agent::runner::fmt::{brief, fmt_duration};
use crate::workflow::agent::ui::command::UiCommand;
use crate::workflow::agent::ui::event::*;

/// 方向键列表选择状态（设备选择等）：options 非空时进入。
pub(super) struct ChoiceState {
    pub(super) prompt: String,
    pub(super) options: Vec<String>,
    pub(super) selected: usize,
    /// true=列表末尾带内联输入行（选项 or 直接打字二选一，ask_user 带 options 用）
    pub(super) allow_free: bool,
    /// 内联输入行的当前内容
    pub(super) free_input: String,
}

/// TUI 渲染状态：apply 把 UiEvent 转成若干彩色 Line 累积；view 据此画屏。
pub(super) struct TuiModel {
    /// 滚动消息流（owned String 保证 'static）
    pub(super) lines: Vec<Line<'static>>,
    pub(super) phase: Option<(Phase, Option<u32>)>,
    pub(super) round: usize,
    pub(super) tok_up: i64,
    pub(super) tok_down: i64,
    /// 等用户回答的提问（Some=AI 在等纯文本输入）
    pub(super) awaiting: Option<String>,
    /// 方向键列表选择（Some=正在从候选里选，如设备选择）
    pub(super) choosing: Option<ChoiceState>,
    pub(super) input: String,
    /// 从底向上的滚动偏移（仅 follow=false 时生效）
    pub(super) scroll: u16,
    /// true=自动贴底显示最新
    pub(super) follow: bool,
    pub(super) finished: bool,
    pub(super) should_quit: bool,
    /// 最近一条「步骤进行中」行的下标；done 时原地替换成 ✓/✗（步骤保持单行，不另起一行）
    last_step_idx: Option<usize>,
    /// 输入框光标位置（字符索引，0..=input.chars().count()）
    pub(super) cursor: usize,
    /// 当前平台（从 SessionInfo 取，用于 Page 行的平台图标）
    platform: Option<String>,
    /// 当前 AI 配置 (模型, 供应商, 推理)——从 SessionInfo 存下，/model 查询用，不灌消息流
    ai_config: Option<(String, String, String)>,
}

impl TuiModel {
    pub(super) fn new() -> Self {
        Self {
            lines: Vec::new(),
            phase: None,
            round: 0,
            tok_up: 0,
            tok_down: 0,
            awaiting: None,
            choosing: None,
            input: String::new(),
            scroll: 0,
            follow: true,
            finished: false,
            should_quit: false,
            last_step_idx: None,
            cursor: 0,
            platform: None,
            ai_config: None,
        }
    }

    fn add_tokens(&mut self, t: Tokens) {
        self.tok_up += t.prompt;
        self.tok_down += t.completion;
    }

    /// 行尾暗灰 token 角标（与正文同 Line）
    fn tok_span(t: Tokens) -> Span<'static> {
        Span::styled(
            format!("  ↑{} ↓{}", t.prompt, t.completion),
            Style::default().fg(Color::DarkGray),
        )
    }

    fn push(&mut self, line: Line<'static>) {
        self.lines.push(line);
    }

    /// 一句话 → 单色整行
    fn push_colored(&mut self, text: String, color: Color) {
        self.push(Line::from(Span::styled(text, Style::default().fg(color))));
    }

    /// 块间留一个空行（幂等：上一行已空 / 开头 则不重复加）
    fn ensure_blank(&mut self) {
        if let Some(last) = self.lines.last() {
            if last.width() > 0 {
                self.lines.push(Line::from(""));
            }
        }
    }

    /// Page 的 ⎿ 子项行（暗灰缩进）
    fn push_sub(&mut self, text: String) {
        self.push(Line::from(Span::styled(
            format!("⎿ {}", text),
            Style::default().fg(Color::DarkGray),
        )));
    }

    /// 步骤完成：原地把「进行中」行替换成 ✓/✗ 结果行（步骤单行，不另起一行）
    fn replace_step_line(&mut self, line: Line<'static>) {
        if let Some(i) = self.last_step_idx.take() {
            if i < self.lines.len() {
                self.lines[i] = line;
                return;
            }
        }
        self.push(line);
    }

    /// 清空输入框（光标归零）
    fn clear_input(&mut self) {
        self.input.clear();
        self.cursor = 0;
    }

    /// 把可用 / 指令列表推进消息流（/help）
    fn push_help(&mut self) {
        self.ensure_blank();
        self.push_colored("可用指令：".to_string(), Color::Cyan);
        for (name, desc) in SLASH_COMMANDS {
            self.push_sub(format!("{}  {}", name, desc));
        }
    }

    /// 把一个 UiEvent 转成 1~N 行彩色 Line 累积进 lines
    pub(super) fn apply(&mut self, ev: UiEvent) {
        // Step（进行中/完成/失败）紧贴在触发它的「● AI 回复」下面、缩进显示，不另加块间空行；
        // 其余块级事件（● 回复 / 观察 / 阶段 …）之间统一留一个空行。
        let is_step = matches!(&ev, UiEvent::Step { .. });
        if !is_step {
            self.ensure_blank();
        }
        match ev {
            UiEvent::Phase { phase, n } => {
                // 阶段只更新顶栏状态（「探索中·第N轮」），不再往消息流里插 `━━ 探索 ━━` 标题——
                // 与顶栏重复、还把对话流切得支离破碎
                self.phase = Some((phase, n));
            }
            UiEvent::SessionInfo { device, platform, model, provider, reasoning } => {
                self.platform = Some(platform.clone());
                self.ai_config = Some((model, provider, reasoning)); // /model 查询用，不灌消息流
                // setup 选好的参数：一行即可（不再显示用例——用户的话已有输入回显）
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Green)),
                    Span::styled("准备就绪", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("  设备 {} · 平台 {}", device, platform),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            UiEvent::Page {
                round,
                struct_elements,
                ocr_added,
                ocr_failed,
                tabs,
                location,
                not_ready,
                ..
            } => {
                self.round = round;
                // 平台图标 + 「观察当前页面」标题 + ⎿ 分行（页面元素 / OCR / 标签页 / 所处）
                let icon = platform_icon(self.platform.as_deref());
                self.push(Line::from(Span::styled(
                    format!("{} 观察当前页面", icon),
                    Style::default().fg(Color::Gray),
                )));
                self.push_sub(format!("{} 页面元素", struct_elements));
                if ocr_failed {
                    self.push_sub("OCR 调用失败".to_string());
                } else if let Some(n) = ocr_added {
                    self.push_sub(format!("补充 {} OCR元素", n));
                }
                if tabs > 0 {
                    self.push_sub(format!("{} 标签页", tabs));
                }
                if let Some(loc) = location {
                    let loc = loc.trim();
                    if !loc.is_empty() {
                        self.push_sub(format!("所处 {}", brief(loc, 40)));
                    }
                }
                if let Some(nr) = not_ready {
                    self.push_sub(
                        match nr {
                            NotReady::SessionClosed => "会话已关闭（收尾）",
                            NotReady::NeedLaunch => "页面未就绪，待 launch",
                        }
                        .to_string(),
                    );
                }
            }
            UiEvent::OcrError { error, .. } => {
                self.push_colored(format!("! {}", brief(&error, 160)), Color::Red);
            }
            UiEvent::Stuck { message, .. } => {
                self.push_colored(format!("~ {}", message), Color::Yellow);
            }
            UiEvent::AgentThought { text, tokens, .. } => {
                self.add_tokens(tokens);
                // Explorer 子 agent 回复：● + 名字高亮（专属蓝色），句子用默认色
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Blue)),
                    Span::styled("Explorer ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                    Span::raw(brief(&text, 200)),
                    Self::tok_span(tokens),
                ]));
            }
            // 主 AI（编排官）：助手本体，纯文本渲染——无 ● 无名字无专属色，与子 agent 区分开。
            // 它是对话主体：多行回复**完整**渲染（不截成首行），token 角标跟在末行。
            UiEvent::Assistant { text, tokens } => {
                self.add_tokens(tokens);
                let mut body: Vec<&str> = text.lines().collect();
                while body.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    body.pop();
                }
                let n = body.len();
                for (i, l) in body.iter().enumerate() {
                    if i + 1 == n {
                        self.push(Line::from(vec![Span::raw(l.to_string()), Self::tok_span(tokens)]));
                    } else {
                        self.push(Line::from(Span::raw(l.to_string())));
                    }
                }
            }
            // 主 AI 的计划清单：每项一行，状态用符号+色（☐待办 ▶进行中 ☑完成，完成项划掉）。
            UiEvent::Todo { items } => {
                self.push(Line::from(Span::styled("  计划", Style::default().fg(Color::DarkGray))));
                for it in &items {
                    let (mark, color) = match it.status {
                        TodoStatus::Pending => ("☐", Color::DarkGray),
                        TodoStatus::InProgress => ("▶", Color::Yellow),
                        TodoStatus::Done => ("☑", Color::Green),
                    };
                    let text_style = if it.status == TodoStatus::Done {
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default()
                    };
                    self.push(Line::from(vec![
                        Span::styled(format!("    {} ", mark), Style::default().fg(color)),
                        Span::styled(it.text.clone(), text_style),
                    ]));
                }
            }
            UiEvent::SubAgent { kind, text, tokens, .. } => {
                self.add_tokens(tokens);
                // 子 agent 回复：● + 英文名高亮（agent 专属色），句子用默认色
                let color = subagent_color(kind);
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(color)),
                    Span::styled(format!("{} ", kind.label()), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::raw(brief(&text, 200)),
                    Self::tok_span(tokens),
                ]));
            }
            UiEvent::Step { step, state, preview, duration_ms, error, .. } => match state {
                // 进行中：`[ N] 动作 ...`（记下行号，done 时原地替换，不另起一行）
                StepState::Running => {
                    self.push(Line::from(vec![
                        Span::styled(format!("  [{:>2}] ", step), Style::default().fg(Color::DarkGray)),
                        Span::raw(preview),
                        Span::styled(" ...", Style::default().fg(Color::DarkGray)),
                    ]));
                    self.last_step_idx = Some(self.lines.len() - 1);
                }
                // 成功：原地替换为 `[ N] 动作 ... ✓ 耗时`
                StepState::Ok => {
                    let dur = duration_ms.map(fmt_duration).unwrap_or_default();
                    self.replace_step_line(Line::from(vec![
                        Span::styled(format!("  [{:>2}] ", step), Style::default().fg(Color::DarkGray)),
                        Span::raw(preview),
                        Span::styled(" ... ", Style::default().fg(Color::DarkGray)),
                        Span::styled("✓", Style::default().fg(Color::Green)),
                        Span::styled(format!(" {}", dur), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                // 失败：原地替换为 `[ N] 动作 ... ✗ 错误`
                StepState::Fail => {
                    let e = error.unwrap_or_default();
                    self.replace_step_line(Line::from(vec![
                        Span::styled(format!("  [{:>2}] ", step), Style::default().fg(Color::DarkGray)),
                        Span::raw(preview),
                        Span::styled(" ... ", Style::default().fg(Color::DarkGray)),
                        Span::styled("✗", Style::default().fg(Color::Red)),
                        Span::styled(format!(" {}", brief(&e, 120)), Style::default().fg(Color::Red)),
                    ]));
                }
            },
            UiEvent::Rename { old, new } => {
                self.push_colored(format!("✎ 改名：{} → {}", old, new), Color::Magenta);
            }
            UiEvent::AutoStop { reason } => {
                self.push_colored(format!("■ {}", reason), Color::Red);
            }
            UiEvent::ExitingNote { message } => {
                self.push_colored(message, Color::Yellow);
            }
            UiEvent::Notice { level, text } => {
                self.push_colored(text, level_color(level));
            }
            UiEvent::AwaitingInput { question, options, free_input, .. } => {
                if !options.is_empty() {
                    // 候选项 → choosing 模式（方向键列表）；问题推进消息流留痕（选择结果提交时回显 ❯）
                    self.ensure_blank();
                    self.push(Line::from(vec![
                        Span::styled("? ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(question.clone(), Style::default().fg(Color::Cyan)),
                    ]));
                    self.choosing = Some(ChoiceState {
                        prompt: question,
                        options,
                        selected: 0,
                        allow_free: free_input,
                        free_input: String::new(),
                    });
                    self.follow = true;
                    self.scroll = 0;
                } else if question.trim().is_empty() {
                    // 空问题 = 纯"等输入"状态（编排官 REPL 每轮的等待）——不灌消息流，免得刷屏
                    self.awaiting = Some(question);
                    self.cursor = 0;
                    self.follow = true;
                    self.scroll = 0;
                } else {
                    // 纯文本输入：问题**推进消息流留痕**（? 前缀）——ask_user 问了什么必须能回看，
                    // 只显示在输入框标题的话，答完就消失了；输入框标题同时也显示。
                    self.ensure_blank();
                    self.push(Line::from(vec![
                        Span::styled("? ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(question.clone(), Style::default().fg(Color::Cyan)),
                    ]));
                    self.awaiting = Some(question);
                    self.cursor = 0;
                    self.follow = true;
                    self.scroll = 0;
                }
            }
            UiEvent::GuidanceAccepted { .. } => {
                // 用户的话已在发送时本地回显；这里只确认「已采纳」（前后空行由 ensure_blank 保证）
                self.push_colored("↳ 已采纳指导".to_string(), Color::Magenta);
            }
            UiEvent::ScriptGenerated { name, steps, success } => {
                if success {
                    self.push_colored(
                        format!("✓ 脚本生成完毕：{}（{} 步）", name, steps),
                        Color::Green,
                    );
                } else {
                    self.push_colored(
                        format!("⚠ 探索未达成，脚本不完整：{}（{} 步）", name, steps),
                        Color::Yellow,
                    );
                }
            }
            UiEvent::Summary { explore, diagnose, verify, reason, model, tokens } => {
                self.push(Line::from(""));
                self.push(Line::from(Span::styled(
                    "─ 结果 ──────────────────────────────",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                self.push_status("探索", &explore);
                self.push_status("诊断", &diagnose);
                self.push_status("验证", &verify);
                self.push(Line::from(vec![
                    Span::styled("依据  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(brief(&reason, 200)),
                ]));
                self.push(Line::from(vec![
                    Span::styled("模型  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(model),
                ]));
                self.push(Line::from(Span::styled(
                    format!(
                        "Token ↑{} ↓{} · 合计 {}",
                        tokens.prompt,
                        tokens.completion,
                        tokens.prompt + tokens.completion
                    ),
                    Style::default().fg(Color::DarkGray),
                )));
                self.push(Line::from(Span::styled(
                    "─────────────────────────────────────",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            }
            UiEvent::Elements { committed, items, committed_to_lib } => {
                self.push(Line::from(""));
                self.push(Line::from(Span::styled(
                    "─ 元素库更新 ────────────────────────",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                if !committed_to_lib {
                    self.push_colored(
                        "未稳定通过——脚本未保存；本次元素只存在临时库（随运行目录丢弃），正式元素库未改动"
                            .to_string(),
                        Color::Yellow,
                    );
                } else if items.is_empty() {
                    self.push(Line::from(vec![
                        Span::styled("提交  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("（无新元素）", Style::default().fg(Color::DarkGray)),
                    ]));
                } else {
                    let line_for = |it: &ElementItem| match &it.desc {
                        Some(d) => format!("{} · {}", it.name, brief(d, 80)),
                        None => it.name.clone(),
                    };
                    self.push(Line::from(vec![
                        Span::styled(
                            format!("提交 {} 个  ", committed),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(line_for(&items[0]), Style::default().fg(Color::Green)),
                    ]));
                    for it in &items[1..] {
                        self.push(Line::from(Span::styled(
                            format!("           {}", line_for(it)),
                            Style::default().fg(Color::Green),
                        )));
                    }
                    self.push_colored(
                        "（最终脚本用到的元素已写入正式库，desc 据实际作用生成，请人工二次审核）"
                            .to_string(),
                        Color::DarkGray,
                    );
                }
                self.push(Line::from(Span::styled(
                    "─────────────────────────────────────",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            }
            UiEvent::Done { .. } => {
                self.finished = true;
                self.push(Line::from(""));
                self.push_colored("（已结束，按 q 退出）".to_string(), Color::DarkGray);
            }
        }
    }

    /// 结果框里的一行状态（标签暗灰 + 文字按 level 上色）
    fn push_status(&mut self, label: &str, st: &StatusLine) {
        self.push(Line::from(vec![
            Span::styled(format!("{}  ", label), Style::default().fg(Color::DarkGray)),
            Span::styled(st.text.clone(), Style::default().fg(level_color(st.level))),
        ]));
    }

    /// 粘贴（bracketed paste）：整段插到光标处——换行转空格，绝不当 Enter 误提交半截内容
    pub(super) fn handle_paste(&mut self, s: &str) {
        if let Some(ch) = self.choosing.as_mut() {
            // 带内联输入行的选择：**选中输入行时**粘贴进它（换行转空格）；其余情况粘贴无意义
            if ch.allow_free && ch.selected == ch.options.len() {
                let cleaned: String = s.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
                ch.free_input.push_str(&cleaned);
            }
            return;
        }
        let cleaned: String = s
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        if cleaned.is_empty() {
            return;
        }
        let mut chars: Vec<char> = self.input.chars().collect();
        let at = self.cursor.min(chars.len());
        for (k, c) in cleaned.chars().enumerate() {
            chars.insert(at + k, c);
        }
        self.input = chars.into_iter().collect();
        self.cursor = at + cleaned.chars().count();
    }

    /// 键盘动作 → UiCommand / 滚动 / 退出
    pub(super) fn handle_key(&mut self, k: KeyEvent, tx: &UnboundedSender<UiCommand>) {
        // Ctrl+C：强制退出兜底（任何状态都能跳出，防卡死）
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            if !self.finished {
                let _ = tx.send(UiCommand::Abort);
            }
            self.should_quit = true;
            return;
        }
        // Esc：停止当前探索（软停，可继续输入指导让 AI 接着跑）；choosing 时=放弃选择并退出
        if k.code == KeyCode::Esc {
            if self.choosing.is_some() {
                let _ = tx.send(UiCommand::Abort);
                self.choosing = None;
            } else if !self.finished && self.awaiting.is_none() {
                // 立即请求中断当前动作（滚动查找/回放等长动作轮询此标志会立刻停），再发 Pause 进暂停
                crate::utils::interrupt::request_pause();
                let _ = tx.send(UiCommand::Pause);
            }
            return;
        }

        // ---- choosing 模式：方向键列表选择（allow_free 时末尾带内联输入行），优先于一般按键 ----
        if let Some(ch) = self.choosing.as_mut() {
            let n = ch.options.len();
            // 可选行数：选项 0..n-1；allow_free 时第 n 行 = 内联输入行
            let last = if ch.allow_free { n } else { n.saturating_sub(1) };
            match k.code {
                KeyCode::Up => {
                    ch.selected = ch.selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if ch.selected < last {
                        ch.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    if ch.allow_free && ch.selected == n {
                        // 内联输入行提交（空内容不提交）
                        let t = ch.free_input.trim().to_string();
                        if t.is_empty() {
                            return;
                        }
                        let _ = tx.send(UiCommand::Answer { text: format!("text:{}", t) });
                        self.choosing = None;
                        self.push(Line::from(vec![
                            Span::styled("❯ ", Style::default().fg(Color::Blue)),
                            Span::raw(t),
                        ]));
                    } else {
                        let idx = ch.selected;
                        let label = ch.options.get(idx).map(|o| o.split('\t').last().unwrap_or(o).to_string()).unwrap_or_default();
                        // free 模式走 pick:N 协议（与 text: 区分）；旧模式（设备选择/授权）保持裸下标
                        let text = if ch.allow_free { format!("pick:{}", idx) } else { idx.to_string() };
                        let _ = tx.send(UiCommand::Answer { text });
                        self.choosing = None;
                        // 回显选中项（与文字输入的 ❯ 回显一致，问答成对留痕）
                        self.push(Line::from(vec![
                            Span::styled("❯ ", Style::default().fg(Color::Blue)),
                            Span::raw(label),
                        ]));
                    }
                }
                // 内联输入行编辑：**仅当选中它时**打字才生效（其余行保持纯上下选择）
                KeyCode::Backspace if ch.allow_free && ch.selected == n => {
                    ch.free_input.pop();
                }
                // 数字键 1..9 直接选对应项并提交（选中输入行时数字是内容，走上面 Char 分支之前先排除）
                KeyCode::Char(c @ '1'..='9') if !(ch.allow_free && ch.selected == n) => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < n {
                        let label = ch.options.get(idx).map(|o| o.split('\t').last().unwrap_or(o).to_string()).unwrap_or_default();
                        let text = if ch.allow_free { format!("pick:{}", idx) } else { idx.to_string() };
                        let _ = tx.send(UiCommand::Answer { text });
                        self.choosing = None;
                        self.push(Line::from(vec![
                            Span::styled("❯ ", Style::default().fg(Color::Blue)),
                            Span::raw(label),
                        ]));
                    }
                }
                KeyCode::Char(c) if ch.allow_free && ch.selected == n && !k.modifiers.contains(KeyModifiers::CONTROL) => {
                    ch.free_input.push(c);
                }
                _ => {}
            }
            return;
        }

        match k.code {
            // ↑↓ / PageUp·PageDown 滚动消息区（不再用 j/k，避免吃掉字母输入）
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_add(1);
                self.follow = false;
            }
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_sub(1);
                if self.scroll == 0 {
                    self.follow = true;
                }
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_add(10);
                self.follow = false;
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_sub(10);
                if self.scroll == 0 {
                    self.follow = true;
                }
            }
            // ←→ Home End 移动输入光标
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                if self.cursor < self.input.chars().count() {
                    self.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                self.cursor = self.input.chars().count();
            }
            // Tab：/ 指令补全到第一个匹配项
            KeyCode::Tab => {
                if self.input.starts_with('/') {
                    if let Some((name, _)) = slash_matches(&self.input).first() {
                        self.input = name.to_string();
                        self.cursor = self.input.chars().count();
                    }
                }
            }
            KeyCode::Enter => {
                let cmd = self.input.trim().to_string();
                if cmd.starts_with('/') {
                    // / 指令体系
                    match cmd.as_str() {
                        "/exit" | "/quit" => {
                            if !self.finished {
                                let _ = tx.send(UiCommand::Abort);
                            }
                            self.should_quit = true;
                        }
                        "/stop" => {
                            // 软停当前探索（等同 Esc）
                            if !self.finished && self.awaiting.is_none() {
                                let _ = tx.send(UiCommand::Pause);
                            }
                        }
                        "/model" => {
                            self.ensure_blank();
                            match &self.ai_config {
                                Some((model, provider, reasoning)) => self.push_colored(
                                    format!("模型 {} · 供应商 {} · 推理 {}（运行中切换暂未支持：改 config 的 [ai] 段或 --ai-* 参数后重启生效）", model, provider, reasoning),
                                    Color::Cyan,
                                ),
                                None => self.push_colored("尚未收到配置信息（会话还没开始）".to_string(), Color::Yellow),
                            }
                        }
                        "/help" => self.push_help(),
                        other => self.push_colored(
                            format!("未知指令：{}（输入 / 看可用指令）", other),
                            Color::Yellow,
                        ),
                    }
                    self.clear_input();
                } else if !self.input.is_empty() {
                    let text = self.input.clone();
                    // 本地立即回显用户输入（让用户知道已发送、不必连发）
                    self.ensure_blank();
                    self.push(Line::from(vec![
                        Span::styled("❯ ", Style::default().fg(Color::Blue)),
                        Span::raw(text.clone()),
                    ]));
                    if self.awaiting.is_some() {
                        let _ = tx.send(UiCommand::Answer { text });
                        self.awaiting = None;
                    } else {
                        // 运行中（非等输入）打字 + Enter = 中途指导
                        let _ = tx.send(UiCommand::Guidance { text });
                    }
                    self.clear_input();
                }
            }
            // 在光标处删除/插入（支持中文按字符）
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let mut chars: Vec<char> = self.input.chars().collect();
                    chars.remove(self.cursor - 1);
                    self.input = chars.into_iter().collect();
                    self.cursor -= 1;
                }
            }
            KeyCode::Delete => {
                let mut chars: Vec<char> = self.input.chars().collect();
                if self.cursor < chars.len() {
                    chars.remove(self.cursor);
                    self.input = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut chars: Vec<char> = self.input.chars().collect();
                let at = self.cursor.min(chars.len());
                chars.insert(at, c);
                self.input = chars.into_iter().collect();
                self.cursor = at + 1;
            }
            _ => {}
        }
    }
}

/// 可用的 / 指令（输入 / 弹提示，Tab 补全）
pub(super) const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/exit", "退出 TUI"),
    ("/stop", "停止当前探索（可继续输入指导）"),
    ("/model", "显示当前模型/供应商/推理配置"),
    ("/help", "显示可用指令"),
];

/// 按 input 前缀过滤匹配的 / 指令
pub(super) fn slash_matches(input: &str) -> Vec<&'static (&'static str, &'static str)> {
    let q = input.trim();
    SLASH_COMMANDS.iter().filter(|(n, _)| n.starts_with(q)).collect()
}

/// 平台图标（Nerd Font）：iOS / Android / Web
fn platform_icon(platform: Option<&str>) -> &'static str {
    match platform.map(|s| s.to_lowercase()) {
        Some(ref s) if s.contains("ios") => "󰀵",
        Some(ref s) if s.contains("android") => "󰀲",
        Some(ref s) if s.contains("web") => "󰖟",
        _ => "·",
    }
}

/// 子 agent 专属点色（● 标注；不同 agent 不同色，便于一眼区分谁在说话）
fn subagent_color(kind: SubAgent) -> Color {
    match kind {
        SubAgent::Asserter => Color::Cyan,
        SubAgent::Supervisor => Color::Yellow,
        SubAgent::Reflector => Color::Magenta,
        SubAgent::Doctor => Color::LightBlue,
        SubAgent::Optimizer => Color::Green,
    }
}

/// 语义色级 → ratatui 颜色
fn level_color(l: Level) -> Color {
    match l {
        Level::Info => Color::Cyan,
        Level::Ok => Color::Green,
        Level::Warn => Color::Yellow,
        Level::Err => Color::Red,
        Level::Dim => Color::DarkGray,
    }
}
