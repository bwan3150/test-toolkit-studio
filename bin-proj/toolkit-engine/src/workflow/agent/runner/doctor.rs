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

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::engines::ocr::enrich_with_ocr;
use crate::{AiConfig, LlmReply, LlmSession, LlmTool, LlmToolCall, Params, Platform, RunEvent, ScriptParser, ScriptRunner, TksParam};

use super::super::execution::script::write_script;
use super::super::perception::{capture, render_element_list};
use super::super::prompt::PromptSet;
use super::super::transcript::Transcript;
use super::flow::{brief, drive, fmt_tokens, friendly, paint, DriveCtx};
use super::options::VerifyReport;
use super::verify::{do_replay, page_contains, reset_state, strip_trailing_close};

/// 医生主循环最多诊断几轮(每轮 = 一次完整诊断回放 + 一批编辑)
const MAX_DOCTOR_ITERS: usize = 10;
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
    /// 该步标注截图路径（落盘产物，写入 conversation.json 供复盘）
    screenshot: Option<String>,
    /// 该步页面结构文件路径
    xml: Option<String>,
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
    /// 从第 from_line 步起活体重探(回放前缀定位设备 + 现场重新导航)
    Reexplore { from_line: usize, reason: String },
    /// 重新诊断回放(测试当前编辑效果)
    Run,
    /// 收尾(医生认为已达标且最短)
    Finish { reason: String },
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
                    "from_line": { "type": "integer", "description": "从第几步起重新导航(1-based)。应选**真正开始走错**的那一步，通常比报错步更早。" },
                    "reason": { "type": "string", "description": "为什么这段要活体重探(看富 trace 说清楚哪步跑偏了)" }
                },
                "required": ["from_line", "reason"]
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
    let tty = std::io::stderr().is_terminal();
    let check = strip_trailing_close(lines);
    if check.is_empty() {
        tx.log(phase, serde_json::json!({ "iter": iter, "script": check, "reached": false, "note": "空脚本", "steps": [] }));
        return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: "空脚本".into() };
    }
    let _ = write_script(script_path, case, &check);
    reset_state(ctx.device, &check).await;

    // 一次带产物的完整回放：每步落盘 screenshots/step_NNN + page/step_NNN
    let log_root = ctx.artifacts.run_dir.join("doctor");
    let _ = std::fs::create_dir_all(&log_root);
    let mut total = 0usize;
    let mut sink = |e: &RunEvent| {
        if !verbose {
            return;
        }
        match e {
            RunEvent::RunStart { total_steps, .. } => total = *total_steps,
            RunEvent::StepEnd { index, command, success, error, .. } => {
                if *success {
                    eprintln!("    {}  {}  {}", paint(tty, "32", "✓"), paint(tty, "2", &format!("步 {}/{}", index + 1, total)), friendly(command));
                } else {
                    eprintln!(
                        "    {}  {}",
                        paint(tty, "31", &format!("✗ 步 {}/{}  {}", index + 1, total, friendly(command))),
                        paint(tty, "31", &format!("— {}", brief(error.as_deref().unwrap_or(""), 80))),
                    );
                }
            }
            _ => {}
        }
    };
    let result = ScriptRunner::new(params.clone()).run(script_path, Some(&log_root), &mut sink).await;
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            tx.log(phase, serde_json::json!({ "iter": iter, "script": check, "reached": false, "note": format!("回放出错：{}", e), "steps": [] }));
            return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: format!("回放出错：{}", e) };
        }
    };
    let run_dir = result.run_dir.clone().map(PathBuf::from).unwrap_or_else(|| log_root.clone());

    // 离线逐步重建页面(结构 + OCR)，并记下每步产物路径
    let mut steps: Vec<DiagStep> = Vec::with_capacity(result.steps.len());
    for st in &result.steps {
        let mut full_txt = String::new();
        if let Some(xml_rel) = &st.xml {
            let xml_abs = run_dir.join(xml_rel);
            if let Ok(mut els) = ctx.fetcher.fetch_elements_from_file(&xml_abs) {
                if let (Some(src), Some(png_rel)) = (ctx.ocr, &st.screenshot) {
                    if let Ok(bytes) = std::fs::read(run_dir.join(png_rel)) {
                        let _ = enrich_with_ocr(&mut els, &bytes, src).await;
                    }
                }
                let none = vec![None; els.len()];
                full_txt = render_element_list(&els, &none);
            }
        }
        let abs = |rel: &Option<String>| rel.as_ref().map(|r| run_dir.join(r).to_string_lossy().to_string());
        steps.push(DiagStep {
            no: st.index + 1,
            line: st.command.clone(),
            ok: st.success,
            err: st.error.clone(),
            page_full: full_txt,
            screenshot: abs(&st.screenshot),
            xml: abs(&st.xml),
        });
    }

    let fail_idx = result.steps.iter().position(|s| !s.success);

    // 目标标志校验：全跑通才有意义(失败步中断时设备停在错页)。等渲染稳定后取一帧实时页面。
    let reached = if result.success {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if marker.is_empty() {
            true
        } else {
            matches!(capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await, Ok(p) if page_contains(&p, marker))
        }
    } else {
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
            eprintln!("    {}  {}", paint(tty, "32", "✓ 到达目标"), brief(marker, 40));
        } else {
            eprintln!("    {}  {}", paint(tty, "31", "✗ 未达目标"), brief(&note, 70));
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
    Diagnosis { reached, steps, fail_idx, note }
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
fn render_trace_prompt(case: &str, marker: &str, diag: &Diagnosis, objective_minimize: bool) -> String {
    const PER_STEP_ELEMENTS: usize = 8; // 普通步每步展示的元素条数（失败步/终点页给全量）
    let last_no = diag.steps.last().map(|s| s.no).unwrap_or(0);
    let mut s = String::new();
    s.push_str(&format!("整体测试目标：{}\n目标标志（只有真到达目标才会出现的文字）：「{}」\n\n", case, marker));
    s.push_str(&format!("【本轮诊断结论】{}\n\n", diag.note));
    s.push_str(
        "【本轮逐步 trace】每步 = 步号 · 成败 · .tks 行，其下「页面」是该步执行后实际所处页面的元素。\
         请顺着每步页面看这条脚本到底走了哪条路——报错那一步往往不是根因，更早某步「页面没变/走到了错页面」才是真正跑偏处。\n\n",
    );
    for st in &diag.steps {
        let mark = if st.ok { "✓" } else { "✗" };
        let err = st.err.as_deref().map(|e| format!("  ← 错误：{}", brief(e, 70))).unwrap_or_default();
        s.push_str(&format!("{}. {} {}{}\n", st.no, mark, friendly(&st.line), err));
        if !st.page_full.trim().is_empty() {
            // 失败步、以及"全跑通但没到目标"时的终点步 → 完整页面；其余步 → 前若干个元素
            let critical = !st.ok || (diag.fail_idx.is_none() && !diag.reached && st.no == last_no);
            let page = if critical { st.page_full.clone() } else { top_lines(&st.page_full, PER_STEP_ELEMENTS) };
            s.push_str(&format!("    页面：\n{}\n", indent_page(&page)));
        }
    }

    if objective_minimize {
        s.push_str(
            "\n当前脚本**已能到达目标**。现在请尽量**删冗余步**让它最短（重复点击、走错绕回、点了没用的空步、\
             连续多余滑动）。每删完用 run 确认仍到达目标；删错导致目标丢失会被自动还原。确认没有更多可删时 finish。",
        );
    } else {
        s.push_str(
            "\n请结合每步页面判断**真正开始跑偏的那一步**（常比报错步更早），用最小改动修好它：能删冗余/改一行参数就别 reexplore，\
             确实需要点全新元素/走全新路径才 reexplore。改完用 run 看效果。",
        );
    }
    s
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
        for p in &step.params {
            if let TksParam::Element { name, .. } = p {
                if lib["elements"].get(name).is_none() {
                    return Err(format!(
                        "元素「{}」不在元素库中——replace/insert 只能引用已有元素；要点新元素/走新路径请改用 reexplore",
                        name
                    ));
                }
            }
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
        "reexplore" => Ok(EditOp::Reexplore {
            from_line: uint("from_line").ok_or("缺少 from_line")?,
            reason: string("reason").unwrap_or_default(),
        }),
        "run" => Ok(EditOp::Run),
        "finish" => Ok(EditOp::Finish { reason: string("reason").unwrap_or_default() }),
        other => Err(format!("未知工具：{}", other)),
    }
}

// ===================== 活体逃生口(reexplore) =====================

/// 从第 from_line 步起活体重探：回放前缀把设备定位到位 + 用探索会话现场重新导航 + 拼回。
/// 复用探索会话(有设备知识 + 探索工具)。成功返回新脚本；失败/中断返回 None。
#[allow(clippy::too_many_arguments)]
async fn reexplore_segment(
    explore_sess: &mut LlmSession,
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    lines: &[String],
    from_line: usize,
    reason: &str,
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    let tty = std::io::stderr().is_terminal();
    let d = from_line.clamp(1, lines.len() + 1);
    let cut = (d - 1).min(lines.len());
    let mut prefix: Vec<String> = lines[..cut].to_vec();
    report.repairs += 1;

    tx.log(
        "doctor_reexplore",
        serde_json::json!({ "repair": report.repairs, "from_line": d, "kept_prefix_steps": cut, "reason": reason }),
    );
    eprintln!();
    eprintln!(
        "  {}",
        paint(tty, "1;33", &format!("◆ 活体重探(第 {} 次)：从第 {} 步起重新导航，保留前 {} 步（{}）", report.repairs, d, cut, brief(reason, 50)))
    );

    // 回放前缀把设备定位到第 cut 步后的页面(前缀含「启动」，先净化关闭即可)
    reset_state(ctx.device, lines).await;
    if !prefix.is_empty() {
        let _ = write_script(script_path, case, &prefix);
        let _ = do_replay(params, script_path, false).await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let preamble = format!(
        "现在进入【活体重探】。已把脚本前 {} 步重新跑了一遍、设备停在那之后的页面。\
         脚本医生判断从第 {} 步起要重新导航（原因：{}）。请从**当前页面**出发，重新找到正确做法、\
         把剩下的测试目标走完，最后 finish。（只产出从这里往后的操作，前 {} 步会自动保留。）整体测试目标：{}",
        cut, d, reason, cut, case
    );
    explore_sess.user(preamble);
    let tail = match drive(explore_sess, tx, ctx, false, "重探·").await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("  {}", paint(tty, "31", &format!("活体重探出错：{}", e)));
            return None;
        }
    };
    for c in tail.created {
        if !report.created.contains(&c) {
            report.created.push(c);
        }
    }
    for u in tail.updated {
        if !report.updated.contains(&u) {
            report.updated.push(u);
        }
    }
    if tail.aborted {
        eprintln!("  {}", paint(tty, "33", "已终止（用户中断）"));
        return None;
    }
    if !tail.success {
        tx.log("doctor_reexplore_failed", serde_json::json!({ "repair": report.repairs, "reason": tail.reason }));
        eprintln!("  {}", paint(tty, "33", "活体重探未能达成测试目标（探索引擎也没走通）"));
        return None;
    }
    // 改名同步到前缀(库 key 已变，否则回放找不到)
    for (old, new) in &tail.renames {
        let (from, to) = (format!("{{{}}}", old), format!("{{{}}}", new));
        for l in prefix.iter_mut() {
            if l.contains(&from) {
                *l = l.replace(&from, &to);
            }
        }
    }
    let tail_len = tail.lines.len();
    prefix.extend(tail.lines);
    eprintln!("  {}", paint(tty, "1;32", &format!("✓ 活体重探完毕：续接 {} 步，新脚本共 {} 步", tail_len, prefix.len())));
    eprintln!();
    Some(prefix)
}

