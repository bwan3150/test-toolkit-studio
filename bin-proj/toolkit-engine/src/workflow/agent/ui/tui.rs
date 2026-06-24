// 【TUI 前端】ratatui 全屏交互（真 TTY 时用）。
//
// 引擎跑在 tokio 侧、TUI 跑在独立 std::thread 独占终端，两条 unbounded channel 桥接：
//   events_tx (引擎 emit → TUI 渲染)、commands_tx (TUI 键盘 → 引擎 drain)。
// emit 永不阻塞引擎（unbounded send）；drain/await_answer 同步块内 try_recv，绝不跨 await 持 std Mutex。
// 退出靠用户在 TUI 里按 q（结束后）/ Ctrl+C（中断→再按强制退出）；run_tui 所有返回路径都恢复终端。

use std::io::Stdout;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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
        self.emit(UiEvent::AwaitingInput { round, question });
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

/// TUI 渲染状态：apply 把 UiEvent 转成若干彩色 Line 累积；view 据此画屏。
struct TuiModel {
    /// 滚动消息流（owned String 保证 'static）
    lines: Vec<Line<'static>>,
    phase: Option<(Phase, Option<u32>)>,
    round: usize,
    tok_up: i64,
    tok_down: i64,
    started: Instant,
    /// 等用户回答的提问（Some=AI 在等输入）
    awaiting: Option<String>,
    input: String,
    /// 从底向上的滚动偏移（仅 follow=false 时生效）
    scroll: u16,
    /// true=自动贴底显示最新
    follow: bool,
    finished: bool,
    should_quit: bool,
    abort_sent: bool,
}

impl TuiModel {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            phase: None,
            round: 0,
            tok_up: 0,
            tok_down: 0,
            started: Instant::now(),
            awaiting: None,
            input: String::new(),
            scroll: 0,
            follow: true,
            finished: false,
            should_quit: false,
            abort_sent: false,
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

