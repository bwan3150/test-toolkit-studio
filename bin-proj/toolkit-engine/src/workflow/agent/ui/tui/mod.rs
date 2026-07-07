// 【TUI 前端】**手写 inline 渲染**（dialoguer/Claude Code 模式，真 TTY 时用）：不进备用屏幕、
// 不用 ratatui Terminal（其 insert_before 逐 cell 输出会把中文宽字符打成"选 择 设 备"）——
// 历史行转 ANSI 字符串直接 print 进 scrollback（宽字符终端原生处理；原生滚动/选择/复制、退出留痕）；
// 底部小窗（状态行+输入区+底栏，**高度动态**）用相对定位每帧重画：光标上移 N 行→清到屏底→重打。三分结构：
//   mod.rs    TuiFrontend(引擎侧句柄/Frontend impl) + 终端生命周期 + 主循环
//   model.rs  TuiModel 渲染状态：UiEvent→彩色行、键盘/粘贴处理、/指令
//   view.rs   画屏：布局/顶栏/消息区(窗口化)/输入区/底栏
//
// 引擎跑在 tokio 侧、TUI 跑在独立 std::thread 独占终端，两条 unbounded channel 桥接：
//   events_tx (引擎 emit → TUI 渲染)、commands_tx (TUI 键盘 → 引擎 drain)。
// emit 永不阻塞引擎（unbounded send）；drain/await_answer 同步块内 try_recv，绝不跨 await 持 std Mutex。
// 退出靠用户在 TUI 里按 q（结束后）/ Ctrl+C（中断→再按强制退出）；run_tui 所有返回路径都恢复终端。

mod model;
mod view;

use std::sync::Mutex;
use std::time::Duration;

use std::io::Write;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind},
    execute, queue,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::command::UiCommand;
use super::event::UiEvent;
use super::Frontend;
use model::TuiModel;
use view::{build_view, line_ansi};


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

    /// 提问/授权直接弹给终端前的用户。
    fn supports_prompts(&self) -> bool {
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
        self.emit(UiEvent::AwaitingInput { round, question, options: Vec::new(), free_input: false });
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
            free_input: false,
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

    /// 「选项 或 直接输入」：choosing 列表末尾带**内联输入行**——用户可 ↑↓ 选，也可直接打字
    /// （一打字焦点自动跳到输入行），不必先选"其他"再等输入框。
    /// 内部协议（仅 TUI 线程 ↔ 本方法）：Answer.text = "pick:N"（选中第 N 项）或 "text:…"（自由输入）。
    async fn await_choice_or_text(&self, prompt: String, options: Vec<String>) -> Option<super::ChoiceReply> {
        self.emit(UiEvent::AwaitingInput {
            round: 0,
            question: prompt,
            options: options.clone(),
            free_input: true,
        });
        loop {
            {
                let mut rx = match self.commands_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => {
                            if let Some(n) = text.strip_prefix("pick:").and_then(|s| s.parse::<usize>().ok()) {
                                return Some(super::ChoiceReply::Pick(n));
                            }
                            if let Some(t) = text.strip_prefix("text:") {
                                return Some(super::ChoiceReply::Text(t.to_string()));
                            }
                            return None;
                        }
                        UiCommand::Abort => return None,
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
    enable_raw_mode()?;
    let mut out = std::io::stdout();
    // 开局清屏：shell 历史/上次会话残留全部推进 scrollback（往上滚仍可见），本场从干净首屏开始
    execute!(
        out,
        EnableBracketedPaste,
        Clear(ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;
    let mut model = TuiModel::new();
    let mut screen = Inline::default();
    let res = run_loop(&mut out, &mut screen, &mut events_rx, &commands_tx, &mut model);
    // 收尾：擦掉底部小窗、光标停在内容尾（shell 提示接着出现），恢复终端
    let end_row = screen.content_row;
    let _ = execute!(
        out,
        crossterm::cursor::MoveTo(0, end_row),
        Clear(ClearType::FromCursorDown),
        DisableBracketedPaste,
        Show
    );
    let _ = disable_raw_mode();
    res
}

/// panic 时也恢复终端（否则用户终端卡在 raw mode）
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), DisableBracketedPaste, Show);
        prev(info);
    }));
}

/// 手写 inline 屏(**钉底式**)：小窗永远画在终端窗口最底部;内容从屏幕顶往下长
/// (content_row 跟踪内容尾的屏幕行),未满屏时中间留空,长到小窗顶后自然滚屏。
/// 全部基于绝对坐标 MoveTo + 终端自然滚屏,每帧取实际屏幕尺寸,抗 resize。
#[derive(Default)]
struct Inline {
    /// 内容尾:下一条历史行该打印的屏幕行号(0-based;到达屏幕底后停在底行,靠滚屏前进)
    content_row: u16,
}

