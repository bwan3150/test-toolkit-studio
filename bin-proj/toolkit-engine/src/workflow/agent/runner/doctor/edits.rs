// 【医生编辑工具】EditOp(医生的一次编辑/控制动作) + 工具 schema + 行校验 + 元素重选辅助
// (reposition 定位 / tier_for 落库通道 / build_action_line 生成 .tks 行)。主循环在 mod.rs。

use std::path::Path;
use std::sync::Arc;

use crate::tools::element::OcrChannel;
use crate::workflow::step_to_source;
use crate::{LlmTool, LlmToolCall, LocatorStrategy, Params, ScriptParser, TksCommand, TksParam, TksStep, UIElement};

use super::super::super::execution::script::write_script;
use super::super::super::prompt::PromptSet;
use super::super::ctx::DriveCtx;
use super::super::verify::{do_replay, reset_state};

/// 医生发起的一次编辑/控制动作
pub(super) enum EditOp {
    /// 删除 from..=to 行(1-based 含两端)
    Delete { from: usize, to: usize },
    /// 把第 line 行替换成 content(原始 .tks 文本)
    Replace { line: usize, content: String },
    /// 在第 after 行之后插入 content(after=0 → 插到最前)
    Insert { after: usize, content: String },
    /// 定位到第 step 步将操作的实时页面（重启+回放到 step-1 步），fetch 实时元素交给医生重选，
    /// 随后用 Pick 选定正确元素。用于"该步点错了元素/元素记错了/要换别的元素"。
    Reexplore { step: usize, reason: String },
    /// 在 Reexplore 定位后的实时页面里，选定第 id 个元素作为该步的操作目标：
    /// 实时落库为 name，并把该步的 .tks 行改成对它的 action（click/input/long_press/clear/assert）。
    Pick { id: usize, name: String, action: String, text: Option<String> },
    /// 看图视觉选：元素列表里没有/反复点不中（如纯 img 图标、滑动没到位）时，看 reexplore 那帧
    /// 截图直接给像素框 region=[x1,y1,x2,y2]（优先）或点 (x,y)，按像素落 img 元素并替换该步。
    PickVisual { region: Option<[i32; 4]>, x: Option<i32>, y: Option<i32>, name: String, action: String, text: Option<String> },
    /// 重新诊断回放(测试当前编辑效果)
    Run,
    /// 收尾(医生认为已达标且最短)
    Finish { reason: String },
}

/// Reexplore 定位后暂存的实时页面（供随后的 Pick / PickVisual 选元素落库）
pub(super) struct PendingReselect {
    /// 要修的步号(1-based)
    pub(super) step: usize,
    /// 该实时页面解析出的元素
    pub(super) elements: Vec<UIElement>,
    /// 该实时页面的截图路径（供 PickVisual 看图框选、按像素落 img 元素）
    pub(super) shot_path: std::path::PathBuf,
}

/// 医生工具的 name + 参数 schema 表（description 不在此——由 PromptSet 提供，
/// 内置默认见 prompt/builtin/tools/doctor/<name>.md，外部 <prompts_dir>/tools/doctor/<name>.md 可覆盖）。
fn doctor_tool_schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "delete_lines",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "integer", "description": "起始行号(1-based)" },
                    "to": { "type": "integer", "description": "结束行号(1-based，含)；删单行时 to=from" },
                    "reason": { "type": "string", "description": "为什么删这些行" }
                },
                "required": ["from", "to"]
            }),
        ),
        (
            "replace_line",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "line": { "type": "integer", "description": "要替换的行号(1-based)" },
                    "content": { "type": "string", "description": "新的 .tks 行，如 `输入 [{搜索框}] \"正确文本\"` 或 `定向滑动 [{640,406}, 上, quarter]`" },
                    "reason": { "type": "string", "description": "为什么这样改" }
                },
                "required": ["line", "content"]
            }),
        ),
        (
            "insert_after",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "after": { "type": "integer", "description": "插入位置：在这一行之后(0=最前)" },
                    "content": { "type": "string", "description": "要插入的 .tks 行" },
                    "reason": { "type": "string", "description": "为什么插入" }
                },
                "required": ["after", "content"]
            }),
        ),
        (
            "reexplore",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "step": { "type": "integer", "description": "要重新选元素的步号(1-based)。系统会重启并回放到它的前一步、停在该步将操作的实时页面，给你实时元素列表。" },
                    "reason": { "type": "string", "description": "为什么这步要重选元素(看 trace 说清楚哪步点错了/元素记错了)" }
                },
                "required": ["step", "reason"]
            }),
        ),
        (
            "pick",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "reexplore 给的实时元素列表里的元素序号" },
                    "name": { "type": "string", "description": "给该元素起的稳定语义名（落库+写进 .tks；列表里标了「已收录」的复用其库名）" },
                    "action": { "type": "string", "enum": ["click", "hover", "input", "long_press", "clear", "assert"], "description": "对该元素的操作，默认 click；hover=悬停展开下拉(仅 web)" },
                    "text": { "type": "string", "description": "action=input 时要输入的文本" }
                },
                "required": ["id", "name"]
            }),
        ),
        (
            "pick_visual",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "region": { "type": "array", "items": { "type": "integer" }, "minItems": 4, "maxItems": 4, "description": "目标在截图中的像素框 [x1,y1,x2,y2]（优先，越贴合越好）" },
                    "x": { "type": "integer", "description": "无 region 时给点击点 x（像素）" },
                    "y": { "type": "integer", "description": "无 region 时给点击点 y（像素）" },
                    "name": { "type": "string", "description": "给该目标起的稳定语义名（落库+写进 .tks）" },
                    "action": { "type": "string", "enum": ["click", "hover", "input", "long_press", "clear", "assert"], "description": "对该目标的操作，默认 click；hover=悬停展开下拉(仅 web)" },
                    "text": { "type": "string", "description": "action=input 时要输入的文本" }
                },
                "required": ["name"]
            }),
        ),
        ("run", serde_json::json!({ "type": "object", "properties": {} })),
        (
            "finish",
            serde_json::json!({
                "type": "object",
                "properties": { "reason": { "type": "string", "description": "收尾依据" } },
                "required": ["reason"]
            }),
        ),
    ]
}