    /// 把一个 UiEvent 转成 1~N 行彩色 Line 累积进 lines
    fn apply(&mut self, ev: UiEvent) {
        match ev {
            UiEvent::Phase { phase, n } => {
                self.phase = Some((phase, n));
                let title = match n {
                    Some(k) => format!("━━ {} #{} ━━", phase.label(), k),
                    None => format!("━━ {} ━━", phase.label()),
                };
                // 空行分隔 + 粗体下划线标题
                self.push(Line::from(""));
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
                not_ready,
                ..
            } => {
                self.round = round;
                let mut parts = vec![format!("{} 元素", struct_elements)];
                if ocr_failed {
                    parts.push("OCR✗".to_string());
                } else if let Some(n) = ocr_added {
                    parts.push(format!("+{} OCR", n));
                }
                if tabs > 0 {
                    parts.push(format!("{} 标签", tabs));
                }
                if let Some(nr) = not_ready {
                    parts.push(
                        match nr {
                            NotReady::SessionClosed => "会话已关闭（收尾）",
                            NotReady::NeedLaunch => "待 launch",
                        }
                        .to_string(),
                    );
                }
                self.push_colored(format!("· {}", parts.join(" · ")), Color::DarkGray);
            }
            UiEvent::OcrError { error, .. } => {
                self.push_colored(format!("! {}", brief(&error, 160)), Color::Red);
            }
            UiEvent::Stuck { message, .. } => {
                self.push_colored(format!("~ {}", message), Color::Yellow);
            }
            UiEvent::AgentThought { text, tokens, .. } => {
                self.add_tokens(tokens);
                self.push(Line::from(vec![
                    Span::raw(brief(&text, 200)),
                    Self::tok_span(tokens),
                ]));
            }
            UiEvent::SubAgent { kind, level, text, tokens } => {
                self.add_tokens(tokens);
                self.push(Line::from(vec![
                    Span::styled(
                        format!("└ {} ", kind.label()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(brief(&text, 200), Style::default().fg(level_color(level))),
                    Self::tok_span(tokens),
                ]));
            }
            UiEvent::Step { step, state, preview, duration_ms, error, .. } => match state {
                // Running：用 spinner 字符示意进行中（最新一行即可，不强求覆盖）
                StepState::Running => {
                    self.push(Line::from(vec![
                        Span::styled(
                            format!("[{:>2}] ", step),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("⟳ ", Style::default().fg(Color::Cyan)),
                        Span::raw(preview),
                    ]));
                }
                StepState::Ok => {
                    let dur = duration_ms.map(fmt_duration).unwrap_or_default();
                    self.push(Line::from(vec![
                        Span::styled(
                            format!("[{:>2}] ", step),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("✓ ", Style::default().fg(Color::Green)),
                        Span::raw(preview),
                        Span::styled(format!("  {}", dur), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                StepState::Fail => {
                    let e = error.unwrap_or_default();
                    self.push(Line::from(vec![
                        Span::styled(
                            format!("[{:>2}] ", step),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("✗ ", Style::default().fg(Color::Red)),
                        Span::raw(preview),
                        Span::styled(
                            format!("  {}", brief(&e, 120)),
                            Style::default().fg(Color::Red),
                        ),
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
            UiEvent::AwaitingInput { question, .. } => {
                self.push_colored(format!("AI 提问：{}", question), Color::Cyan);
                self.awaiting = Some(question);
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
        // Ctrl+C：第一次发 Abort，第二次强制退出
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            if !self.abort_sent {
                let _ = tx.send(UiCommand::Abort);
                self.abort_sent = true;
            } else {
                self.should_quit = true;
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
                if !self.input.is_empty() {
                    let text = self.input.clone();
                    if self.awaiting.is_some() {
                        let _ = tx.send(UiCommand::Answer { text });
                        self.awaiting = None;
                    } else {
                        let _ = tx.send(UiCommand::Guidance { text });
                    }
                    self.input.clear();
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            // 结束后 q / Esc 退出（与正常输入互斥：finished 时不再当字符录入）
            KeyCode::Char('q') if self.finished => {
                self.should_quit = true;
            }
            KeyCode::Esc if self.finished => {
                self.should_quit = true;
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(c);
            }
            _ => {}
        }
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

/// 四段布局：顶栏(1) / 消息区(min) / 输入框(3) / 底栏(1)
fn view(frame: &mut Frame, model: &TuiModel) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(3),
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
        .unwrap_or_else(|| "harness".to_string());
    let elapsed = fmt_duration(model.started.elapsed().as_millis() as u64);
    let top = format!(
        " {} · 第{}轮 · ↑{} ↓{} · {} ",
        phase_label, model.round, model.tok_up, model.tok_down, elapsed
    );
    frame.render_widget(
        Paragraph::new(Line::from(top)).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );

    // ---- 消息区：自动换行；follow 贴底 / scroll 从底向上偏移 ----
    let msg_area = chunks[1];
    let total = model.lines.len() as u16;
    let height = msg_area.height;
    // 文本通常会换行，行数会比 lines 多，这里用 lines 数粗略估算贴底偏移（够用即可）
    let max_top = total.saturating_sub(height);
    let y = if model.follow {
        max_top
    } else {
        max_top.saturating_sub(model.scroll)
    };
    frame.render_widget(
        Paragraph::new(model.lines.clone())
            .wrap(Wrap { trim: false })
            .scroll((y, 0)),
        msg_area,
    );

    // ---- 输入框：awaiting 时高亮提示 ----
    let (title, border_style) = if model.awaiting.is_some() {
        (
            "AI 在等你回答（Enter 提交）",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        ("指导（Enter 发送）", Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(
        Paragraph::new(format!("> {}", model.input)).block(
            Block::bordered()
                .title(title)
                .border_style(border_style),
        ),
        chunks[2],
    );

    // ---- 底栏：快捷键提示（暗灰）----
    frame.render_widget(
        Paragraph::new(
            "Enter 发送 · ↑↓ 滚动 · End 贴底 · Ctrl+C 中断（再按强制退出）· 结束后 q 退出",
        )
        .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}
