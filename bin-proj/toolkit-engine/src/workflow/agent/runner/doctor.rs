// 【脚本医生 / 编辑器 agent】—— 代码 agent 式的脚本修复 + 提炼
//
// 取代旧的「断点续接修复(repair_once) + 候选删冗余(minimize_candidates)」。把两件事
// 统一成一个**专职会话**(独立工具集)的主循环：
//   ① 诊断回放：整脚本跑一遍，**每步都留下页面**(结构 + OCR)，产出富 trace；
//   ② 把「编号脚本 + 富 trace」交给医生 agent，它用编辑工具**改任意行**：
//        delete_lines / replace_line / insert_after —— 纯文本编辑(删/改/插)，
//          其中 replace/insert 只准引用**已在元素库里的元素**(校验，杜绝凭空捏造)；
//        reexplore —— 需要全新导航/新元素时的**活体逃生口**：回放前缀定位设备后
//          现场重探那一段再拼回(复用探索会话，有设备知识)；
//        run —— 重新诊断回放(医生「测试自己的修改」的方式)；
//        finish —— 改完收尾(系统仍以诊断兜底校验)。
//   ③ 自动护栏：每次诊断后维护「最短的达标版本 best」。一旦某批改动让目标**丢失**，
//        自动还原到 best 并告知医生——这样「删冗余」既能让脚本变短、又绝不把可用脚本改坏。
//
// 与旧版的本质区别：旧版只会「从某点把尾巴整段重探」，删不掉中间冗余步、改不了单行坏参数、
// 也看不到每步页面。医生能看每步去了哪、做最小必要的外科手术。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::engines::ocr::enrich_with_ocr;
use crate::tools::element::{add_element_target, OcrChannel};
use crate::workflow::step_to_source;
use crate::{
    AiConfig, LlmReply, LlmSession, LlmTool, LlmToolCall, LocatorStrategy, Params, Platform, RunEvent, ScriptParser, ScriptRunner,
    TksCommand, TksParam, TksStep, UIElement,
};

use super::super::execution::script::write_script;
use super::super::execution::{auto_name, visual_auto_name};
use super::super::perception::{capture, render_element_list};
use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::super::ui::{Level, Phase, StepState, SubAgent, Tokens, UiCommand, UiEvent};
use super::flow::{brief, friendly, is_launch_line, parse_desc_json, DriveCtx};
use super::options::VerifyReport;
use super::verify::{do_replay, page_contains, reset_state, strip_trailing_close};

// 医生主循环诊断轮数上限来自 config [harness].doctor_iters（params.harness.doctor_iters）。
/// 单轮内医生最多调用几次 LLM(防在编辑里空转)
const MAX_EDITS_PER_ITER: usize = 24;

// ===================== 富 trace 数据结构 =====================

/// 诊断回放中的一步
struct DiagStep {
    /// 1-based 步号
    no: usize,
    /// .tks 原始行
    line: String,
    ok: bool,
    err: Option<String>,
    /// 该步执行后页面的元素列表（渲染 trace 时普通步截断、失败步/终点步给全量）
    page_full: String,
    /// 该步执行后页面的**结构签名**（仅结构元素、排除 OCR 伪元素，OCR 噪声不影响）——
    /// 用于稳健判断"这步有没有让页面结构变化"，不受 OCR 文字识别的轻微抖动干扰。
    struct_sig: u64,
    /// 该步标注截图路径（落盘产物，写入 conversation.json 供复盘）
    screenshot: Option<String>,
    /// 该步页面结构文件路径
    xml: Option<String>,
}

/// 一组元素的结构签名（仅结构元素、排除 OCR 伪元素），用于判断页面结构有没有变化。
fn struct_signature(els: &[UIElement]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    els.iter()
        .filter(|e| e.class_name != "OcrText")
        .map(|e| e.to_ai_text())
        .collect::<Vec<_>>()
        .join("|")
        .hash(&mut h);
    h.finish()
}

/// 一次诊断结果
pub(super) struct Diagnosis {
    /// 是否真到达目标(脚本全跑通 且 目标标志出现)
    pub(super) reached: bool,
    steps: Vec<DiagStep>,
    /// 第一个失败步的下标(0-based)；None=全跑通
    fail_idx: Option<usize>,
    /// 一句话结论(给医生 & CLI)
    note: String,
    /// 脚本跑完后的最终页面元素（渲染文本），供医生 finish 时对照用户需求做终点校验
    final_page: String,
}