/// 组装医生工具集：schema 来自上表，description 来自 PromptSet（可外部覆盖）。
/// 医生工具不注入 comment（它们各自有 reason 字段表达意图）。
pub(super) fn build_doctor_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    doctor_tool_schemas()
        .into_iter()
        .map(|(name, schema)| LlmTool::new(name, prompts.role_tool_description("doctor", name), schema))
        .collect()
}

/// 校验一行 .tks 是否可安全引入：能解析，且其中所有**元素引用**都已存在于元素库。
/// 坐标(Coordinate)不算元素引用、不校验。返回 Err(原因) 表示不安全。
pub(super) fn validate_line(content: &str, element_path: &Path) -> std::result::Result<(), String> {
    let script = ScriptParser::new()
        .parse(&format!("步骤:\n{}", content))
        .map_err(|e| format!("无法解析为有效 .tks 步骤：{}", e))?;
    if script.steps.is_empty() {
        return Err("没有解析出任何有效步骤".into());
    }
    // 元素库：{ "elements": { "<name>": {...} } }
    let lib: serde_json::Value = std::fs::read_to_string(element_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "elements": {} }));
    for step in &script.steps {
        // ① 元素引用必须是库里已有的（否则回放找不到）
        for p in &step.params {
            if let TksParam::Element { name, .. } = p {
                if lib["elements"].get(name).is_none() {
                    return Err(format!(
                        "元素「{}」不在元素库中——replace/insert 只能引用已收录元素；要点页面上库里还没有的元素，请用 reexplore 定位+pick 现场选并存库",
                        name
                    ));
                }
            }
        }
        // ② 点击/输入/长按/清空/断言 的**目标**(首参)只能是 [{库元素名}] 或坐标 [{x,y}]，
        //    不能是裸文本/元素描述（如 `点击 资源链接`、`点击 p(text=...)`、`svg(...)`）——那会被当成页面
        //    文本搜索、极不可靠，且通常是"想点一个还没存库的元素"的错误写法。该用 reexplore+pick。
        let targeting = matches!(
            step.command,
            TksCommand::Click | TksCommand::Hover | TksCommand::Press | TksCommand::Input | TksCommand::Clear | TksCommand::Assert
        );
        if targeting && !matches!(step.params.first(), Some(TksParam::Element { .. }) | Some(TksParam::Coordinate(_))) {
            return Err(format!(
                "「{}」的目标必须是 [{{库里已有的元素名}}] 或坐标 [{{x,y}}]，不能用裸文本/元素描述\
                 （如 `点击 资源链接`、`点击 p(text=...)`）——那只是页面文本搜索、不可靠。要点页面上库里还没有的元素，\
                 请用 reexplore 定位到该步、再 pick 在实时页面上选中它（会存库），别凭描述硬点。",
                step.raw.trim()
            ));
        }
    }
    Ok(())
}

