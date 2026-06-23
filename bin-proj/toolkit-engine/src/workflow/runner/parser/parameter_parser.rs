// 参数解析器 - 解析脚本中的各种参数类型

use crate::{TksParam, Point, LocatorStrategy};
use std::collections::HashMap;

/// 参数解析器
pub struct ParameterParser {
    direction_map: HashMap<String, String>,
}

impl ParameterParser {
    pub fn new(direction_map: HashMap<String, String>) -> Self {
        Self { direction_map }
    }

    /// 解析参数字符串，返回参数列表
    pub fn parse_parameters(&self, params_str: &str) -> Vec<TksParam> {
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

    /// 解析单个参数
    pub fn parse_parameter(&self, param: &str) -> TksParam {
        let param = param.trim();

        // 1. 移除引号 → 文本
        if (param.starts_with('"') && param.ends_with('"'))
            || (param.starts_with('\'') && param.ends_with('\''))
        {
            return TksParam::Text(param[1..param.len() - 1].to_string());
        }

        // 2. 解析带单位的时长（等待用，人读无歧义）：必须先于裸数字、且 ms 先于 s
        //    1000ms → 1000 毫秒；5s → 5 秒。底层统一存毫秒(Duration)。
        if let Some(v) = param.strip_suffix("ms") {
            if let Ok(ms) = v.trim().parse::<u32>() {
                return TksParam::Duration(ms);
            }
        }
        if let Some(v) = param.strip_suffix('s') {
            if let Ok(seconds) = v.trim().parse::<u32>() {
                return TksParam::Duration(seconds * 1000);
            }
        }

        // 3. 解析数字（无单位的纯数字：按压时长/滑动幅度等仍是裸数字）
        if let Ok(num) = param.parse::<i32>() {
            return TksParam::Number(num);
        }

        // 5. 解析元素引用 {元素名} 或 {元素名}&策略
        if let Some(element_param) = self.parse_element_param(param) {
            return element_param;
        }

        // 6. 解析方向
        if let Some(direction) = self.direction_map.get(param) {
            return TksParam::Direction(direction.clone());
        }

        // 7. 解析布尔值
        match param {
            "存在" | "true" => return TksParam::Boolean(true),
            "不存在" | "false" => return TksParam::Boolean(false),
            _ => {}
        }

        // 8. 默认返回文本
        TksParam::Text(param.to_string())
    }

    /// 解析元素参数 {元素名} 或 {元素名}&策略
    /// 统一处理，不再区分 XML 和图片元素
    fn parse_element_param(&self, param: &str) -> Option<TksParam> {
        // 必须以 { 开头
        if !param.starts_with('{') {
            return None;
        }

        // 找到右大括号位置
        let close_pos = param.find('}')?;
        let inner = &param[1..close_pos];
        let after = &param[close_pos + 1..];

        // 检查是否为坐标格式 {x,y}
        if inner.contains(',') && !after.contains('&') {
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (
                    parts[0].trim().parse::<i32>(),
                    parts[1].trim().parse::<i32>(),
                ) {
                    return Some(TksParam::Coordinate(Point::new(x, y)));
                }
            }
        }

        // 解析元素名和策略
        let name = inner.trim().to_string();

        // 解析策略
        let strategy = if after.starts_with('&') {
            LocatorStrategy::from_str(after[1..].trim())
        } else {
            LocatorStrategy::Auto
        };

        Some(TksParam::Element { name, strategy })
    }
}
