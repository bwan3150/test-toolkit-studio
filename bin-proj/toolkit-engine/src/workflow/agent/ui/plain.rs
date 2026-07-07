// 【Plain 前端】行式输出（非 TTY / 管道 / CI / 重定向时用）。
//
// 不复刻历史 eprintln 的逐字噪声，而是用一套统一规整的行式风格渲染 UiEvent——既是回归兜底，
// 也顺手把旧的又乱又丑的输出收拾干净。drain_commands 读进程级中断标志；await_answer 走旧的
// 阻塞 read_user_line（CLI 无 TUI 时的 ask_user 通道）。

use std::io::IsTerminal;

use super::super::interaction::read_user_line;
use super::super::runner::fmt::{brief, fmt_duration, fmt_tokens, paint};
use super::command::UiCommand;
use super::event::*;
use super::Frontend;

/// 语义色级 → ANSI 码
fn lc(l: Level) -> &'static str {
    match l {
        Level::Info => "36",
        Level::Ok => "32",
        Level::Warn => "33",
        Level::Err => "31",
        Level::Dim => "2",
    }
}

pub struct PlainFrontend {
    tty: bool,
}

impl PlainFrontend {
    pub fn new() -> Self {
        Self { tty: std::io::stderr().is_terminal() }
    }

    fn toks(&self, t: Tokens) -> String {
        paint(self.tty, "2", &format!("↑{} ↓{}", fmt_tokens(t.prompt), fmt_tokens(t.completion)))
    }
}

