// 【JSON 前端】NDJSON 双向（被 Electron app spawn 时用）。
//
// 引擎 → 前端：每个 UiEvent 序列化成一行 JSON 写到 stdout（外部标签 {"type":...}），flush 即时可读。
// 前端 → 引擎：后台 tokio 任务逐行读 stdin，每行反序列化成 UiCommand 投递到 channel；
//   引擎在安全点 drain_commands() 取走，await_answer() 轮询取 Answer/Abort。
// 解析失败的行直接忽略（不 panic、不打断流），保证脏输入不会拖垮引擎。

use std::io::{Stdout, Write};
use std::sync::Mutex;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedReceiver;

use super::command::UiCommand;
use super::event::UiEvent;
use super::{ChoiceReply, Frontend};

pub struct JsonFrontend {
    /// 后台 stdin 读取任务投递来的命令（引擎在安全点 try_recv 取）
    cmd_rx: Mutex<UnboundedReceiver<UiCommand>>,
    /// 输出句柄：emit 时锁住、写一行 JSON 再 flush
    out: Mutex<Stdout>,
}

impl JsonFrontend {
    /// 建 channel + 起后台 stdin 读取任务，返回持有 rx + stdout 的句柄。
    pub fn spawn() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<UiCommand>();
        tokio::spawn(async move {
            let mut lines = BufReader::new(tokio::io::stdin()).lines();
            // next_line() 返回 Ok(Some(line)) 直到 EOF（Ok(None)）；读错则收尾退出
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim();
                if line.is_empty() {
                    continue; // 空行跳过
                }
                match serde_json::from_str::<UiCommand>(line) {
                    Ok(cmd) => {
                        if tx.send(cmd).is_err() {
                            break; // 接收端已丢弃（引擎结束），停止读取
                        }
                    }
                    Err(_) => continue, // 解析失败：忽略该行，不 panic
                }
            }
        });
        Self {
            cmd_rx: Mutex::new(rx),
            out: Mutex::new(std::io::stdout()),
        }
    }
}

#[async_trait::async_trait]
impl Frontend for JsonFrontend {
    /// app 可代问代答（AwaitingInput 事件 ↔ Answer 命令）：ask_user / 文件授权都经此送达用户。
    /// 注意 is_interactive 仍为 false——跑完即结束、不开 REPL；这两个维度是独立的。
    fn supports_prompts(&self) -> bool {
        true
    }

    fn emit(&self, ev: UiEvent) {
        // 序列化失败则丢弃该事件（不应发生；UiEvent 全字段可序列化）
        if let Ok(s) = serde_json::to_string(&ev) {
            if let Ok(mut out) = self.out.lock() {
                let _ = writeln!(out, "{}", s);
                let _ = out.flush();
            }
        }
    }

    fn drain_commands(&self) -> Vec<UiCommand> {
        let mut cmds = Vec::new();
        if let Ok(mut rx) = self.cmd_rx.lock() {
            while let Ok(c) = rx.try_recv() {
                cmds.push(c);
            }
        }
        cmds
    }

    async fn await_answer(&self, round: usize, question: String) -> Option<String> {
        // 先告知前端「正在等输入」，前端据此提示用户并回 Answer / Abort。
        // **free_input=true**：这是纯文本提问，回什么都收 —— 早先写死 false，
        // 前端照着它把输入框禁掉，于是"等你回答"变成了"没法回答"
        self.emit(UiEvent::AwaitingInput { round, question, options: Vec::new(), free_input: true });
        // 轮询：每 50ms 取一次命令。不可跨 await 持有 std Mutex——锁只在同步块内用。
        loop {
            {
                let mut rx = match self.cmd_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => return Some(text),
                        UiCommand::Abort => return None,
                        // Guidance/Pause/Resume：等输入期间不处理，忽略
                        _ => {}
                    }
                }
            } // 锁在此处释放，再 await
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 「给几个选项，也可以自己写」（`ask_user` 带 options 走这里）。
    ///
    /// 默认实现把选项**拼进问题文字**再走 await_answer —— 前端只收到一段长文本，
    /// 渲染不出可点的选项，`options` 字段也是空的。这里如实发：`options` 给出候选，
    /// `free_input=true` 表示"选一个或者自己写都行"。
    /// 选项是**建议**不是**闭集**：AI 想不全用户要什么，堵死自由输入等于逼人从错的里面挑。
    async fn await_choice_or_text(&self, prompt: String, options: Vec<String>) -> Option<ChoiceReply> {
        self.emit(UiEvent::AwaitingInput {
            round: 0,
            question: prompt,
            options: options.clone(),
            free_input: true,
        });
        loop {
            {
                let mut rx = match self.cmd_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => {
                            let t = text.trim();
                            if t.is_empty() {
                                return None;
                            }
                            // 选项原文精确匹配 → 当作选中（前端点了那个选项）
                            if let Some(i) = options.iter().position(|o| o == t) {
                                return Some(ChoiceReply::Pick(i));
                            }
                            // 纯数字下标也认（与 await_choice 同口径）；否则就是用户自己写的话
                            if let Ok(n) = t.parse::<usize>() {
                                if n < options.len() {
                                    return Some(ChoiceReply::Pick(n));
                                }
                            }
                            return Some(ChoiceReply::Text(text));
                        }
                        UiCommand::Abort => return None,
                        _ => {}
                    }
                }
            } // 锁在此释放，再 await
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 从候选里选一个（文件授权三选、设备选择等）：emit AwaitingInput{options 非空}，
    /// app 弹窗后回 `{"type":"answer","text":"…"}`——text 为 **0-based 下标**（与 TUI 内部一致），
    /// 也接受选项原文精确匹配。Abort → None（调用方按"拒绝/放弃"处理）。
    async fn await_choice(&self, prompt: String, options: Vec<String>) -> Option<usize> {
        self.emit(UiEvent::AwaitingInput { round: 0, question: prompt, options: options.clone(), free_input: false });
        loop {
            {
                let mut rx = match self.cmd_rx.lock() {
                    Ok(rx) => rx,
                    Err(_) => return None,
                };
                while let Ok(c) = rx.try_recv() {
                    match c {
                        UiCommand::Answer { text } => {
                            let t = text.trim();
                            if let Some(i) = options.iter().position(|o| o == t) {
                                return Some(i);
                            }
                            return t.parse::<usize>().ok().filter(|n| *n < options.len());
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
        if let Ok(mut out) = self.out.lock() {
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_event_serializes_with_type_tag() {
        let ev = UiEvent::Page {
            round: 1,
            struct_elements: 3,
            ocr_added: None,
            ocr_failed: false,
            tabs: 0,
            location: None,
            known: 0,
            unknown: 3,
            revisits: 0,
            not_ready: None,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""type":"page""#), "实际序列化：{}", s);
    }

    #[test]
    fn json_command_deserializes() {
        // guidance
        let g = serde_json::from_str::<UiCommand>(r#"{"type":"guidance","text":"x"}"#).unwrap();
        match g {
            UiCommand::Guidance { text } => assert_eq!(text, "x"),
            other => panic!("期望 Guidance，得到 {:?}", other),
        }
        // abort
        let a = serde_json::from_str::<UiCommand>(r#"{"type":"abort"}"#).unwrap();
        assert!(matches!(a, UiCommand::Abort), "期望 Abort，得到 {:?}", a);
        // answer
        let ans = serde_json::from_str::<UiCommand>(r#"{"type":"answer","text":"y"}"#).unwrap();
        match ans {
            UiCommand::Answer { text } => assert_eq!(text, "y"),
            other => panic!("期望 Answer，得到 {:?}", other),
        }
    }
}
