// 工具调用 → 强类型 AgentAction 解析

use crate::{LlmToolCall, Result, TkeError};

use super::action::AgentAction;

/// 把一次工具调用解析成 (强类型动作, comment)
/// comment 为 AI 这一步的思考（横切字段，所有动作类工具统一注入），与具体动作语义无关。
pub fn parse_tool_call(call: &LlmToolCall) -> Result<(AgentAction, Option<String>)> {
    let a = &call.arguments;
    let comment = opt_str(a, "comment");
    let action = match call.name.as_str() {
        "launch" => AgentAction::Launch {
            target: req_str(a, "target")?,
            activity: opt_str(a, "activity"),
        },
        "close" => AgentAction::Close { target: req_str(a, "target")? },
        "click" => AgentAction::Click {
            element_id: req_usize(a, "element_id")?,
            name: req_str(a, "name")?,
            desc: opt_str(a, "desc"),
        },
        "input" => AgentAction::Input {
            element_id: req_usize(a, "element_id")?,
            name: req_str(a, "name")?,
            desc: opt_str(a, "desc"),
            text: req_str(a, "text")?,
        },
        "long_press" => AgentAction::LongPress {
            element_id: req_usize(a, "element_id")?,
            name: req_str(a, "name")?,
            desc: opt_str(a, "desc"),
            duration_ms: opt_u64(a, "duration_ms").unwrap_or(1000),
        },
        "clear" => AgentAction::Clear {
            element_id: req_usize(a, "element_id")?,
            name: req_str(a, "name")?,
            desc: opt_str(a, "desc"),
        },
        "click_visual" => AgentAction::ClickVisual {
            region: opt_region(a, "region"),
            x: opt_i32(a, "x"),
            y: opt_i32(a, "y"),
            name: req_str(a, "name")?,
            desc: opt_str(a, "desc"),
        },
        "swipe_direction" => AgentAction::SwipeDir {
            direction: req_str(a, "direction")?,
            distance: opt_i32(a, "distance"),
            amount: opt_str(a, "amount"),
        },
        "back" => AgentAction::Back,
        "switch" => AgentAction::Switch { target: req_str(a, "target")? },
        "hide_keyboard" => AgentAction::HideKeyboard,
        "wait" => AgentAction::Wait {
            ms: opt_u64(a, "ms"),
            element: opt_str(a, "element"),
        },
        "request_screenshot" => AgentAction::RequestScreenshot { reason: req_str(a, "reason")? },
        "ask_user" => AgentAction::AskUser { question: req_str(a, "question")? },
        "finish" => AgentAction::Finish {
            success: a.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
            reason: req_str(a, "reason")?,
            script_name: opt_str(a, "script_name"),
        },
        other => {
            return Err(TkeError::ScriptExecuteError(format!("未知的工具调用: {}", other)));
        }
    };
    Ok((action, comment))
}

// ===== 参数提取小工具 =====

fn req_str(v: &serde_json::Value, key: &str) -> Result<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| TkeError::ScriptExecuteError(format!("工具参数缺少字符串字段: {}", key)))
}

fn opt_str(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
}

fn req_usize(v: &serde_json::Value, key: &str) -> Result<usize> {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|n| n as usize)
        .ok_or_else(|| TkeError::ScriptExecuteError(format!("工具参数缺少整数字段: {}", key)))
}

fn opt_u64(v: &serde_json::Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| x.as_u64())
}

fn opt_i32(v: &serde_json::Value, key: &str) -> Option<i32> {
    v.get(key).and_then(|x| x.as_i64()).map(|n| n as i32)
}

/// 解析 [x1,y1,x2,y2] 像素框；非 4 元素数组返回 None
fn opt_region(v: &serde_json::Value, key: &str) -> Option<[i32; 4]> {
    let arr = v.get(key)?.as_array()?;
    if arr.len() != 4 {
        return None;
    }
    let mut out = [0i32; 4];
    for (i, e) in arr.iter().enumerate() {
        out[i] = e.as_i64()? as i32;
    }
    Some(out)
}
