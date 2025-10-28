// ScriptParser模块 - 负责解析.tks脚本文件

use crate::{Result, TkeError, TksScript, TksStep, TksCommand, TksParam, Point};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ScriptParser {
    // 命令映射
    command_map: HashMap<String, TksCommand>,
    // 方向映射
    direction_map: HashMap<String, String>,
}

impl ScriptParser {
    pub fn new() -> Self {
        let mut command_map = HashMap::new();
        command_map.insert("启动".to_string(), TksCommand::Launch);
        command_map.insert("关闭".to_string(), TksCommand::Close);
        command_map.insert("点击".to_string(), TksCommand::Click);
        command_map.insert("按压".to_string(), TksCommand::Press);
        command_map.insert("滑动".to_string(), TksCommand::Swipe);
        command_map.insert("定向滑动".to_string(), TksCommand::DirectionalSwipe);
        command_map.insert("输入".to_string(), TksCommand::Input);
        command_map.insert("清理".to_string(), TksCommand::Clear);
        command_map.insert("隐藏键盘".to_string(), TksCommand::HideKeyboard);
        command_map.insert("返回".to_string(), TksCommand::Back);
        command_map.insert("等待".to_string(), TksCommand::Wait);
        command_map.insert("断言".to_string(), TksCommand::Assert);
        
        let mut direction_map = HashMap::new();
        direction_map.insert("上".to_string(), "up".to_string());
        direction_map.insert("下".to_string(), "down".to_string());
        direction_map.insert("左".to_string(), "left".to_string());
        direction_map.insert("右".to_string(), "right".to_string());
        
        Self {
            command_map,
            direction_map,
        }
    }
    
    // 解析脚本文件
    pub fn parse_file(&self, script_path: &PathBuf) -> Result<TksScript> {
        let content = std::fs::read_to_string(script_path)
            .map_err(|e| TkeError::IoError(e))?;
        
        let mut script = self.parse(&content)?;
        script.file_path = Some(script_path.clone());
        
        Ok(script)
    }
    
    // 解析脚本内容
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
    
    // 解析单个步骤
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
        let params = self.parse_parameters(params_str);
        