impl Diagnosis {
    /// 「无效操作步」的步号集合（1-based）——执行后**页面结构没变化**（点了个不起作用的元素），
    /// 反思官可安全删。判据三条同时满足：
    ///  ① 该步执行后**结构签名与上一步相同**（排除 OCR 噪声，比"page_full 精确相等"稳健得多——
    ///     之前用 page_full 精确比，OCR 文字的轻微抖动就把"没变"误判成"变了"，导致无效点击漏网）；
    ///  ② 当前页**非空**（0 元素的空页常是刚开出来的 PDF/新标签等承重结果，保守保留）；
    ///  ③ **紧跟其后不是断言步**——点击后下一步就 assert，说明这步已被断言确认有效（典型：点击开新
    ///     标签、下一步断言新标签内容；新标签不改当前页结构，光看①会误判它无效），这种保留。
    /// 这样「点了没让结构变、又没被随后断言确认」的无效点击会被抓出来；而"有效但不改当前页结构"
    /// 的承重点击（开新标签/下载）因②③得到保护。删错了还有反思官删后整体重诊断兜底（跑不到目标即放弃本次优化）。
    pub(super) fn noop_step_nos(&self) -> std::collections::HashSet<usize> {
        let mut set = std::collections::HashSet::new();
        for i in 1..self.steps.len() {
            if self.steps[i].page_full.trim().is_empty() {
                continue; // ② 空页不算
            }
            if self.steps[i].struct_sig != self.steps[i - 1].struct_sig {
                continue; // ① 结构变了 = 有效
            }
            if let Some(next) = self.steps.get(i + 1) {
                if next.line.trim_start().starts_with("断言") {
                    continue; // ③ 紧跟断言 = 已确认有效
                }
            }
            set.insert(self.steps[i].no);
        }
        set
    }

    /// 给反思官的紧凑逐步清单：步号 · 成败 · .tks 行 · 该步后页面前几项（求短，用于路径优化分析）
    pub(super) fn trace_lines(&self) -> String {
        let mut s = String::new();
        for st in &self.steps {
            let mark = if st.ok { "✓" } else { "✗" };
            s.push_str(&format!("{}. {} {}", st.no, mark, friendly(&st.line)));
            let page = top_lines(&st.page_full, 6);
            if !page.trim().is_empty() {
                s.push_str(&format!("  [页面: {}]", page.replace('\n', " ")));
            }
            s.push('\n');
        }
        s
    }

    /// 页面访问/重复分析（给反思官判冗余）：按每步页面内容分组，标出哪些步落在**完全相同**的页面、
    /// 各页被访问几次。同一页被反复访问 = 很可能在原地打转/绕路，即使指令本身不重复也算冗余。
    pub(super) fn page_groups(&self) -> String {
        use std::collections::HashMap;
        // 页面内容 → 首个出现的步号（代表）+ 命中的步号列表
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for st in &self.steps {
            let key = st.page_full.trim();
            if key.is_empty() {
                continue;
            }
            if let Some(&gi) = index.get(key) {
                groups[gi].1.push(st.no);
            } else {
                index.insert(key, groups.len());
                groups.push((st.page_full.clone(), vec![st.no]));
            }
        }
        let mut out = String::new();
        for (gi, (page, steps)) in groups.iter().enumerate() {
            let head = top_lines(page, 3).replace('\n', " ");
            let dup = if steps.len() > 1 {
                format!("（步 {} 落在**完全相同**的页面，访问 {} 次 ← 疑似原地打转/绕路）", steps.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(","), steps.len())
            } else {
                format!("（步 {}）", steps[0])
            };
            out.push_str(&format!("页面#{}: {}  {}\n", gi + 1, head, dup));
        }
        out
    }
}

// ===================== 医生编辑动作 =====================

/// 医生发起的一次编辑/控制动作
enum EditOp {
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
struct PendingReselect {
    /// 要修的步号(1-based)
    step: usize,
    /// 该实时页面解析出的元素
    elements: Vec<UIElement>,
    /// 该实时页面的截图路径（供 PickVisual 看图框选、按像素落 img 元素）
    shot_path: std::path::PathBuf,
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
fn build_doctor_tools(prompts: &PromptSet) -> Vec<LlmTool> {
    doctor_tool_schemas()
        .into_iter()
        .map(|(name, schema)| LlmTool::new(name, prompts.role_tool_description("doctor", name), schema))
        .collect()
}

// ===================== 诊断回放 =====================

/// 诊断回放：整脚本跑一遍(去结尾「关闭」步)，**每步留下页面**，产出富 trace + 是否到达目标。
/// 靠 ScriptRunner 一次带产物的回放就逐步落盘 screenshots/page，回放后离线逐步解析 + OCR 重建。
///
/// 同时把**结构化执行轨迹**记进 conversation.json（事件类型 = `phase`）：本轮跑的完整脚本、
/// 逐步成败/错误/截图/页面结构路径/页面元素、是否到达目标——格式与探索阶段对称，供事无巨细复盘。
/// phase 区分阶段：诊断用 "doctor_diagnose"、稳定性测试用 "verify_stability"。
#[allow(clippy::too_many_arguments)]
pub(super) async fn diagnose(
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    lines: &[String],
    marker: &str,
    phase: &str,
    iter: usize,
    verbose: bool,
) -> Diagnosis {
    let check = strip_trailing_close(lines);
    if check.is_empty() {
        tx.log(phase, serde_json::json!({ "iter": iter, "script": check, "reached": false, "note": "空脚本", "steps": [] }));
        return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: "空脚本".into(), final_page: String::new() };
    }
    let _ = write_script(script_path, case, &check);
    reset_state(ctx.device, &check).await;

