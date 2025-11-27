// 语法高亮模块 - 为编辑器提供语法高亮信息

use serde::Serialize;
use std::collections::HashMap;
use crate::TksCommand;

/// 语法高亮信息
#[derive(Debug, Clone, Serialize)]
pub struct SyntaxHighlight {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub token_type: TokenType,
}

/// Token 类型
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TokenType {
    Keyword,
    Command,
    Parameter,
    String,
    Number,
    Comment,
    Locator,
}

impl TokenType {
    /// 获取 Token 类型对应的颜色
    pub fn to_color(&self) -> &str {
        match self {
            TokenType::Keyword => "#569CD6",   // 蓝色
            TokenType::Command => "#C586C0",   // 紫色
            TokenType::Parameter => "#9CDCFE", // 浅蓝色
            TokenType::String => "#CE9178",    // 橙色
            TokenType::Number => "#B5CEA8",    // 浅绿色
            TokenType::Comment => "#6A9955",   // 绿色
            TokenType::Locator => "#DCDCAA",   // 黄色
        }
    }
}

/// 语法高亮器
pub struct SyntaxHighlighter {
    command_map: HashMap<String, TksCommand>,
}

impl SyntaxHighlighter {
    pub fn new(command_map: HashMap<String, TksCommand>) -> Self {
        Self { command_map }
    }

    /// 获取脚本内容的语法高亮信息
    pub fn get_highlights(&self, content: &str) -> Vec<SyntaxHighlight> {
        let mut highlights = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // 注释
            if trimmed.starts_with('#') {
                highlights.push(SyntaxHighlight {
                    line: line_num,
                    start: 0,
                    end: line.len(),
                    token_type: TokenType::Comment,
                });
                continue;
            }

            // 关键字
            if trimmed.starts_with("用例:")
                || trimmed.starts_with("脚本名:")
                || trimmed == "详情:"
                || trimmed == "步骤:"
            {
                highlights.push(SyntaxHighlight {
                    line: line_num,
                    start: 0,
                    end: trimmed.find(':').unwrap_or(trimmed.len()) + 1,
                    token_type: TokenType::Keyword,
                });
            }

            // 命令
            for cmd_name in self.command_map.keys() {
                if trimmed.starts_with(cmd_name) {
                    highlights.push(SyntaxHighlight {
                        line: line_num,
                        start: 0,
                        end: cmd_name.len(),
                        token_type: TokenType::Command,
                    });
                    break;
                }
            }
        }

        highlights
    }
}