        Some(TksStep {
            command: command.clone(),
            params,
            raw: line.to_string(),
            line_number,
        })
    }
    
    // 解析参数
    fn parse_parameters(&self, params_str: &str) -> Vec<TksParam> {
        if params_str.is_empty() {
            return Vec::new();
        }
        
        let mut params = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut bracket_depth = 0;
        
        for ch in params_str.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current.push(ch);
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                    current.push(ch);
                }
                '{' if !in_quotes => {
                    bracket_depth += 1;
                    current.push(ch);
                }
                '}' if !in_quotes => {
                    bracket_depth -= 1;
                    current.push(ch);
                }
                ',' if !in_quotes && bracket_depth == 0 => {
                    // 参数分隔符
                    if !current.trim().is_empty() {
                        params.push(self.parse_parameter(current.trim()));
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        
        // 处理最后一个参数
        if !current.trim().is_empty() {
            params.push(self.parse_parameter(current.trim()));
        }
        
        params
    }
    
    // 解析单个参数
    fn parse_parameter(&self, param: &str) -> TksParam {
        let param = param.trim();
        
        // 移除引号
        if (param.starts_with('"') && param.ends_with('"')) ||
           (param.starts_with('\'') && param.ends_with('\'')) {
            return TksParam::Text(param[1..param.len()-1].to_string());
        }
        
        // 解析数字
        if let Ok(num) = param.parse::<i32>() {
            return TksParam::Number(num);
        }
        
        // 解析时间（如 10s）
        if param.ends_with('s') {
            if let Ok(seconds) = param[..param.len()-1].parse::<u32>() {
                return TksParam::Duration(seconds * 1000);
            }
        }
        
        // 解析时间（纯数字，秒数 - 与JS版本保持一致）
        if let Ok(seconds) = param.parse::<u32>() {
            // 如果是纯数字且看起来像秒数（小于等于3600秒），则转换为毫秒
            if seconds <= 3600 {
                return TksParam::Duration(seconds * 1000);
            }
        }
        
        // 解析坐标 {x,y} 或 XML元素引用 {元素名} 或 {元素名}&策略
        if param.starts_with('{') && !param.contains('@') {
            // 找到右大括号的位置
            if let Some(close_brace_pos) = param.find('}') {
                let inner = &param[1..close_brace_pos];
                let after_brace = &param[close_brace_pos+1..];

                // 检查是否为坐标格式（包含逗号且不包含&）
                if inner.contains(',') && !after_brace.contains('&') {
                    let parts: Vec<&str> = inner.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(),
                                                 parts[1].trim().parse::<i32>()) {
                            return TksParam::Coordinate(Point::new(x, y));
                        }
                    }
                }

                // 否则为XML元素引用，解析可选的策略
                // 格式1: {元素名} - 全精确匹配
                // 格式2: {元素名}&resourceId - 使用 resourceId 策略
                // 格式3: {元素名}&text - 使用 text 策略
                // 格式4: {元素名}&className - 使用 className 策略
                // 格式5: {元素名}&contentDesc - 使用 contentDesc 策略
                // 格式6: {元素名}&xpath - 使用 xpath 策略
                let (name, strategy) = if after_brace.starts_with('&') {
                    let element_name = inner.trim().to_string();
                    let strategy_name = after_brace[1..].trim().to_string();

                    // 验证策略名称
                    match strategy_name.as_str() {
                        "resourceId" | "text" | "className" | "contentDesc" | "xpath" => {
                            (element_name, Some(strategy_name))
                        }
                        _ => {
                            // 无效的策略名称，忽略策略
                            (inner.to_string(), None)
                        }
                    }
                } else {
                    // 没有指定策略
                    (inner.to_string(), None)
                };

                return TksParam::XmlElement { name, strategy };
            }
        }
        
        // 解析图像引用 @{图片名称}
        if param.starts_with("@{") && param.ends_with('}') {
            let image_name = param[2..param.len()-1].to_string();
            return TksParam::ImageElement(image_name);
        }
        
        // 解析方向
        if let Some(direction) = self.direction_map.get(param) {
            return TksParam::Direction(direction.clone());
        }
        
        // 解析布尔值
        match param {
            "存在" | "true" => return TksParam::Boolean(true),
            "不存在" | "false" => return TksParam::Boolean(false),
            _ => {}
        }
        
        // 默认返回文本
        TksParam::Text(param.to_string())
    }
    
    // 获取语法高亮信息（用于编辑器）
    pub fn get_syntax_highlights(&self, content: &str) -> Vec<SyntaxHighlight> {
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
            if trimmed.starts_with("用例:") || 
               trimmed.starts_with("脚本名:") ||
               trimmed == "详情:" ||
               trimmed == "步骤:" {
                highlights.push(SyntaxHighlight {
                    line: line_num,
                    start: 0,
                    end: trimmed.find(':').unwrap_or(trimmed.len()) + 1,
                    token_type: TokenType::Keyword,
                });
            }
            
            // 命令
            for (cmd_name, _) in &self.command_map {
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

// 语法高亮信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyntaxHighlight {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub token_type: TokenType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TokenType {
    Keyword,
    Command,
    Parameter,
    String,
    Number,
    Comment,
    Locator,
}

// 为语法高亮生成颜色映射
impl TokenType {
    pub fn to_color(&self) -> &str {
        match self {
            TokenType::Keyword => "#569CD6",     // 蓝色
            TokenType::Command => "#C586C0",     // 紫色
            TokenType::Parameter => "#9CDCFE",   // 浅蓝色
            TokenType::String => "#CE9178",      // 橙色
            TokenType::Number => "#B5CEA8",      // 浅绿色
            TokenType::Comment => "#6A9955",     // 绿色
            TokenType::Locator => "#DCDCAA",     // 黄色
        }
    }
}