    // 一次带产物的完整回放：每步落盘 screenshots/step_NNN + page/step_NNN
    let log_root = ctx.artifacts.run_dir.join("doctor");
    let _ = std::fs::create_dir_all(&log_root);
    // 回放逐步进度：经 emit 走前端（Step 事件），不再裸 eprintln 撕裂 TUI。
    // StepStart 先发 Running（带预览），StepEnd 发 Ok/Fail（带耗时/错误）。
    let mut last_preview = String::new();
    let mut sink = |e: &RunEvent| {
        if !verbose {
            return;
        }
        match e {
            // 先显示「即将执行的指令」再操作设备，执行后接上 ✓/✗ + 耗时（与 tke run 对齐）
            RunEvent::StepStart { index, command, .. } => {
                last_preview = friendly(command);
                ctx.ui.emit(UiEvent::Step {
                    step: index + 1,
                    state: StepState::Running,
                    preview: last_preview.clone(),
                    line: None,
                    duration_ms: None,
                    error: None,
                });
            }
            RunEvent::StepEnd { index, success, error, duration_ms, .. } => {
                ctx.ui.emit(UiEvent::Step {
                    step: index + 1,
                    state: if *success { StepState::Ok } else { StepState::Fail },
                    preview: last_preview.clone(),
                    line: None,
                    duration_ms: Some(*duration_ms),
                    error: if *success { None } else { error.as_ref().map(|e| brief(e, 120)) },
                });
            }
            _ => {}
        }
    };
    let result = ScriptRunner::new(params.clone()).run(script_path, Some(&log_root), &mut sink).await;
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            tx.log(phase, serde_json::json!({ "iter": iter, "script": check, "reached": false, "note": format!("回放出错：{}", e), "steps": [] }));
            return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: format!("回放出错：{}", e), final_page: String::new() };
        }
    };
    let run_dir = result.run_dir.clone().map(PathBuf::from).unwrap_or_else(|| log_root.clone());

    // 离线逐步重建页面(结构 + OCR)，并记下每步产物路径
    let mut steps: Vec<DiagStep> = Vec::with_capacity(result.steps.len());
    for st in &result.steps {
        let mut full_txt = String::new();
        let mut struct_sig = 0u64;
        if let Some(xml_rel) = &st.xml {
            let xml_abs = run_dir.join(xml_rel);
            if let Ok(mut els) = ctx.fetcher.fetch_elements_from_file(&xml_abs) {
                struct_sig = struct_signature(&els); // 结构签名取 OCR 增强前的纯结构元素（OCR 不影响）
                if let (Some(src), Some(png_rel)) = (ctx.ocr, &st.screenshot) {
                    if let Ok(bytes) = std::fs::read(run_dir.join(png_rel)) {
                        let _ = enrich_with_ocr(&mut els, &bytes, src).await;
                    }
                }
                let none = vec![None; els.len()];
                full_txt = render_element_list(&els, &none, &ctx.prompts.message("explorer", "element_tag"));
            }
        }
        let abs = |rel: &Option<String>| rel.as_ref().map(|r| run_dir.join(r).to_string_lossy().to_string());
        steps.push(DiagStep {
            no: st.index + 1,
            line: st.command.clone(),
            ok: st.success,
            err: st.error.clone(),
            page_full: full_txt,
            struct_sig,
            screenshot: abs(&st.screenshot),
            xml: abs(&st.xml),
        });
    }

    let fail_idx = result.steps.iter().position(|s| !s.success);

    // 目标标志校验：全跑通才有意义(失败步中断时设备停在错页)。等渲染稳定后取一帧实时页面。
    // 同时把这帧最终页面渲染出来存 final_page，供医生 finish 时做"对照用户原话需求"的终点校验。
    let mut final_page = String::new();
    let reached = if result.success {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
            Ok(p) => {
                let none = vec![None; p.elements.len()];
                final_page = render_element_list(&p.elements, &none, &ctx.prompts.message("explorer", "element_tag"));
                marker.is_empty() || page_contains(&p, marker)
            }
            Err(_) => marker.is_empty(),
        }
    } else {
        // 失败中断：最终页取最后一步的页面
        final_page = steps.last().map(|s| s.page_full.clone()).unwrap_or_default();
        false
    };

    let note = if reached {
        format!("脚本全跑通，目标标志「{}」已出现 → 到达目标", brief(marker, 30))
    } else if let Some(k) = fail_idx {
        format!("第 {} 步失败，回放中断：{}", k + 1, brief(steps.get(k).and_then(|s| s.err.as_deref()).unwrap_or(""), 60))
    } else {
        format!("脚本全跑通，但目标标志「{}」始终未出现(说明走到了错的终点)", brief(marker, 30))
    };

    if verbose {
        if reached {
            ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 到达目标  {}", brief(marker, 40)) });
        } else {
            ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("✗ 未达目标  {}", brief(&note, 70)) });
        }
    }

    // 结构化执行轨迹进 conversation.json：本轮完整脚本 + 逐步成败/错误/截图/页面结构/页面元素 + 结论
    tx.log(
        phase,
        serde_json::json!({            "iter": iter,
            "script": check.clone(),
            "reached": reached,
            "fail_step": fail_idx.map(|k| k + 1),
            "marker": marker,
            "note": note.clone(),
            "steps": steps.iter().map(|s| serde_json::json!({
                "step": s.no,
                "line": friendly(&s.line),
                "ok": s.ok,
                "error": s.err,
                "screenshot": s.screenshot,
                "xml": s.xml,
                "page": s.page_full,
            })).collect::<Vec<_>>(),
        }),
    );
    Diagnosis { reached, steps, fail_idx, note, final_page }
}

