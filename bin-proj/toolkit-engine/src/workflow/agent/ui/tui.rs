// 【TUI 前端】ratatui 全屏交互（真 TTY 时用）。
//
// 引擎跑在 tokio 侧、TUI 跑在独立 std::thread 独占终端，两条 unbounded channel 桥接：
//   events_tx (引擎 emit → TUI 渲染)、commands_tx (TUI 键盘 → 引擎 drain)。
// emit 永不阻塞引擎（unbounded send）；drain/await_answer 同步块内 try_recv，绝不跨 await 持 std Mutex。
// 退出靠用户在 TUI 里按 q（结束后）/ Ctrl+C（中断→再按强制退出）；run_tui 所有返回路径都恢复终端。

use std::io::Stdout;
use std::sync::Mutex;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::super::runner::flow::{brief, fmt_duration};
use super::command::UiCommand;
use super::event::*;
use super::Frontend;

type Backend = CrosstermBackend<Stdout>;

/// 引擎在 tokio 侧持有的 TUI 句柄；TUI 跑在独立 std::thread。
pub struct TuiFrontend {
    events_tx: UnboundedSender<UiEvent>,
    commands_rx: Mutex<UnboundedReceiver<UiCommand>>,
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl TuiFrontend {
    /// 建两条 channel，起独立线程跑 run_tui，返回句柄。
    pub fn spawn() -> anyhow::Result<Self> {
        let (events_tx, events_rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
        let (commands_tx, commands_rx) = tokio::sync::mpsc::unbounded_channel::<UiCommand>();
        let handle = std::thread::spawn(move || {
            let _ = run_tui(events_rx, commands_tx);
        });
        Ok(Self {
            events_tx,
            commands_rx: Mutex::new(commands_rx),
            handle: Mutex::new(Some(handle)),
        })
    }
}

#[async_trait::async_trait]
impl Frontend for TuiFrontend {
    fn emit(&self, ev: UiEvent) {
        // unbounded：永不阻塞引擎；线程已退则丢弃
        let _ = self.events_tx.send(ev);
    }

    fn drain_commands(&self) -> Vec<UiCommand> {
        let mut cmds = Vec::new();
        if let Ok(mut rx) = self.commands_rx.lock() {
            while let Ok(c) = rx.try_recv() {
                cmds.push(c);
            }
        }
        cmds
    }

    async fn await_answer(&self, round: usize, question: String) -> Option<String> {
        // 先告知 TUI「正在等回答」（高亮输入框、回显提问）
        self.emit(UiEvent::AwaitingInput { round, question, options: Vec::new() });
        // 轮询：每 50ms 取一次命令。不可跨 await 持 std Mutex——锁只在同步块内用。
        loop {
            {
                let mut rx = match self.commands_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => return Some(text),
                        UiCommand::Abort => return None,
                        // Guidance/Pause/Resume：等回答期间忽略
                        _ => {}
                    }
                }
            } // 锁在此释放，再 await
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 从候选里选一个（设备选择等）：TUI 渲染成方向键可选列表。
    /// emit AwaitingInput{options 非空} → model 进 choosing 模式；
    /// 用户用 ↑↓ 选、Enter 提交 → handle_key 发 Answer{text=选中下标字符串}。
    /// 这里轮询 commands_rx：Answer 里是下标 → parse 成 usize 返回；Abort → None。
    async fn await_choice(&self, prompt: String, options: Vec<String>) -> Option<usize> {
        self.emit(UiEvent::AwaitingInput {
            round: 0,
            question: prompt,
            options: options.clone(),
        });
        // 与 await_answer 同模式：同步块取锁 try_recv，绝不跨 await 持 std Mutex。
        loop {
            {
                let mut rx = match self.commands_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => return text.trim().parse::<usize>().ok(),
                        UiCommand::Abort => return None,
                        // Guidance/Pause/Resume：选择期间忽略
                        _ => {}
                    }
                }
            } // 锁在此释放，再 await
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn shutdown(self: Box<Self>) {
        // 取出线程句柄，spawn_blocking 等用户在 TUI 里按 q 退出后线程结束
        let handle = self.handle.lock().ok().and_then(|mut h| h.take());
        if let Some(h) = handle {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = h.join();
            })
            .await;
        }
    }
}

// ============================ 终端生命周期 ============================

