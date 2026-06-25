// 【Plain 前端】行式输出（非 TTY / 管道 / CI / 重定向时用）。
//
// 不复刻历史 eprintln 的逐字噪声，而是用一套统一规整的行式风格渲染 UiEvent——既是回归兜底，
// 也顺手把旧的又乱又丑的输出收拾干净。drain_commands 读进程级中断标志；await_answer 走旧的
// 阻塞 read_user_line（CLI 无 TUI 时的 ask_user 通道）。

use std::io::IsTerminal;

use super::super::interaction::read_user_line;
use super::super::runner::flow::{brief, fmt_duration, fmt_tokens, paint};
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
                eprintln!("  {}  {}", brief(&text, 200), self.toks(tokens));
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
            UiEvent::Summary { explore, diagnose, verify, reason, model, tokens } => {
                eprintln!();
                eprintln!("{}", paint(tty, "1", "╭─ 结果 ──────────────────────────────"));
                eprintln!("  {}   {}", paint(tty, "2", "探索"), paint(tty, lc(explore.level), &explore.text));
                eprintln!("  {}   {}", paint(tty, "2", "诊断"), paint(tty, lc(diagnose.level), &diagnose.text));
                eprintln!("  {}   {}", paint(tty, "2", "验证"), paint(tty, lc(verify.level), &verify.text));
                eprintln!("  {}   {}", paint(tty, "2", "依据"), brief(&reason, 200));
                eprintln!("  {}   {}", paint(tty, "2", "模型"), model);
                eprintln!(
                    "  {}  {}",
                    paint(tty, "2", "Token"),
                    paint(tty, "2", &format!("↑{} ↓{} · 合计 {}", fmt_tokens(tokens.prompt), fmt_tokens(tokens.completion), fmt_tokens(tokens.prompt + tokens.completion)))
                );
                eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
            }
            UiEvent::Elements { committed, items, committed_to_lib } => {
                eprintln!();
                eprintln!("{}", paint(tty, "1", "╭─ 元素库更新 ────────────────────────"));
                if !committed_to_lib {
                    eprintln!(
                        "  {}",
                        paint(tty, "33", "未稳定通过——脚本未保存；本次元素只存在临时库（随运行目录丢弃），正式元素库未改动")
                    );
                } else if items.is_empty() {
                    eprintln!("  {}   {}", paint(tty, "2", "提交"), paint(tty, "2", "（无新元素）"));
                } else {
                    let line_for = |it: &ElementItem| match &it.desc {
                        Some(d) => format!("{} · {}", it.name, brief(d, 80)),
                        None => it.name.clone(),
                    };
                    eprintln!("  {}   {}", paint(tty, "2", &format!("提交 {} 个", committed)), paint(tty, "32", &line_for(&items[0])));
                    for it in &items[1..] {
                        eprintln!("         {}", paint(tty, "32", &line_for(it)));
                    }
                    eprintln!("  {}", paint(tty, "2", "（最终脚本用到的元素已写入正式库，desc 据实际作用生成，请人工二次审核）"));
                }
                eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
            }
            // 行式模式下：ask_user 由 await_answer 直接走 read_user_line（自带提问输出），
            // 这里不再重复打印；Done 的产物路径由 harness::handle 在收尾时统一打印。
            UiEvent::SessionInfo { device, platform, case } => {
                eprintln!(
                    "{}",
                    paint(tty, "1", &format!("准备就绪 · 设备 {} · 平台 {} · 用例 {}", device, platform, brief(&case, 80)))
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