/// 取文本前 n 行（每页元素列表可能很长，逐步展示时截断防 token 爆炸）
fn top_lines(s: &str, n: usize) -> String {
    let head = s.lines().take(n).collect::<Vec<_>>().join("\n");
    let extra = s.lines().count().saturating_sub(n);
    if extra > 0 {
        format!("{}\n…还有 {} 个元素未列出", head, extra)
    } else {
        head
    }
}

/// 多行页面文本统一缩进，嵌进 trace 时对齐
fn indent_page(s: &str) -> String {
    s.lines().map(|l| format!("      {}", l)).collect::<Vec<_>>().join("\n")
}

/// 把诊断结果渲染成给医生的提示词：编号脚本 + **本轮诊断回放每一步都带「该步执行后页面」**
/// （失败步/终点页给完整元素列表，其余步给前若干个），让医生看清整条脚本实际走了哪条路。
/// 历史轮次的 trace 由 `user_trace` 自动省略页面详情，故这里可以放心给全（每轮只有一份完整 trace 在上下文里）。
fn render_trace_prompt(prompts: &PromptSet, case: &str, marker: &str, diag: &Diagnosis) -> String {
    const PER_STEP_ELEMENTS: usize = 8; // 普通步每步展示的元素条数（失败步/终点页给全量）
    let last_no = diag.steps.last().map(|s| s.no).unwrap_or(0);
    // 逐步 trace 文本（数据，按规则组装；模板只提供外层说明文字）
    let mut steps = String::new();
    for st in &diag.steps {
        let mark = if st.ok { "✓" } else { "✗" };
        let err = st.err.as_deref().map(|e| format!("  ← 错误：{}", brief(e, 70))).unwrap_or_default();
        steps.push_str(&format!("{}. {} {}{}\n", st.no, mark, friendly(&st.line), err));
        if !st.page_full.trim().is_empty() {
            // 失败步、以及"全跑通但没到目标"时的终点步 → 完整页面；其余步 → 前若干个元素
            let critical = !st.ok || (diag.fail_idx.is_none() && !diag.reached && st.no == last_no);
            let page = if critical { st.page_full.clone() } else { top_lines(&st.page_full, PER_STEP_ELEMENTS) };
            steps.push_str(&format!("    页面：\n{}\n", indent_page(&page)));
        }
    }
    // 医生只管修到正确（路径优化交给反思官），目标固定为"修复"
    let objective = prompts.message("doctor", "trace_objective_fix");
    render(
        &prompts.message("doctor", "trace"),
        &[("case", case), ("marker", marker), ("note", &diag.note), ("steps", &steps), ("objective", &objective)],
    )
}

// ===================== 行编辑 + 校验 =====================

