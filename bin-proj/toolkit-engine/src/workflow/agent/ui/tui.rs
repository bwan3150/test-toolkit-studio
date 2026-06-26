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

    /// TUI 是唯一可与用户多轮对话的前端：编排官跑完一条用例后可在此等下一句（REPL）。
    fn is_interactive(&self) -> bool {
        true
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
    /// 输入框光标位置（字符索引，0..=input.chars().count()）
    cursor: usize,
    /// 当前平台（从 SessionInfo 取，用于 Page 行的平台图标）
    platform: Option<String>,
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
            cursor: 0,
            platform: None,
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
            UiEvent::SessionInfo { device, platform, case } => {
                self.platform = Some(platform.clone());
                // setup 选好的参数，显示在最上面（探索开始前的第一条）
                self.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Green)),
                    Span::styled("准备就绪", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("  设备 {} · 平台 {}", device, platform),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                self.push_sub(format!("用例：{}", brief(&case, 80)));
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
            UiEvent::Assistant { text, tokens } => {
                self.add_tokens(tokens);
                self.push(Line::from(vec![
                    Span::raw(brief(&text, 400)),
                    Self::tok_span(tokens),
                ]));
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
            UiEvent::AwaitingInput { question, options, .. } => {
                if !options.is_empty() {
                    // 候选项 → 进 choosing 模式（方向键列表，在输入区渲染）；不灌消息流
                    self.choosing = Some(ChoiceState {
                        prompt: question,
                        options,
                        selected: 0,
                    });
                    self.follow = true;
                    self.scroll = 0;
                } else {
                    // 纯文本输入：问题直接显示在输入框标题处（不再灌进消息流），更像 opencode
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
                // 立即请求中断当前动作（滚动查找/回放等长动作轮询此标志会立刻停），再发 Pause 进暂停
                crate::utils::interrupt::request_pause();
                let _ = tx.send(UiCommand::Pause);
            }
            return;
        }

        // ---- choosing 模式：方向键列表选择，优先于一般按键 ----
        if let Some(ch) = self.choosing.as_mut() {
            let n = ch.options.len();
            match k.code {
                KeyCode::Up => {
                    ch.selected = ch.selected.saturating_sub(1);
                }
                KeyCode::Down => {
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
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/exit", "退出 TUI"),
    ("/stop", "停止当前探索（可继续输入指导）"),
    ("/help", "显示可用指令"),
];

/// 按 input 前缀过滤匹配的 / 指令
fn slash_matches(input: &str) -> Vec<&'static (&'static str, &'static str)> {
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

// ============================ 视图 ============================

/// 四段布局：顶栏(1) / 消息区(min) / 输入框(动态) / 底栏(1)
fn view(frame: &mut Frame, model: &TuiModel) {
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
        (ch.options.len() as u16 + groups + 3).clamp(4, 16)
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
    let input_area = chunks[3];
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
    let (title, border_style): (String, Style) = if let Some(q) = model.awaiting.as_ref() {
        // AI 提问直接显示在输入框标题处（opencode 风格）。不再写死"描述你要测什么"——
        // 问什么由编排官提示词决定（tke 是通用设备 agent，测试只是其一），UI 只如实显示它问的那句。
        (format!("? {}", brief(q, 70)), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
    body.push(Line::from(format!("> {}", model.input)));
    frame.render_widget(
        Paragraph::new(body).block(
            Block::bordered()
                .title(Line::from(title))
                .border_style(border_style),
        ),
        area,
    );
    // 光标定位到 model.cursor 处（输入行是块内最后一行）：x = 边框1 + "> "2 + 光标前子串宽度。
    let prefix: String = model.input.chars().take(model.cursor).collect();
    let pre_w = Span::raw(prefix).width() as u16;
    let cursor_x = area.x + 1 + 2 + pre_w;
    let cursor_y = area.y + area.height.saturating_sub(2);
    // 夹在区域内，避免越界
    let cx = cursor_x.min(area.x + area.width.saturating_sub(1));
    frame.set_cursor_position(Position::new(cx, cursor_y));
}
