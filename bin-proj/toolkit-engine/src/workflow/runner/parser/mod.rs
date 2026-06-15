// ScriptParser模块 - 负责解析.tks脚本文件

mod constants;
mod parameter_parser;
mod syntax_highlight;

pub use syntax_highlight::{SyntaxHighlight, SyntaxHighlighter};

use crate::{Result, TksScript, TksStep, TksCommand};
use constants::{create_command_map, create_direction_map};
use parameter_parser::ParameterParser;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

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
            let trimmed = line.trim();

            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // 找到步骤部分
            if trimmed == "步骤:" {
                in_steps = true;
                continue;
            }

            // 只解析步骤部分的内容
            if in_steps {
                if let Some(step) = self.parse_step(trimmed, line_num + 1) {
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