/// 解析医生的一次工具调用 → EditOp
pub(super) fn parse_edit(call: &LlmToolCall) -> std::result::Result<EditOp, String> {
    let a = &call.arguments;
    let uint = |k: &str| a.get(k).and_then(|v| v.as_u64()).map(|n| n as usize);
    let string = |k: &str| a.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    match call.name.as_str() {
        "delete_lines" => {
            let from = uint("from").ok_or("缺少 from")?;
            let to = uint("to").ok_or("缺少 to")?;
            Ok(EditOp::Delete { from, to })
        }
        "replace_line" => Ok(EditOp::Replace {
            line: uint("line").ok_or("缺少 line")?,
            content: string("content").ok_or("缺少 content")?,
        }),
        "insert_after" => Ok(EditOp::Insert {
            after: uint("after").ok_or("缺少 after")?,
            content: string("content").ok_or("缺少 content")?,
        }),
        "pick" => Ok(EditOp::Pick {
            id: uint("id").ok_or("缺少 id")?,
            name: string("name").ok_or("缺少 name")?,
            action: string("action").unwrap_or_else(|| "click".to_string()),
            text: string("text"),
        }),
        "pick_visual" => {
            let region = a.get("region").and_then(|v| v.as_array()).and_then(|arr| {
                if arr.len() == 4 {
                    let mut r = [0i32; 4];
                    for (i, e) in arr.iter().enumerate() {
                        r[i] = e.as_i64()? as i32;
                    }
                    Some(r)
                } else {
                    None
                }
            });
            Ok(EditOp::PickVisual {
                region,
                x: a.get("x").and_then(|v| v.as_i64()).map(|n| n as i32),
                y: a.get("y").and_then(|v| v.as_i64()).map(|n| n as i32),
                name: string("name").ok_or("缺少 name")?,
                action: string("action").unwrap_or_else(|| "click".to_string()),
                text: string("text"),
            })
        }
        "reexplore" => Ok(EditOp::Reexplore {
            step: uint("step").ok_or("缺少 step")?,
            reason: string("reason").unwrap_or_default(),
        }),
        "run" => Ok(EditOp::Run),
        "finish" => Ok(EditOp::Finish { reason: string("reason").unwrap_or_default() }),
        other => Err(format!("未知工具：{}", other)),
    }
}

/// 重启净化 + 回放 lines[0..cut]，把设备定位到第 cut 步后的页面（即第 cut+1 步将操作的页面）。
pub(super) async fn reposition(ctx: &DriveCtx<'_>, params: &Arc<Params>, script_path: &Path, case: &str, lines: &[String], cut: usize) {
    reset_state(ctx.device, lines).await;
    if cut > 0 {
        let prefix = &lines[..cut.min(lines.len())];
        let _ = write_script(script_path, case, prefix);
        // verbose=true：reexplore 定位时把「回放前缀」逐步打印出来，让人看清浏览器在重走哪几步、
        // 停到哪个页面，而不是浏览器默默动、CLI 一片空白后突然蹦出结果。
        let _ = do_replay(params, script_path, true, ctx.ui).await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// 元素的落库通道（与 execution::tier_for 一致）：OcrText→结构空+ocr 文字；其它→结构+ocr。
pub(super) fn tier_for(el: &UIElement) -> (Option<&UIElement>, OcrChannel) {
    if el.class_name == "OcrText" {
        let ocr = el.text.clone().filter(|t| !t.trim().is_empty()).map(OcrChannel::Text).unwrap_or(OcrChannel::FromCrop);
        (None, ocr)
    } else {
        let ocr = el
            .text
            .clone()
            .or_else(|| el.content_desc.clone())
            .filter(|t| !t.trim().is_empty())
            .map(OcrChannel::Text)
            .unwrap_or(OcrChannel::FromCrop);
        (Some(el), ocr)
    }
}

/// 据 action 给某元素构造一行可回放的 .tks（经 Phase2 序列化器，保证与 parser 同构）。
pub(super) fn build_action_line(action: &str, name: &str, text: Option<&str>) -> std::result::Result<String, String> {
    let el = TksParam::Element { name: name.to_string(), strategy: LocatorStrategy::Auto };
    let (command, params) = match action {
        "click" => (TksCommand::Click, vec![el]),
        "hover" => (TksCommand::Hover, vec![el]),
        "input" => (TksCommand::Input, vec![el, TksParam::Text(text.unwrap_or_default().to_string())]),
        "long_press" => (TksCommand::Press, vec![el, TksParam::Number(1000)]),
        "clear" => (TksCommand::Clear, vec![el]),
        "assert" => (TksCommand::Assert, vec![el, TksParam::Text("存在".to_string())]),
        other => return Err(format!("不支持的动作「{}」（仅 click/hover/input/long_press/clear/assert）", other)),
    };
    Ok(step_to_source(&TksStep { command, params, raw: String::new(), line_number: 0 }))
}