/// 独立线程入口：装 panic hook、进终端、跑主循环、无论结果都恢复终端。
fn run_tui(
    mut events_rx: UnboundedReceiver<UiEvent>,
    commands_tx: UnboundedSender<UiCommand>,
) -> std::io::Result<()> {
    install_panic_hook();
    let mut terminal = enter_terminal()?;
    let mut model = TuiModel::new();
    let res = run_loop(&mut terminal, &mut events_rx, &commands_tx, &mut model);
    // 无论 res 如何都要恢复终端
    let _ = leave_terminal(&mut terminal);
    res
}

/// raw mode + 备用屏幕 + 隐藏光标
fn enter_terminal() -> std::io::Result<Terminal<Backend>> {
    enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen, Hide)?;
    Terminal::new(CrosstermBackend::new(out))
}

/// 退出备用屏幕 + 显示光标 + 关 raw mode（CrosstermBackend 实现了 Write）
fn leave_terminal(terminal: &mut Terminal<Backend>) -> std::io::Result<()> {
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    let _ = terminal.show_cursor();
    Ok(())
}

/// panic 时也恢复终端（否则用户终端卡在 raw/alt-screen）
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);
        prev(info);
    }));
}

/// 主循环：抽干引擎事件 → 处理键盘 → 重绘。靠用户按 q 退出。
fn run_loop(
    terminal: &mut Terminal<Backend>,
    events_rx: &mut UnboundedReceiver<UiEvent>,
    commands_tx: &UnboundedSender<UiCommand>,
    model: &mut TuiModel,
) -> std::io::Result<()> {
    loop {
        // 抽干引擎事件（断开返回 Disconnected，正常忽略——靠用户按 q 退出）
        while let Ok(ev) = events_rx.try_recv() {
            model.apply(ev);
        }
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    model.handle_key(k, commands_tx);
                }
            }
        }
        terminal.draw(|f| view(f, model))?;
        if model.should_quit {
            break;
        }
    }
    Ok(())
}

// ============================ 渲染状态 ============================

/// 方向键列表选择状态（设备选择等）：options 非空时进入。
struct ChoiceState {
    prompt: String,
    options: Vec<String>,
    selected: usize,
}

/// TUI 渲染状态：apply 把 UiEvent 转成若干彩色 Line 累积；view 据此画屏。
struct TuiModel {
    /// 滚动消息流（owned String 保证 'static）
    lines: Vec<Line<'static>>,
    phase: Option<(Phase, Option<u32>)>,
    round: usize,
    tok_up: i64,
    tok_down: i64,
    /// 等用户回答的提问（Some=AI 在等纯文本输入）
    awaiting: Option<String>,
    /// 方向键列表选择（Some=正在从候选里选，如设备选择）
    choosing: Option<ChoiceState>,
    input: String,
    /// 从底向上的滚动偏移（仅 follow=false 时生效）
    scroll: u16,
    /// true=自动贴底显示最新
    follow: bool,
    finished: bool,
    should_quit: bool,
    /// 最近一条「步骤进行中」行的下标；done 时原地替换成 ✓/✗（步骤保持单行，不另起一行）
    last_step_idx: Option<usize>,
}