/// 校验一行 .tks 是否可安全引入：能解析，且其中所有**元素引用**都已存在于元素库。
/// 坐标(Coordinate)不算元素引用、不校验。返回 Err(原因) 表示不安全。
fn validate_line(content: &str, element_path: &Path) -> std::result::Result<(), String> {
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
fn parse_edit(call: &LlmToolCall) -> std::result::Result<EditOp, String> {
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

// ===================== 元素重选(reexplore + pick) =====================

/// 重启净化 + 回放 lines[0..cut]，把设备定位到第 cut 步后的页面（即第 cut+1 步将操作的页面）。
async fn reposition(ctx: &DriveCtx<'_>, params: &Arc<Params>, script_path: &Path, case: &str, lines: &[String], cut: usize) {
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
fn tier_for(el: &UIElement) -> (Option<&UIElement>, OcrChannel) {
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
fn build_action_line(action: &str, name: &str, text: Option<&str>) -> std::result::Result<String, String> {
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

// ===================== 主循环 =====================

/// 脚本医生主循环：把 lines 修到稳定到达目标(顺带提炼最短)。
/// 返回**最短的达标版本**；始终修不好则返回 None(上层据此判失败、回滚)。
#[allow(clippy::too_many_arguments)]
pub(super) async fn doctor_repair(
    ai: &AiConfig,
    prompts: &PromptSet,
    txp: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    marker: &str,
    mut lines: Vec<String>,
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    // 进入「脚本医生 agent」作用域：本函数产出的事件（会话/请求/编辑/诊断/重选元素）都归属 doctor。
    let mut _dscope = txp.scoped("doctor");
    let tx = &mut *_dscope;

    // 独立医生会话(独立工具集)。系统提示词与工具描述均走 PromptSet（内置 md，可外部覆盖）。
    // 其 token 单独累计、最后并入总量(报告 extra_*)。
    let platform = Platform::from_device(Some(ctx.device));
    let system = prompts.role_system("doctor", ctx.device, platform.name());
    let tools = build_doctor_tools(prompts);
    let mut editor = match LlmSession::new(ai, system.clone(), tools.clone()) {
        Ok(s) => s,
        Err(e) => {
            ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("脚本医生会话创建失败：{}", e) });
            return None;
        }
    };
    // 记录医生会话(独立 agent)的系统提示词 + 工具定义，便于在 conversation.json 里按 agent 复盘
    tx.log(
        "doctor_session",
        serde_json::json!({            "model": editor.model(),
            "system_prompt": system,
            "tools": tools.iter().map(|t| serde_json::json!({ "name": t.name, "description": t.description, "schema": t.schema })).collect::<Vec<_>>(),
        }),
    );

    let mut stagnation = 0usize; // 连续「没改动且没达标」次数
    let mut pending: Option<PendingReselect> = None; // reexplore 定位后暂存的实时页面（供 pick）
    let mut finish_rechecks = 0usize; // finish 终点校验被打回的次数（防无限）

    for iter in 1..=params.harness.doctor_iters {
        if super::interrupt::aborted() {
            ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已中断（Ctrl+C），停止医生修复".into() });
            return None;
        }
        ctx.ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
        ctx.ui.emit(UiEvent::Notice { level: Level::Info, text: format!("▶ 诊断回放（第 {} 轮，重启净化中…）", iter) });
        let diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;

        // 医生只管**正确性**：一旦能跑通且到达目标，立即把正确脚本交回上层（路径优化是反思官的事）
        if diag.reached {
            tx.log("doctor_reached", serde_json::json!({ "iter": iter, "steps": lines.len() }));
            return Some(lines);
        }

        let prompt = render_trace_prompt(prompts, case, marker, &diag);
        tx.log("doctor_request", serde_json::json!({ "iter": iter, "reached": false, "prompt": prompt.clone() }));
        // user_trace：自动省略上一轮 trace 的页面详情，只保留最新一份完整 trace（防上下文暴涨）
        editor.user_trace(prompt);

        // 内层：收医生的编辑动作，直到它 run / finish / 触发重新诊断
        let lines_before = lines.clone();
        let mut did_reexplore = false;
        let mut edit_calls = 0usize;
        let mut go_finish = false;
        loop {
            // 安全点：取前端命令（Abort 终止；Guidance 即时注入医生会话；Pause 软停等用户给医生建议再继续）。
            // 回放/滚动查找被 Esc 软停后控制权回到本循环，下一轮顶部就能 drain 到 Pause（仿 flow.rs）。
            for c in ctx.ui.drain_commands() {
                match c {
                    UiCommand::Abort => {
                        ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已终止（用户中断），停止医生修复".into() });
                        return None;
                    }
                    UiCommand::Guidance { text } => {
                        let m = format!("【用户给诊断医生的建议】{}\n请据此调整诊断/修复方向。", text);
                        tx.log("user_guidance", serde_json::json!({ "iter": iter, "content": m.clone() }));
                        editor.user(m);
                        ctx.ui.emit(UiEvent::GuidanceAccepted { text });
                    }
                    UiCommand::Pause => {
                        super::interrupt::clear_pause(); // 已响应软停，清标志（回放/诊断动作已停下）
                        match ctx
                            .ui
                            .await_answer(0, "已暂停。给诊断医生一条建议（或回车让它自行诊断），再按 Ctrl+C 退出".to_string())
                            .await
                        {
                            Some(g) if !g.trim().is_empty() => {
                                let m = format!("【用户给诊断医生的建议】{}\n请据此调整诊断/修复方向。", g);
                                tx.log("user_guidance", serde_json::json!({ "iter": iter, "content": m.clone() }));
                                editor.user(m);
                                ctx.ui.emit(UiEvent::GuidanceAccepted { text: g });
                            }
                            Some(_) => {}
                            None => {
                                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已终止（用户中断），停止医生修复".into() });
                                return None;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if super::interrupt::aborted() {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "已中断（Ctrl+C），停止医生修复".into() });
                return None;
            }
            if edit_calls >= MAX_EDITS_PER_ITER {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "医生单轮编辑过多，强制重新诊断".into() });
                break;
            }
            edit_calls += 1;
            let reply = match editor.next().await {
                Ok(r) => r,
                Err(e) => {
                    ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("医生决策出错：{}", e) });
                    return None;
                }
            };
            let (pt, ct) = editor.last_usage();
            // 医生会话独立计 token，累进报告的 extra（最终并入总量统计）
            report.extra_prompt += pt;
            report.extra_completion += ct;

            let (text, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 160);
                    ctx.ui.emit(UiEvent::SubAgent {
                        kind: SubAgent::Doctor,
                        level: Level::Dim,
                        text: if b.is_empty() { "（未调用工具）".into() } else { b },
                        tokens: Tokens::new(pt, ct),
                    });
                    let m = prompts.message("doctor", "nudge_use_tool");
                    tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
                    editor.user(m);
                    continue;
                }
                LlmReply::ToolCalls { text, calls } => (text, calls),
            };
            // 协议：所有 tool_call 都要回执，只处理第一个
            let primary = calls[0].clone();
            for extra in &calls[1..] {
                editor.tool_result(extra.call_id.as_str(), "已忽略：每轮仅处理第一个工具调用");
            }
            let op = match parse_edit(&primary) {
                Ok(v) => v,
                Err(e) => {
                    editor.tool_result(primary.call_id.as_str(), format!("参数错误：{}", e));
                    continue;
                }
            };
            // 一行思考：理由优先(reason 字段)，其次模型同时给的文字；工具名用紫色标签区分
            let reason = primary
                .arguments
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let say = reason
                .map(|s| brief(s, 180))
                .or_else(|| text.as_deref().map(|s| brief(s, 180)).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "（未说明理由）".to_string());
            ctx.ui.emit(UiEvent::SubAgent {
                kind: SubAgent::Doctor,
                level: Level::Info,
                text: format!("⟫ {}：{}", primary.name, say),
                tokens: Tokens::new(pt, ct),
            });
            // 编辑前脚本全貌（变更后由各 arm 记 doctor_edit_applied）
            let script_before = lines.clone();
            tx.log(
                "doctor_edit",
                serde_json::json!({ "iter": iter, "tool": primary.name.clone(), "reason": reason, "thinking": text.clone(), "args": primary.arguments.clone(), "script_before": script_before }),
            );

            match op {
                EditOp::Delete { from, to } => {
                    let n = lines.len();
                    if from < 1 || to < from || from > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法删 {}..={}", n, from, to));
                        continue;
                    }
                    let to = to.min(n);
                    // 保护启动步：删除范围内若含「启动」步则拒绝——删了浏览器/App 再也起不来、重启净化也空转。
                    if lines[(from - 1)..to].iter().any(|l| is_launch_line(l)) {
                        editor.tool_result(primary.call_id.as_str(), "删除范围里含「启动」步——它打开浏览器/拉起 App，删了脚本就再也起不来、重启净化也会空转。请缩小范围、保留启动步。");
                        continue;
                    }
                    let removed: Vec<String> = lines.drain((from - 1)..to).collect();
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 删第 {}-{} 行  {}", from, to, removed.iter().map(|l| friendly(l)).collect::<Vec<_>>().join(" / ")) });
                    editor.tool_result(primary.call_id.as_str(), format!("已删第 {}-{} 行，脚本现 {} 行。改完记得 run 验证。", from, to, lines.len()));
                }
                EditOp::Replace { line, content } => {
                    let n = lines.len();
                    if line < 1 || line > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法替换第 {} 行", n, line));
                        continue;
                    }
                    // 保护启动步：启动步只能改成另一个「启动」(如换网址/App)，不能改成点击/其它，否则浏览器/App 起不来。
                    if is_launch_line(&lines[line - 1]) && !is_launch_line(&content) {
                        editor.tool_result(primary.call_id.as_str(), format!("第 {} 步是「启动」步，只能改成另一个「启动 …」(如换网址/App)，不能改成点击/其它——否则浏览器/App 起不来。若是导航不对，请 insert_after 在它之后插入导航步。", line));
                        continue;
                    }
                    if let Err(why) = validate_line(&content, ctx.element_path) {
                        editor.tool_result(primary.call_id.as_str(), format!("替换被拒：{}", why));
                        continue;
                    }
                    let old = std::mem::replace(&mut lines[line - 1], content.trim().to_string());
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 改第 {} 行  {} → {}", line, friendly(&old), friendly(&lines[line - 1])) });
                    editor.tool_result(primary.call_id.as_str(), format!("已替换第 {} 行。改完记得 run 验证。", line));
                }
                EditOp::Insert { after, content } => {
                    let n = lines.len();
                    if after > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法在第 {} 行后插入", n, after));
                        continue;
                    }
                    if let Err(why) = validate_line(&content, ctx.element_path) {
                        editor.tool_result(primary.call_id.as_str(), format!("插入被拒：{}", why));
                        continue;
                    }
                    lines.insert(after, content.trim().to_string());
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 在第 {} 行后插入  {}", after, friendly(&lines[after])) });
                    editor.tool_result(primary.call_id.as_str(), format!("已插入，脚本现 {} 行。改完记得 run 验证。", lines.len()));
                }
                EditOp::Reexplore { step, reason } => {
                    if report.repairs >= params.harness.repairs {
                        editor.tool_result(primary.call_id.as_str(), format!("已达重选上限（{} 次），请改用文本编辑或 finish。", params.harness.repairs));
                        continue;
                    }
                    let n = lines.len();
                    if step < 1 || step > n {
                        editor.tool_result(primary.call_id.as_str(), format!("step 越界：脚本共 {} 行", n));
                        continue;
                    }
                    // 保护启动步：第 step 步是「启动」(打开浏览器/拉起 App)时不能 reexplore——
                    // reexplore 之后的 pick 会用点击覆盖这一步，毁掉脚本入口、后续重启净化空转。
                    if is_launch_line(&lines[step - 1]) {
                        editor.tool_result(primary.call_id.as_str(), format!(
                            "第 {} 步是「启动」步(打开浏览器/拉起 App)，不能 reexplore 重选成点击——\
                             那会用点击覆盖启动步、毁掉脚本入口，后续重启净化也会因找不到启动目标而空转。\
                             若是「启动后导航入口不对」：保留这一步，用 insert_after 在它之后插入正确的导航步；\
                             若要换网址/App：用 replace_line 把它改成另一个「启动 …」。", step));
                        continue;
                    }
                    report.repairs += 1;
                    let cut = step - 1; // 回放到目标步的前一步
                    tx.log("doctor_reexplore", serde_json::json!({ "iter": iter, "step": step, "kept_prefix": cut, "reason": reason }));
                    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("◆ 重新定位到第 {} 步（重启+回放前 {} 步）：{}", step, cut, brief(&reason, 50)) });
                    reposition(ctx, params, script_path, case, &lines, cut).await;
                    // fetch 实时页面，交给医生重选
                    match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
                        Ok(p) => {
                            let none = vec![None; p.elements.len()];
                            let list = render_element_list(&p.elements, &none, &prompts.message("explorer", "element_tag"));
                            let count = p.elements.len();
                            let shot = p.shot_path.clone();
                            pending = Some(PendingReselect { step, elements: p.elements, shot_path: shot.clone() });
                            editor.tool_result(primary.call_id.as_str(), "已定位到该步实时页面（元素列表 + 截图见下一条消息）");
                            // 同时发**元素列表 + 截图**：列表里有就 pick(id)，列表里没有/反复点不中（纯图标、
                            // 滑动没到位）就看截图用 pick_visual 给像素框/点。
                            let msg = format!(
                                "已回放到第 {} 步、设备停在第 {} 步将操作的实时页面。\n实时元素（共 {} 个）：\n{}\n\n\
                                 请为第 {} 步选正确目标：\n\
                                 · 目标在上面列表里 → pick(id, name, action)；\n\
                                 · 列表里没有、或之前反复点这个元素都没反应（很可能是纯图标、或目标被滑出屏幕外）→ \
                                 看下面的截图用 pick_visual 给像素框 region=[x1,y1,x2,y2]（优先）或点 (x,y)。\n\
                                 input 动作再给 text。",
                                cut, step, count, list, step
                            );
                            tx.log("llm_message", serde_json::json!({ "iter": iter, "content": msg.clone(), "image": shot.to_string_lossy() }));
                            if editor.user_with_image(&msg, &shot).is_err() {
                                editor.user(msg);
                            }
                        }
                        Err(e) => {
                            editor.tool_result(primary.call_id.as_str(), format!("定位后采集页面失败：{}。可重试 reexplore 或改用文本编辑。", e));
                        }
                    }
                    continue; // 等医生 pick / pick_visual；不重诊断
                }
                EditOp::Pick { id, name, action, text } => {
                    let Some(pend) = pending.as_ref() else {
                        editor.tool_result(primary.call_id.as_str(), "还没定位：请先 reexplore 到要改的步骤，拿到实时元素再 pick。");
                        continue;
                    };
                    let Some(el) = pend.elements.get(id) else {
                        editor.tool_result(primary.call_id.as_str(), format!("id 越界：该页共 {} 个元素", pend.elements.len()));
                        continue;
                    };
                    // 元素名按特征自动生成（与探索一致），医生不必起名；AI 传的 name 忽略
                    let _ = &name;
                    let eff_name = auto_name(el);
                    let line = match build_action_line(&action, &eff_name, text.as_deref()) {
                        Ok(l) => l,
                        Err(why) => {
                            editor.tool_result(primary.call_id.as_str(), why);
                            continue;
                        }
                    };
                    // 实时落库（从 reexplore 那次 capture 写入工作区的截图裁 img）
                    let (structure, ocr) = tier_for(el);
                    if let Err(e) =
                        add_element_target(ctx.device.to_string(), ctx.element_path, &eff_name, None, el.bounds.clone(), structure, ocr, false).await
                    {
                        editor.tool_result(primary.call_id.as_str(), format!("元素落库失败：{}", e));
                        continue;
                    }
                    if !report.created.contains(&eff_name) {
                        report.created.push(eff_name.clone());
                    }
                    let step_idx = pend.step - 1;
                    let old = lines.get(step_idx).cloned().unwrap_or_default();
                    if step_idx < lines.len() {
                        lines[step_idx] = line.clone();
                    }
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 重选第 {} 步元素  {} → {}", pend.step, friendly(&old), friendly(&line)) });
                    tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": "pick", "step": pend.step, "name": eff_name, "script_after": lines.clone() }));
                    editor.tool_result(primary.call_id.as_str(), format!("已把第 {} 步改为「{}」（元素「{}」已实时存库）。请 run 验证。", pend.step, friendly(&line), eff_name));
                    pending = None;
                    did_reexplore = true;
                    break; // 重选后重新诊断
                }
                EditOp::PickVisual { region, x, y, name, action, text } => {
                    let _ = &name; // 元素名按特征(坐标)自动生成，AI 传的 name 忽略
                    let Some(pend) = pending.as_ref() else {
                        editor.tool_result(primary.call_id.as_str(), "还没定位：请先 reexplore 到要改的步骤，看到截图再 pick_visual。");
                        continue;
                    };
                    // 看图框选 → 像素 bounds（region 优先；否则以 (x,y) 取屏宽 15% 方块），落纯 img 元素
                    let (sw, sh) = image::image_dimensions(&pend.shot_path).unwrap_or((1080, 1920));
                    let bounds = match region {
                        Some([x1, y1, x2, y2]) => crate::Bounds::new(x1, y1, x2, y2),
                        None => {
                            let cx = x.unwrap_or(sw as i32 / 2);
                            let cy = y.unwrap_or(sh as i32 / 2);
                            let half = (sw as i32 * 15 / 100).max(20) / 2;
                            crate::Bounds::new(cx - half, cy - half, cx + half, cy + half)
                        }
                    };
                    let name = visual_auto_name(&bounds); // 自动命名
                    let line = match build_action_line(&action, &name, text.as_deref()) {
                        Ok(l) => l,
                        Err(why) => {
                            editor.tool_result(primary.call_id.as_str(), why);
                            continue;
                        }
                    };
                    // 三级·仅视觉：结构空、ocr 空、仅 img 模板（从 reexplore 那帧截图裁）
                    if let Err(e) =
                        add_element_target(ctx.device.to_string(), ctx.element_path, &name, None, bounds, None, OcrChannel::None, false).await
                    {
                        editor.tool_result(primary.call_id.as_str(), format!("视觉元素落库失败：{}", e));
                        continue;
                    }
                    if !report.created.contains(&name) {
                        report.created.push(name.clone());
                    }
                    let step_idx = pend.step - 1;
                    let old = lines.get(step_idx).cloned().unwrap_or_default();
                    if step_idx < lines.len() {
                        lines[step_idx] = line.clone();
                    }
                    ctx.ui.emit(UiEvent::Notice { level: Level::Ok, text: format!("✓ 视觉重选第 {} 步元素  {} → {}", pend.step, friendly(&old), friendly(&line)) });
                    tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": "pick_visual", "step": pend.step, "name": name, "script_after": lines.clone() }));
                    editor.tool_result(primary.call_id.as_str(), format!("已把第 {} 步改为视觉点击「{}」（img 模板已存库）。请 run 验证。", pend.step, friendly(&line)));
                    pending = None;
                    did_reexplore = true;
                    break;
                }
                EditOp::Run => {
                    editor.tool_result(primary.call_id.as_str(), "重新回放诊断中，稍后给你最新 trace。");
                    break;
                }
                EditOp::Finish { reason } => {
                    editor.tool_result(primary.call_id.as_str(), "收到收尾请求，系统再诊断一次兜底校验。");
                    tx.log("doctor_finish", serde_json::json!({ "iter": iter, "reason": reason }));
                    go_finish = true;
                    break;
                }
            }
            // 走到这里 = 文本编辑(删/改/插)成功落地（失败/越界已 continue、reexplore/run/finish 已 break）：
            // 记录变更后脚本全貌，与 doctor_edit 的 script_before 对照。
            tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": primary.name.clone(), "script_after": lines.clone() }));
        }

        // 收尾：finish 后再诊断一次确认是否真达标；达标即返回正确脚本，否则打回继续修
        if go_finish {
            ctx.ui.emit(UiEvent::Notice { level: Level::Info, text: "▶ 收尾兜底诊断（重启净化中…）".into() });
            let final_diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;
            if final_diag.reached {
                // 终点校验：把最终页面 + 用户原话需求发给医生判断是否真到对的地方（marker 命中只是文本，
                // 这里再做一层语义判断，防"标志在但页面不对"）。被打回有上限，避免无限。
                if finish_rechecks < 2 && !final_diag.final_page.trim().is_empty() {
                    finish_rechecks += 1;
                    let check = render(&prompts.message("doctor", "finish_check"), &[("case", case), ("page", &final_diag.final_page)]);
                    tx.log("llm_message", serde_json::json!({ "iter": iter, "content": check.clone() }));
                    editor.user(check);
                    if let Ok(LlmReply::Text(t)) = editor.next().await {
                        let (pt, ct) = editor.last_usage();
                        report.extra_prompt += pt;
                        report.extra_completion += ct;
                        tx.log("finish_check", serde_json::json!({ "iter": iter, "content": t.clone() }));
                        if let Some(obj) = parse_desc_json(&t) {
                            if obj["done"].as_bool() == Some(false) {
                                let why = obj["reason"].as_str().unwrap_or("最终页面不符合用户需求").to_string();
                                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("终点校验未通过：{}", brief(&why, 80)) });
                                let m = format!("终点校验未通过：{}。脚本最终到的页面不是用户要的目的地，请继续修（看下面的最新 trace）。", why);
                                tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
                                editor.user(m);
                                continue; // 不收尾，继续修
                            }
                        }
                    }
                }
                tx.log("doctor_reached", serde_json::json!({ "iter": iter, "steps": lines.len() }));
                return Some(lines);
            }
            let m = prompts.message("doctor", "finish_pushback");
            tx.log("llm_message", serde_json::json!({ "iter": iter, "content": m.clone() }));
            editor.user(m);
            // 落到下一轮
        }

        // 停滞检测：本轮没改动、没重探、且没达标 → 计一次停滞；连续 2 次就收手（修不动了）
        let unchanged = lines == lines_before;
        if unchanged && !did_reexplore && !diag.reached {
            stagnation += 1;
            if stagnation >= 2 {
                ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: "医生连续两轮无有效改动且未达标，放弃修复".into() });
                return None;
            }
        } else {
            stagnation = 0;
        }
    }

    ctx.ui.emit(UiEvent::Notice { level: Level::Warn, text: format!("已达医生诊断轮数上限（{} 轮）仍未修到目标", params.harness.doctor_iters) });
    None
}