impl Inline {
    /// 历史行插入:光标到内容尾→清到屏底(顺带擦掉小窗)→逐行 print(到底后自然滚屏)。
    /// content_row 记账必须按**wrap 后的物理行数**加(长行占多行,只加 1 会与真实光标失配,
    /// 后续 MoveTo 定位错 → 小窗残影);整段包同步输出,原子刷新不闪。
    fn insert_history(&mut self, out: &mut impl Write, lines: &[ratatui::text::Line<'static>]) -> std::io::Result<()> {
        let (tw, sh) = crossterm::terminal::size()?;
        let tw = tw.max(1) as usize;
        let bottom = sh.saturating_sub(1);
        self.content_row = self.content_row.min(bottom);
        queue!(out, BeginSynchronizedUpdate, crossterm::cursor::MoveTo(0, self.content_row), Hide, Clear(ClearType::FromCursorDown))?;
        for l in lines {
            queue!(out, Print(line_ansi(l)), Print("\r\n"))?;
            let w = l.width();
            let used = ((w / tw + usize::from(w % tw != 0)).max(1)) as u16;
            self.content_row = (self.content_row + used).min(bottom);
        }
        queue!(out, EndSynchronizedUpdate)?;
        out.flush()
    }

    /// 重画小窗(钉在屏幕底部)：内容已侵入小窗区则先滚屏腾位,再从 sh-小窗高 处画。
    fn redraw(
        &mut self,
        out: &mut impl Write,
        rows: &[ratatui::text::Line<'static>],
        cursor: Option<(u16, u16)>,
    ) -> std::io::Result<()> {
        let (_, sh) = crossterm::terminal::size()?;
        let win_h = (rows.len() as u16).min(sh);
        let wy = sh.saturating_sub(win_h);
        queue!(out, BeginSynchronizedUpdate)?;
        // 内容尾越过小窗顶：在底行发换行滚屏腾位(内容整体上移,不丢——进 scrollback)
        if self.content_row > wy {
            queue!(out, crossterm::cursor::MoveTo(0, sh.saturating_sub(1)))?;
            for _ in 0..(self.content_row - wy) {
                queue!(out, Print("\r\n"))?;
            }
            self.content_row = wy;
        }
        queue!(out, crossterm::cursor::MoveTo(0, wy), Hide, Clear(ClearType::FromCursorDown))?;
        let last = rows.len().saturating_sub(1);
        for (i, r) in rows.iter().enumerate() {
            queue!(out, Print(line_ansi(r)))?;
            if i < last {
                queue!(out, Print("\r\n"))?;
            }
        }
        // 光标：有输入位→绝对定位并显示;无(choosing)→保持隐藏
        if let Some((row, col)) = cursor {
            queue!(out, crossterm::cursor::MoveTo(col, wy + row.min(win_h.saturating_sub(1))), Show)?;
        }
        queue!(out, EndSynchronizedUpdate)?;
        out.flush()
    }
}

/// 主循环：抽干引擎事件 → flush 历史进 scrollback → 处理键盘 → 重画小窗。靠 /exit(q) 退出。
fn run_loop(
    out: &mut std::io::Stdout,
    screen: &mut Inline,
    events_rx: &mut UnboundedReceiver<UiEvent>,
    commands_tx: &UnboundedSender<UiCommand>,
    model: &mut TuiModel,
) -> std::io::Result<()> {
    let mut dirty = true; // 首帧必画
    let mut last_tick = std::time::Instant::now();
    loop {
        // 抽干引擎事件（断开返回 Disconnected，正常忽略——靠用户退出）
        while let Ok(ev) = events_rx.try_recv() {
            model.apply(ev);
            dirty = true;
        }
        // 新定稿行进 scrollback（历史归终端管，宽字符原生渲染）
        if model.flushed < model.lines.len() {
            let new: Vec<ratatui::text::Line<'static>> = model.lines[model.flushed..].to_vec();
            model.flushed = model.lines.len();
            screen.insert_history(out, &new)?;
            dirty = true;
        }
        // 活动中驱动 spinner 动画(~120ms 一帧);空闲(等输入/选择/已结束)不重绘,
        // 用户可自由上滚查看历史而不被输出拉回底部
        let busy = !model.finished && model.awaiting.is_none() && model.choosing.is_none();
        if busy && last_tick.elapsed() >= Duration::from_millis(120) {
            model.spin = model.spin.wrapping_add(1);
            last_tick = std::time::Instant::now();
            dirty = true;
        }
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    model.handle_key(k, commands_tx);
                    dirty = true;
                }
                // bracketed paste：整段进输入框，粘贴内容里的换行不会触发 Enter 误提交
                Event::Paste(s) => {
                    model.handle_paste(&s);
                    dirty = true;
                }
                Event::Resize(_, h) => {
                    // 终端 reflow 后绝对坐标记账失效——夹紧到新屏高,下一帧整窗重画
                    screen.content_row = screen.content_row.min(h.saturating_sub(1));
                    dirty = true;
                }
                _ => {}
            }
        }
        if dirty {
            let width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);
            let (rows, cursor) = build_view(model, width);
            screen.redraw(out, &rows, cursor)?;
            dirty = false;
        }
        if model.should_quit {
            break;
        }
    }
    Ok(())
}
