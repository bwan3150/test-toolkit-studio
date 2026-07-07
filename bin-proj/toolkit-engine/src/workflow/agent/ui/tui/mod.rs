// 【TUI 前端】ratatui 全屏交互（真 TTY 时用）。三分结构：
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

use std::io::Stdout;
use std::sync::Mutex;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::command::UiCommand;
use super::event::UiEvent;
use super::Frontend;
use model::TuiModel;
use view::view;

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
    let mut terminal = enter_terminal()?;
    let mut model = TuiModel::new();
    let res = run_loop(&mut terminal, &mut events_rx, &commands_tx, &mut model);
    // 无论 res 如何都要恢复终端
    let _ = leave_terminal(&mut terminal);
    // 备用屏幕(alt-screen)不属于终端滚动缓冲区——TUI 里的内容退出即蒸发,选择也不跟滚动。
    // 退出后把整场消息流(带颜色)完整回放进 scrollback:记录永在,可滚/可选/可复制,排查全靠它。
    dump_transcript(&model.lines);
    res
}

/// 把消息流按原有颜色回放到普通终端（stderr，与其余输出一致）。
fn dump_transcript(lines: &[ratatui::text::Line<'static>]) {
    use ratatui::style::{Color, Modifier};
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
    eprintln!();
    eprintln!("\x1b[2m─ 本次会话完整记录（可滚动/复制排查）─\x1b[0m");
    for line in lines {
        let mut out = String::new();
        for span in &line.spans {
            let mut codes: Vec<&str> = Vec::new();
            if span.style.add_modifier.contains(Modifier::BOLD) {
                codes.push("1");
            }
            if let Some(c) = span.style.fg.and_then(fg_code) {
                codes.push(c);
            }
            if codes.is_empty() {
                out.push_str(&span.content);
            } else {
                out.push_str(&format!("\x1b[{}m{}\x1b[0m", codes.join(";"), span.content));
            }
        }
        eprintln!("{}", out);
    }
}

/// raw mode + 备用屏幕 + 隐藏光标 + bracketed paste（粘贴走 Event::Paste，不再逐字当按键）
fn enter_terminal() -> std::io::Result<Terminal<Backend>> {
    enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen, EnableBracketedPaste, Hide)?;
    Terminal::new(CrosstermBackend::new(out))
}

/// 退出备用屏幕 + 显示光标 + 关 raw mode（CrosstermBackend 实现了 Write）
fn leave_terminal(terminal: &mut Terminal<Backend>) -> std::io::Result<()> {
    execute!(terminal.backend_mut(), DisableBracketedPaste, LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    let _ = terminal.show_cursor();
    Ok(())
}

/// panic 时也恢复终端（否则用户终端卡在 raw/alt-screen）
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), DisableBracketedPaste, LeaveAlternateScreen, Show);
        prev(info);
    }));
}

/// 主循环：抽干引擎事件 → 处理键盘 → 重绘。靠用户按 q 退出。
/// **脏标记**：只有真的有事件/输入/resize 时才重绘——TUI 无动画元素，空闲时每 16ms
/// 无条件 draw 纯属烧 CPU（配合 view 的窗口化渲染，把"每帧 O(历史)"的老债一起清掉）。
fn run_loop(
    terminal: &mut Terminal<Backend>,
    events_rx: &mut UnboundedReceiver<UiEvent>,
    commands_tx: &UnboundedSender<UiCommand>,
    model: &mut TuiModel,
) -> std::io::Result<()> {
    let mut dirty = true; // 首帧必画
    loop {
        // 抽干引擎事件（断开返回 Disconnected，正常忽略——靠用户按 q 退出）
        while let Ok(ev) = events_rx.try_recv() {
            model.apply(ev);
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
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }
        if dirty {
            terminal.draw(|f| view(f, model))?;
            dirty = false;
        }
        if model.should_quit {
            break;
        }
    }
    Ok(())
}