impl TuiModel {
    fn new() -> Self {
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

    /// 把一个 UiEvent 转成 1~N 行彩色 Line 累积进 lines
    fn apply(&mut self, ev: UiEvent) {
        // Step（进行中/完成/失败）紧贴在触发它的「● AI 回复」下面、缩进显示，不另加块间空行；
        // 其余块级事件（● 回复 / 观察 / 阶段 …）之间统一留一个空行。
        let is_step = matches!(&ev, UiEvent::Step { .. });
        if !is_step {
            self.ensure_blank();
        }
        match ev {
            UiEvent::Phase { phase, n } => {
                self.phase = Some((phase, n));
                let title = match n {
                    Some(k) => format!("━━ {} #{} ━━", phase.label(), k),
                    None => format!("━━ {} ━━", phase.label()),
                };
                // 粗体下划线标题（块间空行由 apply 统一处理）
                self.push(Line::from(Span::styled(
                    title,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                )));
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
                // 「观察当前页面」标题 + ⎿ 分行列：页面元素 / OCR 补充 / 标签页 / 所处
                self.push_colored("观察当前页面".to_string(), Color::Gray);
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
                // 探索主 agent 的回复：● 白点 + 句子（单独一行，指令执行缩进在下面）
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::White)),
                    Span::raw(brief(&text, 200)),
                    Self::tok_span(tokens),
                ]));
            }
            UiEvent::SubAgent { kind, level, text, tokens } => {
                self.add_tokens(tokens);
                // 子 agent 回复：● 点用该 agent 专属色 + 名称 + 句子
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(subagent_color(kind))),
                    Span::styled(format!("{} ", kind.label()), Style::default().fg(Color::DarkGray)),
                    Span::styled(brief(&text, 200), Style::default().fg(level_color(level))),
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
            UiEvent::AwaitingInput { question, options, .. } => {
                if !options.is_empty() {
                    // 候选项 → 进 choosing 模式（方向键列表），不进文本输入。
                    // 提示回显进消息流，列表本身在输入区渲染。
                    self.push_colored(format!("? {}", question), Color::Cyan);
                    self.choosing = Some(ChoiceState {
                        prompt: question,
                        options,
                        selected: 0,
                    });
                    self.follow = true;
                    self.scroll = 0;
                } else {
                    // 纯文本输入：question 可能多行——逐行渲染，避免挤成一行
                    let mut iter = question.lines();
                    if let Some(first) = iter.next() {
                        self.push_colored(format!("AI 提问：{}", first), Color::Cyan);
                    }
                    for line in iter {
                        self.push_colored(format!("  {}", line), Color::Cyan);
                    }
                    self.awaiting = Some(question);
                    self.follow = true;
                    self.scroll = 0;
                }
            }
            UiEvent::GuidanceAccepted { text } => {
                self.push_colored(
                    format!("↳ 已采纳指导：{}", brief(&text, 200)),
                    Color::Magenta,
                );
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

    /// 键盘动作 → UiCommand / 滚动 / 退出
    fn handle_key(&mut self, k: KeyEvent, tx: &UnboundedSender<UiCommand>) {
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
                let _ = tx.send(UiCommand::Pause);
            }
            return;
        }

        // ---- choosing 模式：方向键列表选择，优先于一般按键 ----
        if let Some(ch) = self.choosing.as_mut() {
            let n = ch.options.len();
            match k.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    ch.selected = ch.selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if ch.selected + 1 < n {
                        ch.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    let _ = tx.send(UiCommand::Answer {
                        text: ch.selected.to_string(),
                    });
                    self.choosing = None;
                }
                // 数字键 1..9 直接选对应项并提交（增强）
                KeyCode::Char(c @ '1'..='9') => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < n {
                        let _ = tx.send(UiCommand::Answer { text: idx.to_string() });
                        self.choosing = None;
                    }
                }
                _ => {}
            }
            return;
        }

        match k.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_add(1);
                self.follow = false;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::End => {
                self.follow = true;
                self.scroll = 0;
            }
            KeyCode::Enter => {
                let cmd = self.input.trim();
                if cmd == "/exit" || cmd == "/quit" {
                    // 一致做法：输入 /exit 退出 TUI（运行中先终止引擎）
                    if !self.finished {
                        let _ = tx.send(UiCommand::Abort);
                    }
                    self.should_quit = true;
                    self.input.clear();
                } else if !self.input.is_empty() {
                    let text = self.input.clone();
                    if self.awaiting.is_some() {
                        let _ = tx.send(UiCommand::Answer { text });
                        self.awaiting = None;
                    } else {
                        // 运行中（非等输入）打字 + Enter = 中途指导
                        let _ = tx.send(UiCommand::Guidance { text });
                    }
                    self.input.clear();
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(c);
            }
            _ => {}
        }
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

// ============================ 视图 ============================