// ===================== 主循环 =====================

/// 脚本医生主循环：把 lines 修到稳定到达目标(顺带提炼最短)。
/// 返回**最短的达标版本**；始终修不好则返回 None(上层据此判失败、回滚)。
#[allow(clippy::too_many_arguments)]
pub(super) async fn doctor_repair(
    explore_sess: &mut LlmSession,
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
    // 进入「脚本医生 agent」作用域：本函数产出的事件（会话/请求/编辑/诊断）都归属 doctor。
    // 其中 reexplore→drive() 会再压入 explorer 作用域、返回后弹回 doctor（嵌套由栈处理）。
    let mut _dscope = txp.scoped("doctor");
    let tx = &mut *_dscope;
    let tty = std::io::stderr().is_terminal();

    // 独立医生会话(独立工具集)。系统提示词与工具描述均走 PromptSet（内置 md，可外部覆盖）。
    // 其 token 单独累计、最后并入总量(报告 extra_*)。
    let platform = Platform::from_device(Some(ctx.device));
    let system = prompts.role_system("doctor", ctx.device, platform.name());
    let tools = build_doctor_tools(prompts);
    let mut editor = match LlmSession::new(ai, system.clone(), tools.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  {}", paint(tty, "31", &format!("脚本医生会话创建失败：{}", e)));
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

    // 最短的达标版本(护栏：删坏了自动还原到它)
    let mut best: Option<Vec<String>> = None;
    let mut stagnation = 0usize; // 连续「没改动且没达标」次数

    for iter in 1..=MAX_DOCTOR_ITERS {
        eprintln!();
        eprintln!("  {}", paint(tty, "1;36", &format!("▶ 诊断回放（第 {} 轮，重启净化中…）", iter)));
        let diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;

        // 维护 best + 删坏自动还原
        if diag.reached {
            let shorter = best.as_ref().map(|b| lines.len() < b.len()).unwrap_or(true);
            if shorter {
                best = Some(lines.clone()); // 保留结尾「关闭」步，落盘版本含 close
            }
        } else if best.is_some() {
            // 曾经达标、现在又不达标 → 上一批改动把它改坏了，自动还原
            let b = best.clone().unwrap();
            eprintln!("  {}", paint(tty, "33", &format!("上一批改动导致目标丢失，已自动还原到上一个达标版本（{} 步）", b.len())));
            tx.log("doctor_auto_revert", serde_json::json!({ "iter": iter, "restored_steps": b.len() }));
            lines = b;
            editor.user(format!(
                "你上一批改动导致**目标丢失**，系统已自动还原到上一个达标版本（{} 步）。\
                 这个版本已经能到达目标。如果你是在删冗余，请换更保守的删法（一次删更少）或直接 finish。",
                lines.len()
            ));
            continue;
        }

        let objective_minimize = best.is_some();
        let prompt = render_trace_prompt(case, marker, &diag, objective_minimize);
        // 记录真实发送给医生的整段 prompt（含逐步 trace），便于复盘 & 迭代上下文压缩
        tx.log(
            "doctor_request",
            serde_json::json!({                "iter": iter,
                "objective": if objective_minimize { "minimize" } else { "fix" },
                "reached": diag.reached,
                "prompt": prompt.clone(),
            }),
        );
        // user_trace：自动省略上一轮 trace 的页面详情，只保留最新一份完整 trace（防上下文暴涨）
        editor.user_trace(prompt);
        eprintln!(); // 诊断输出与医生编辑之间空一行，分隔更清楚

        // 内层：收医生的编辑动作，直到它 run / finish / 触发重新诊断
        let lines_before = lines.clone();
        let mut did_reexplore = false;
        let mut edit_calls = 0usize;
        let mut go_finish = false;
        loop {
            if edit_calls >= MAX_EDITS_PER_ITER {
                eprintln!("  {}", paint(tty, "33", "医生单轮编辑过多，强制重新诊断"));
                break;
            }
            edit_calls += 1;
            let reply = match editor.next().await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("  {}", paint(tty, "31", &format!("医生决策出错：{}", e)));
                    // 用已有 best 兜底
                    return best;
                }
            };
            let (pt, ct) = editor.last_usage();
            // 医生会话独立计 token，累进报告的 extra（最终并入总量统计）
            report.extra_prompt += pt;
            report.extra_completion += ct;
            let toks = paint(tty, "2", &format!("↑{} ↓{}", fmt_tokens(pt), fmt_tokens(ct)));

            let (text, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 160);
                    eprintln!("  {}  {}", if b.is_empty() { "（未调用工具）".into() } else { b }, toks);
                    editor.user("请只通过调用一个编辑工具（delete_lines / replace_line / insert_after / reexplore）或 run / finish 来操作。");
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
            eprintln!("  {}  {}  {}", paint(tty, "1;35", &format!("⟫ {}", primary.name)), say, toks);
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
                    let removed: Vec<String> = lines.drain((from - 1)..to).collect();
                    eprintln!("  {}  {}", paint(tty, "32", &format!("✓ 删第 {}-{} 行", from, to)), paint(tty, "2", &removed.iter().map(|l| friendly(l)).collect::<Vec<_>>().join(" / ")));
                    editor.tool_result(primary.call_id.as_str(), format!("已删第 {}-{} 行，脚本现 {} 行。改完记得 run 验证。", from, to, lines.len()));
                }
                EditOp::Replace { line, content } => {
                    let n = lines.len();
                    if line < 1 || line > n {
                        editor.tool_result(primary.call_id.as_str(), format!("行号越界：脚本共 {} 行，无法替换第 {} 行", n, line));
                        continue;
                    }
                    if let Err(why) = validate_line(&content, ctx.element_path) {
                        editor.tool_result(primary.call_id.as_str(), format!("替换被拒：{}", why));
                        continue;
                    }
                    let old = std::mem::replace(&mut lines[line - 1], content.trim().to_string());
                    eprintln!("  {}  {} → {}", paint(tty, "32", &format!("✓ 改第 {} 行", line)), paint(tty, "2", &friendly(&old)), paint(tty, "2", &friendly(&lines[line - 1])));
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
                    eprintln!("  {}  {}", paint(tty, "32", &format!("✓ 在第 {} 行后插入", after)), paint(tty, "2", &friendly(&lines[after])));
                    editor.tool_result(primary.call_id.as_str(), format!("已插入，脚本现 {} 行。改完记得 run 验证。", lines.len()));
                }
                EditOp::Reexplore { from_line, reason } => {
                    if report.repairs >= super::verify::MAX_REPAIRS {
                        editor.tool_result(primary.call_id.as_str(), format!("已达活体重探上限（{} 次），请改用文本编辑或 finish。", super::verify::MAX_REPAIRS));
                        continue;
                    }
                    match reexplore_segment(explore_sess, tx, ctx, params, script_path, case, &lines, from_line, &reason, report).await {
                        Some(new) => {
                            lines = new;
                            did_reexplore = true;
                            tx.log("doctor_edit_applied", serde_json::json!({ "iter": iter, "tool": "reexplore", "script_after": lines.clone() }));
                            editor.tool_result(primary.call_id.as_str(), format!("已活体重探并拼接，脚本现 {} 行。请 run 验证是否到达目标。", lines.len()));
                        }
                        None => {
                            editor.tool_result(primary.call_id.as_str(), "活体重探失败/未走通（或用户中断）。换个 from_line、或改用文本编辑、或 finish。");
                        }
                    }
                    break; // 重探后立刻重新诊断
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

        // 收尾：再诊断一次确认是否真达标
        if go_finish {
            eprintln!();
            eprintln!("  {}", paint(tty, "1;36", "▶ 收尾兜底诊断（重启净化中…）"));
            let final_diag = diagnose(tx, ctx, params, script_path, case, &lines, marker, "doctor_diagnose", iter, true).await;
            if final_diag.reached {
                let shorter = best.as_ref().map(|b| lines.len() < b.len()).unwrap_or(true);
                if shorter {
                    best = Some(lines.clone());
                }
                report.hit_iter_limit = false; // 正常收尾、未触上限
                return best;
            }
            // finish 但其实没到 → 若有 best 就返回 best，否则继续修
            if best.is_some() {
                report.hit_iter_limit = false;
                return best;
            }
            editor.user("你 finish 了，但重新诊断**仍未到达目标**。请继续修（看下面的最新 trace）。");
            // 落到下一轮
        }

        // 停滞检测：本轮没改动、没重探、且没达标 → 计一次停滞；连续 2 次就收手
        let unchanged = lines == lines_before;
        if unchanged && !did_reexplore && !diag.reached {
            stagnation += 1;
            if stagnation >= 2 {
                eprintln!("  {}", paint(tty, "33", "医生连续两轮无有效改动且未达标，停止"));
                report.hit_iter_limit = false; // 停滞收手（非轮数上限）
                return best;
            }
        } else {
            stagnation = 0;
        }
    }

    eprintln!("  {}", paint(tty, "33", &format!("已达医生诊断轮数上限（{} 轮，已保留当前最好版本）", MAX_DOCTOR_ITERS)));
    report.hit_iter_limit = true; // 达到优化上限：若 best 存在则"仍可跑、未必最短"
    best
}
