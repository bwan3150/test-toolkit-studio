// 事件输出器 - 人类可读 / 机器可读双模式
// 终端(TTY)默认输出规整的进度格式；管道/重定向自动切换 NDJSON（App/CI 用）；
// --json 可强制 NDJSON

use tke::RunEvent;
use std::io::{IsTerminal, Write};

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// 事件输出器
pub enum EventPrinter {
    /// NDJSON 逐行输出（机器读）
    Json,
    /// 规整的进度格式（人读）
    Pretty(PrettyState),
}

/// Pretty 模式的渐进状态
#[derive(Default)]
pub struct PrettyState {
    /// 当前脚本总步数
    total_steps: usize,
    /// flow 中脚本总数
    total_scripts: usize,
    /// 当前脚本序号（flow 中）
    script_index: usize,
}

impl EventPrinter {
    /// 自动选择模式：--json 或非 TTY → NDJSON；终端 → Pretty
    pub fn auto(force_json: bool) -> Self {
        if force_json || !std::io::stdout().is_terminal() {
            EventPrinter::Json
        } else {
            EventPrinter::Pretty(PrettyState::default())
        }
    }

    /// 输出一个事件
    pub fn print(&mut self, event: &RunEvent) {
        match self {
            EventPrinter::Json => {
                if let Ok(json) = serde_json::to_string(event) {
                    println!("{}", json);
                    let _ = std::io::stdout().flush();
                }
            }
            EventPrinter::Pretty(state) => print_pretty(state, event),
        }
    }
}

/// 人类可读格式输出
fn print_pretty(state: &mut PrettyState, event: &RunEvent) {
    match event {
        RunEvent::RunStart { script_name, total_steps, .. } => {
            state.total_steps = *total_steps;
            println!("{BOLD}▶ {script_name}{RESET} {DIM}({total_steps} 步){RESET}");
        }
        RunEvent::StepStart { index, command, .. } => {
            print!("  {DIM}[{:>2}/{}]{RESET} {} {DIM}...{RESET} ",
                   index + 1, state.total_steps, command);
            let _ = std::io::stdout().flush();
        }
        RunEvent::StepEnd { success, error, duration_ms, healed, dialog, .. } => {
            if *success {
                println!("{GREEN}✓{RESET} {DIM}{}{RESET}", fmt_duration(*duration_ms));
            } else {
                println!("{RED}✗{RESET} {DIM}{}{RESET}", fmt_duration(*duration_ms));
                if let Some(err) = error {
                    println!("         {RED}{}{RESET}", err);
                }
            }
            // 原生对话框：浏览器自己画的，截图和页面结构里都没有它的影子，
            // 而它正挡着后面每一步——这行不打出来，AI 只会看到后续操作莫名其妙全失败
            if let Some(d) = dialog {
                println!("         {YELLOW}⚠ 页面弹出对话框：「{d}」{RESET}");
                println!("         {DIM}它挡着后面所有操作，先用 `确认对话框` / `取消对话框` 处理掉{RESET}");
            }
            // AI 辅助驾驶：本步元素按原定位没找到，AI 依当前页面找回（不改脚本/元素包）
            if let Some(h) = healed {
                println!("         {YELLOW}🩹 AI 辅助:「{h}」原定位失效，已按当前页面找回{RESET}");
            }
        }
        RunEvent::RunEnd { success, total_steps, successful_steps, run_dir, healed, .. } => {
            if *success {
                println!("{GREEN}{BOLD}✓ 通过{RESET} {DIM}({successful_steps}/{total_steps} 步){RESET}");
            } else {
                println!("{RED}{BOLD}✗ 失败{RESET} {DIM}({successful_steps}/{total_steps} 步通过){RESET}");
            }
            if !healed.is_empty() {
                println!("  {YELLOW}🩹 AI 辅助驾驶介入 {} 处: {}{RESET}", healed.len(), healed.join("、"));
                println!("  {DIM}这些元素的原定位在当前 App 上已失效（本次未改动脚本/元素包，建议更新）{RESET}");
            }
            if !run_dir.is_empty() {
                println!("  {DIM}产物: {}{RESET}", run_dir);
            }
        }
        RunEvent::FlowStart { flow, total_scripts, .. } => {
            state.total_scripts = *total_scripts;
            println!("{CYAN}{BOLD}═══ Flow: {flow} ({total_scripts} 个脚本) ═══{RESET}");
        }
        RunEvent::ScriptStart { index, script } => {
            state.script_index = *index;
            println!("\n{CYAN}[脚本 {}/{}]{RESET} {}", index + 1, state.total_scripts, script);
        }
        RunEvent::ScriptEnd { success, error, .. } => {
            // 脚本级错误（如文件不存在）没有 run 事件，这里兜底输出
            if !success {
                if let Some(err) = error {
                    if state.total_steps == 0 {
                        println!("  {RED}✗ {}{RESET}", err);
                    }
                }
            }
            state.total_steps = 0;
        }
        RunEvent::FlowEnd { success, total_scripts, successful_scripts, run_dir, .. } => {
            println!();
            if *success {
                println!("{GREEN}{BOLD}═══ Flow 通过 ({successful_scripts}/{total_scripts}) ═══{RESET}");
            } else {
                println!("{RED}{BOLD}═══ Flow 失败 ({successful_scripts}/{total_scripts} 通过) ═══{RESET}");
            }
            if !run_dir.is_empty() {
                println!("  {DIM}产物: {}{RESET}", run_dir);
            }
        }
    }
}

/// 时长格式化: 320ms / 3.7s / 1m12s
fn fmt_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m{:02}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}