/// 四段布局：顶栏(1) / 消息区(min) / 输入框(动态) / 底栏(1)
fn view(frame: &mut Frame, model: &TuiModel) {
    // choosing 模式时输入区要容纳 prompt + 选项列表，按需放大；否则固定 3 行单行输入框。
    let input_h: u16 = match model.choosing.as_ref() {
        // 边框 2 + prompt 1 + 每项 1，封顶 12 行免吃满屏
        Some(ch) => (ch.options.len() as u16 + 3).clamp(4, 12),
        None => 3,
    };
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .split(frame.area());

    // ---- 顶栏：阶段 · 轮次 · token · 用时（反色粗体）----
    let phase_label = model
        .phase
        .map(|(p, n)| match n {
            Some(k) => format!("{} #{}", p.label(), k),
            None => p.label().to_string(),
        })
        .unwrap_or_else(|| "准备中".to_string());
    let top = format!(" {} · 第{}轮 ", phase_label, model.round);
    frame.render_widget(
        Paragraph::new(Line::from(top)).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );

    // ---- 消息区：自动换行 + 准确贴底 ----
    // ratatui 0.29 的 Paragraph::line_count 是 unstable 私有方法，不能用；
    // 退而按宽度逐行估算 wrap 后行数累加（每条按显示宽度 ceil 折行），比按 lines.len() 估算准，
    // 长文本折多行也能正确贴底，最新内容始终可见。
    let msg_area = chunks[1];
    let height = msg_area.height;
    let width = msg_area.width.max(1);
    let total_wrapped: u16 = model
        .lines
        .iter()
        .map(|l| {
            let w = l.width() as u16;
            // 空行算 1 行；否则 ceil(w / width)
            (w / width + if w % width != 0 { 1 } else { 0 }).max(1)
        })
        .sum();
    let max_top = total_wrapped.saturating_sub(height);
    let y = if model.follow {
        max_top
    } else {
        max_top.saturating_sub(model.scroll)
    };
    frame.render_widget(
        Paragraph::new(Text::from(model.lines.clone()))
            .wrap(Wrap { trim: false })
            .scroll((y, 0)),
        msg_area,
    );

    // ---- 输入区：choosing 列表 / awaiting 文本输入 / 普通指导 ----
    let input_area = chunks[2];
    if let Some(ch) = model.choosing.as_ref() {
        render_choice(frame, input_area, ch);
    } else {
        render_input(frame, input_area, model);
    }

    // ---- 底栏：快捷键提示（暗灰，随状态变化）----
    let hint = if model.choosing.is_some() {
        "↑↓ 选择 · Enter 确认 · 1-9 直选 · Esc 放弃"
    } else if model.awaiting.is_some() {
        "Enter 提交 · 输入指导让 AI 继续 · /exit 退出"
    } else if model.finished {
        "输入 /exit 退出"
    } else {
        "Esc 停止 · 打字+Enter 指导 · ↑↓ 滚动 · End 贴底 · /exit 退出"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}

/// choosing 模式：在输入区渲染 prompt + 高亮选项列表（方向键选择）。
fn render_choice(frame: &mut Frame, area: ratatui::layout::Rect, ch: &ChoiceState) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        ch.prompt.clone(),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    for (i, opt) in ch.options.iter().enumerate() {
        if i == ch.selected {
            // 选中项：反白高亮 + ▶ 前导
            lines.push(Line::from(Span::styled(
                format!("▶ {}", opt),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {}", opt),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title("选择设备（↑↓ 选择 · Enter 确认）")
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ),
        area,
    );
}

/// 普通/awaiting 文本输入框 + 可见光标。
fn render_input(frame: &mut Frame, area: ratatui::layout::Rect, model: &TuiModel) {
    let (title, border_style) = if let Some(q) = model.awaiting.as_ref() {
        // setup 用例输入更像「发消息」；其它 ask_user 用「回答」措辞。
        let title = if is_case_prompt(q) {
            "描述你要测什么（Enter 发送）"
        } else {
            "AI 在等你回答（Enter 提交）"
        };
        (
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        ("指导（Enter 发送）", Style::default().fg(Color::DarkGray))
    };
    // 累计 token 放输入框右下角（边框底部右对齐），不再占顶栏
    let toks = Line::from(format!("↑{} ↓{}", model.tok_up, model.tok_down))
        .right_aligned()
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(
        Paragraph::new(format!("> {}", model.input)).block(
            Block::bordered()
                .title(title)
                .title_bottom(toks)
                .border_style(border_style),
        ),
        area,
    );
    // 光标定位到输入文本末尾：边框内 x = 1（边框）+ "> "(2) + 输入显示宽度；y = 边框内行。
    let input_w = Span::raw(model.input.as_str()).width() as u16;
    let cursor_x = area.x + 1 + 2 + input_w;
    let cursor_y = area.y + 1;
    // 夹在区域内，避免越界
    let cx = cursor_x.min(area.x + area.width.saturating_sub(1));
    frame.set_cursor_position(Position::new(cx, cursor_y));
}

/// 粗略判断 awaiting 的提问是不是「setup 用例输入」（用于切换输入框措辞）。
fn is_case_prompt(q: &str) -> bool {
    q.contains("测") || q.contains("用例") || q.contains("describe") || q.contains("test")
}
