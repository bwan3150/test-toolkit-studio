// ScriptParser模块 - 负责解析.tks脚本文件

mod constants;
mod parameter_parser;
mod serialize;
mod syntax_highlight;

pub use serialize::{script_to_source, step_to_source};
pub use syntax_highlight::{SyntaxHighlight, SyntaxHighlighter};

use crate::{Result, TksScript, TksStep, TksCommand};
use constants::{create_command_map, create_direction_map};
use parameter_parser::ParameterParser;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

/// 切出行内注释：`点击 [{1,2}] # 点开详情看是否跳转` → (`点击 [{1,2}]`, `点开详情看是否跳转`)
///
/// **必须跟踪引号状态**——URL 的锚点（`启动 ["https://x/#/list"]`）和文本参数里的井号
/// （`输入 [{1,2}, "话题#标签"]`）都是数据，不是注释。只有**引号外、且前面是空白或行首**
/// 的 `#` 才当注释开头。
fn split_inline_comment(line: &str) -> (&str, Option<String>) {
    let b = line.as_bytes();
    let mut in_quote = false;
    for i in 0..b.len() {
        match b[i] {
            b'"' => in_quote = !in_quote,
            b'#' if !in_quote && (i == 0 || b[i - 1].is_ascii_whitespace()) => {
                let note = line[i + 1..].trim();
                return (
                    line[..i].trim_end(),
                    (!note.is_empty()).then(|| note.to_string()),
                );
            }
            _ => {}
        }
    }
    (line, None)
}

/// 脚本解析器
pub struct ScriptParser {
    command_map: HashMap<String, TksCommand>,
    param_parser: ParameterParser,
}

impl ScriptParser {
    pub fn new() -> Self {
        let command_map = create_command_map();
        let direction_map = create_direction_map();
        let param_parser = ParameterParser::new(direction_map);

        Self {
            command_map,
            param_parser,
        }
    }

    /// 解析脚本文件
    pub fn parse_file(&self, script_path: &PathBuf) -> Result<TksScript> {
        let content = std::fs::read_to_string(script_path)
            .map_err(|e| crate::TkeError::IoError(e))?;

        let mut script = self.parse(&content)?;
        script.file_path = Some(script_path.clone());

        Ok(script)
    }

    /// 解析脚本内容
    pub fn parse(&self, content: &str) -> Result<TksScript> {
        let lines: Vec<&str> = content.lines().collect();

        let mut script = TksScript {
            case_id: String::new(),
            script_name: String::new(),
            details: HashMap::new(),
            steps: Vec::new(),
            file_path: None,
        };

        // 找到 "步骤:" 标记
        let mut in_steps = false;

        for (line_num, line) in lines.iter().enumerate() {
            // 行内注释切出来单独留着：它是「这一步在干什么」，要带进报告
            let (trimmed, note) = split_inline_comment(line.trim());

            // 跳过空行和纯注释行
            if trimmed.is_empty() {
                continue;
            }

            // 找到步骤部分
            if trimmed == "步骤:" {
                in_steps = true;
                continue;
            }

            // 只解析步骤部分的内容
            if in_steps {
                if let Some(mut step) = self.parse_step(trimmed, line_num + 1) {
                    step.note = note;
                    script.steps.push(step);
                }
            }
        }

        Ok(script)
    }

    /// 解析单个步骤
    fn parse_step(&self, line: &str, line_number: usize) -> Option<TksStep> {
        // 匹配命令格式
        // 格式1: 命令 [参数1, 参数2]
        // 格式2: 命令 参数1 参数2
        // 格式3: 命令

        let bracket_re = Regex::new(r"^(\S+)\s*\[(.*)\]$").ok()?;
        let simple_re = Regex::new(r"^(\S+)(?:\s+(.*))?$").ok()?;

        let (command_str, params_str) = if let Some(caps) = bracket_re.captures(line) {
            // 方括号格式
            let cmd = caps.get(1)?.as_str();
            let params = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            (cmd, params)
        } else if let Some(caps) = simple_re.captures(line) {
            // 简单格式
            let cmd = caps.get(1)?.as_str();
            let params = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            (cmd, params)
        } else {
            return None;
        };

        // 查找命令类型
        let command = self.command_map.get(command_str)?;

        // 解析参数
        let params = self.param_parser.parse_parameters(params_str);

        Some(TksStep {
            command: command.clone(),
            params,
            raw: line.to_string(),
            line_number,
            note: None,
        })
    }

    /// 获取语法高亮信息（用于编辑器）
    pub fn get_syntax_highlights(&self, content: &str) -> Vec<SyntaxHighlight> {
        let highlighter = SyntaxHighlighter::new(self.command_map.clone());
        highlighter.get_highlights(content)
    }
}

impl Default for ScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod comment_tests {
    use super::*;

    /// 行内注释被切出来,命令部分不受影响
    #[test]
    fn splits_inline_comment() {
        let (cmd, note) = split_inline_comment("点击 [{1, 2}] # 点开详情看是否跳转");
        assert_eq!(cmd, "点击 [{1, 2}]");
        assert_eq!(note.as_deref(), Some("点开详情看是否跳转"));
    }

    /// **引号内的 # 是数据不是注释**——URL 锚点、话题标签都靠这条不被切坏
    #[test]
    fn keeps_hash_inside_quotes() {
        let (cmd, note) = split_inline_comment(r#"启动 ["https://x.com/#/list"]"#);
        assert_eq!(cmd, r#"启动 ["https://x.com/#/list"]"#);
        assert!(note.is_none(), "URL 锚点不是注释");

        let (cmd, note) = split_inline_comment(r#"输入 [{1,2}, "话题#标签"] # 填个带井号的"#);
        assert_eq!(cmd, r#"输入 [{1,2}, "话题#标签"]"#, "只切引号外那个");
        assert_eq!(note.as_deref(), Some("填个带井号的"));
    }

    /// 紧贴着的 # 不算注释(如 `abc#def`),必须前面是空白——避免误切参数
    #[test]
    fn requires_whitespace_before_hash() {
        let (cmd, note) = split_inline_comment("按键 [KEYCODE#1]");
        assert_eq!(cmd, "按键 [KEYCODE#1]");
        assert!(note.is_none());
    }

    /// 整行注释:命令部分为空
    #[test]
    fn whole_line_comment() {
        let (cmd, note) = split_inline_comment("# 这一段在验证保存逻辑");
        assert_eq!(cmd, "");
        assert_eq!(note.as_deref(), Some("这一段在验证保存逻辑"));
    }

    /// 端到端:解析后 note 挂在对应的步骤上
    #[test]
    fn parser_attaches_note_to_step() {
        let script = ScriptParser::new()
            .parse("步骤:\n启动 [\"https://x\"] # 打开首页\n返回 # 退回去\n")
            .unwrap();
        assert_eq!(script.steps.len(), 2);
        assert_eq!(script.steps[0].note.as_deref(), Some("打开首页"));
        assert_eq!(script.steps[1].note.as_deref(), Some("退回去"));
        assert_eq!(script.steps[0].raw, r#"启动 ["https://x"]"#, "raw 不该含注释");
    }
}