impl Default for PlainFrontend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Frontend for PlainFrontend {
    fn emit(&self, ev: UiEvent) {
        let tty = self.tty;
        match ev {
            UiEvent::Phase { phase, n } => {
                let title = match n {
                    Some(k) => format!("{} #{}", phase.label(), k),
                    None => phase.label().to_string(),
                };
                eprintln!();
                eprintln!("{}", paint(tty, "1", &format!("━━ {} ━━", title)));
            }
            UiEvent::Page { struct_elements, ocr_added, ocr_failed, tabs, not_ready, .. } => {
                let mut parts = vec![format!("{} 元素", struct_elements)];
                if ocr_failed {
                    parts.push("OCR✗".to_string());
                } else if let Some(n) = ocr_added {
                    parts.push(format!("+{} OCR", n));
                }
                if tabs > 0 {
                    parts.push(format!("{} 标签", tabs));
                }
                let mut line = format!("· {}", parts.join(" · "));
                if let Some(nr) = not_ready {
                    let s = match nr {
                        NotReady::SessionClosed => "会话已关闭（收尾）",
                        NotReady::NeedLaunch => "待 launch",
                    };
                    line.push_str(&format!("  · {}", s));
                }
                eprintln!();
                eprintln!("{}", paint(tty, "2", &line));
            }
            UiEvent::OcrError { error, .. } => {
                eprintln!("  {}", paint(tty, "31", &format!("! {}", brief(&error, 160))));
            }
            UiEvent::Stuck { message, .. } => {
                eprintln!("  {}", paint(tty, "33", &format!("~ {}", message)));
            }
            UiEvent::AgentThought { text, tokens, .. } => {
                eprintln!("  {} {}  {}", paint(tty, "2", "└ Explorer"), brief(&text, 200), self.toks(tokens));
            }
            // 主 AI（编排官）：助手本体、对话主体——多行回复**完整**展示（不截成首行，
            // 否则 REPL 里它的方案/解释全被砍掉），token 角标跟在末行。
            UiEvent::Assistant { text, tokens } => {
                let mut body: Vec<&str> = text.lines().collect();
                while body.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    body.pop();
                }
                let n = body.len();
                for (i, l) in body.iter().enumerate() {
                    if i + 1 == n {
                        eprintln!("  {}  {}", l, self.toks(tokens));
                    } else {
                        eprintln!("  {}", l);
                    }
                }
            }
            // 主 AI 的计划清单：每项一行（[ ]待办 [~]进行中 [x]完成）。
            UiEvent::Todo { items } => {
                eprintln!("  计划：");
                for it in &items {
                    let mark = match it.status {
                        TodoStatus::Pending => "[ ]",
                        TodoStatus::InProgress => "[~]",
                        TodoStatus::Done => "[x]",
                    };
                    eprintln!("    {} {}", mark, it.text);
                }
            }
            UiEvent::SubAgent { kind, level, text, tokens } => {
                eprintln!(
                    "  {} {}  {}",
                    paint(tty, "2", &format!("└ {}", kind.label())),
                    paint(tty, lc(level), &brief(&text, 200)),
                    self.toks(tokens)
                );
            }
            UiEvent::Step { step, state, preview, line: _, duration_ms, error } => match state {
                StepState::Running => {} // 行式无需实时进行中
                StepState::Ok => {
                    let dur = duration_ms.map(fmt_duration).unwrap_or_default();
                    eprintln!(
                        "  {} {} {} {}",
                        paint(tty, "2", &format!("[{:>2}]", step)),
                        paint(tty, "32", "✓"),
                        preview,
                        paint(tty, "2", &dur)
                    );
                }
                StepState::Fail => {
                    let e = error.unwrap_or_default();
                    eprintln!(
                        "  {} {} {} {}",
                        paint(tty, "2", &format!("[{:>2}]", step)),
                        paint(tty, "31", "✗"),
                        preview,
                        paint(tty, "2", &brief(&e, 80))
                    );
                }
            },
            UiEvent::Rename { old, new } => {
                eprintln!("  {}", paint(tty, "35", &format!("✎ 改名：{} → {}", old, new)));
            }
            UiEvent::AutoStop { reason } => {
                eprintln!("  {}", paint(tty, "31", &format!("■ {}", reason)));
            }
            UiEvent::ExitingNote { message } => {
                eprintln!("  {}", paint(tty, "33", &message));
            }
            UiEvent::Notice { level, text } => {
                eprintln!("  {}", paint(tty, lc(level), &text));
            }
            UiEvent::GuidanceAccepted { text } => {
                eprintln!("  {}", paint(tty, "36", &format!("↳ 已采纳指导：{}", brief(&text, 200))));
            }
            UiEvent::ScriptGenerated { name, steps, success } => {
                eprintln!();
                if success {
                    eprintln!("{}", paint(tty, "1;32", &format!("✓ 脚本生成完毕：{}（{} 步）", name, steps)));
                } else {
                    eprintln!("{}", paint(tty, "1;33", &format!("⚠ 探索未达成，脚本不完整：{}（{} 步）", name, steps)));
                }
            }
            UiEvent::Summary { status, diagnose, verify, reason, script, model, tokens } => {
                eprintln!();
                eprintln!("{}", paint(tty, "1", "╭─ 结果 ──────────────────────────────"));
                eprintln!("  {}", paint(tty, lc(status.level), &status.text));
                if let Some(d) = diagnose {
                    eprintln!("  {}   {}", paint(tty, "2", "回放"), paint(tty, lc(d.level), &d.text));
                }
                if let Some(v) = verify {
                    eprintln!("  {}   {}", paint(tty, "2", "稳定"), paint(tty, lc(v.level), &v.text));
                }
                eprintln!("  {}   {}", paint(tty, "2", "依据"), brief(&reason, 200));
                if let Some(sc) = script {
                    let stem = sc.name.strip_suffix(".tks").unwrap_or(&sc.name);
                    eprintln!();
                    eprintln!(
                        "  {}   {}",
                        paint(tty, "2", "脚本"),
                        paint(tty, "1", &format!("{}.tks + {}.tklib（{} 元素）", stem, stem, sc.elements.len()))
                    );
                    for (i, step) in sc.steps.iter().enumerate() {
                        eprintln!("    {}", paint(tty, "2", &format!("{:>2}  {}", i + 1, brief(step, 90))));
                    }
                    if !sc.elements.is_empty() {
                        eprintln!();
                        let w = sc.elements.iter().map(|e| e.name.chars().count()).max().unwrap_or(0);
                        for e in &sc.elements {
                            let pad = " ".repeat(w.saturating_sub(e.name.chars().count()));
                            match &e.desc {
                                Some(d) => eprintln!("  {}   {}{}  {}", paint(tty, "2", "元素"), paint(tty, "32", &e.name), pad, paint(tty, "2", &brief(d, 70))),
                                None => eprintln!("  {}   {}", paint(tty, "2", "元素"), paint(tty, "32", &e.name)),
                            }
                        }
                        eprintln!("  {}", paint(tty, "2", "（desc 自动生成，建议人工复核）"));
                    }
                }
                eprintln!();
                eprintln!(
                    "  {}",
                    paint(tty, "2", &format!("{} · ↑{} ↓{} · 合计 {}", model, fmt_tokens(tokens.prompt), fmt_tokens(tokens.completion), fmt_tokens(tokens.prompt + tokens.completion)))
                );
                eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
            }
            // 行式模式下：ask_user 由 await_answer 直接走 read_user_line（自带提问输出），
            // 这里不再重复打印；Done 的产物路径由 harness::handle 在收尾时统一打印。
            UiEvent::SessionInfo { device, platform, model, .. } => {
                // 行式日志保留模型名便于复盘（TUI 里则收进 /model 查询）
                eprintln!(
                    "{}",
                    paint(tty, "1", &format!("准备就绪 · 设备 {} · 平台 {} · 模型 {}", device, platform, model))
                );
            }
            UiEvent::AwaitingInput { .. } | UiEvent::Done { .. } => {}
        }
    }

    fn drain_commands(&self) -> Vec<UiCommand> {
        if super::super::runner::interrupt::aborted() {
            vec![UiCommand::Abort]
        } else {
            Vec::new()
        }
    }

    async fn await_answer(&self, _round: usize, question: String) -> Option<String> {
        Some(read_user_line(&question).await)
    }

    async fn shutdown(self: Box<Self>) {}
}
