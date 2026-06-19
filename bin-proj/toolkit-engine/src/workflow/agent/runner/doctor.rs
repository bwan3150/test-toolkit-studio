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
use crate::{AiConfig, LlmReply, LlmSession, LlmTool, LlmToolCall, Params, RunEvent, ScriptParser, ScriptRunner, TksParam};

use super::super::execution::script::write_script;
use super::super::perception::{capture, render_element_list};
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
    /// 该步执行后页面的精简摘要(前几个元素文本)，给医生看「这步去了哪」
    page_brief: String,
    /// 该步执行后页面的完整元素列表(仅失败步及其前后会塞进提示词，省 token)
    page_full: String,
}

/// 一次诊断结果
struct Diagnosis {
    /// 是否真到达目标(脚本全跑通 且 目标标志出现)
    reached: bool,
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

/// 医生工具集(独立于探索工具)。description 内联——这是内部 agent，暂不走 PromptSet 覆盖。
fn editor_tools() -> Vec<LlmTool> {
    vec![
        LlmTool::new(
            "delete_lines",
            "删除脚本的第 from..=to 行(1-based，含两端)。用于删掉冗余/重复/走错绕回/点了没用的空步。删错了若导致目标丢失会被系统自动还原，可大胆删。",
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
        LlmTool::new(
            "replace_line",
            "把第 line 行替换成新的 .tks 行 content。**只能引用已存在于元素库里的元素**(写成 [{元素名}])，或纯参数改动(如修正输入文本、把滑动幅度 full 改 quarter、改坐标)。需要点一个全新元素/走全新路径请改用 reexplore。",
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
        LlmTool::new(
            "insert_after",
            "在第 after 行之后插入一行 .tks(after=0 表示插到最前)。同 replace_line：只能引用已有元素或纯无元素步(如 `等待 1000`、`定向滑动 [{640,406}, 上, half]`、`隐藏键盘`)。新导航请用 reexplore。",
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
        LlmTool::new(
            "reexplore",
            "活体逃生口：当某段没法靠文本编辑改对(需要点全新元素、走一条全新路径)时，从第 from_line 步起**现场重新导航**——系统会回放前 from_line-1 步把设备定位到位，再让探索引擎实地重探剩余目标、拼回脚本。这是引入新元素/新路径的唯一正确方式。",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "from_line": { "type": "integer", "description": "从第几步起重新导航(1-based)。应选**真正开始走错**的那一步，通常比报错步更早。" },
                    "reason": { "type": "string", "description": "为什么这段要活体重探(看富 trace 说清楚哪步跑偏了)" }
                },
                "required": ["from_line", "reason"]
            }),
        ),
        LlmTool::new(
            "run",
            "重新诊断回放当前脚本，拿到最新的逐步 trace 和「是否到达目标」。这是你测试自己改动的方式——改完就 run 看效果。",
            serde_json::json!({ "type": "object", "properties": {} }),
        ),
        LlmTool::new(
            "finish",
            "收尾：当脚本已稳定到达目标、且你认为已删到最短、没有更多可改时调用。系统会再诊断一次兜底校验。",
            serde_json::json!({
                "type": "object",
                "properties": { "reason": { "type": "string", "description": "收尾依据" } },
                "required": ["reason"]
            }),
        ),
    ]
}

/// 医生系统提示词(内联)
const EDITOR_SYSTEM: &str = "你是「脚本医生」。手里有一份由自动探索产出的 .tks 回放脚本，它能被 `tke run` 逐行回放，\
但当前**回放时跑不到测试目标**(或有冗余步)。你的任务：像代码 agent 一样**编辑这份脚本的任意行**，\
反复「改→run→看 trace」，直到它能稳定回放到目标，并尽量最短。\n\n\
你会反复收到【诊断 trace】：整条脚本逐步回放的结果——每步的 .tks 行、成功/失败、错误、以及**该步执行后页面变成了什么样**。\
请重点看：报错那一步往往不是真正的根因，常常是**更早某步根本没跳转/点错了**(看页面没变就知道)，后面都在错页面上空跑、到某步才暴露。\n\n\
工具：\n\
- delete_lines：删冗余/走错绕回/无用空步(删错会被自动还原，放心删)。\n\
- replace_line / insert_after：纯文本改/插。**只能引用已存在元素**([{元素名}])或纯参数(输入文本、滑动幅度、坐标、等待)。\n\
- reexplore(from_line)：需要点**全新元素**或走**全新路径**时唯一正确的办法——从某步起活体现场重新导航。选「真正开始跑偏的那一步」。\n\
- run：重新回放诊断，测你的改动。\n\
- finish：脚本已稳定到达目标且最短时收尾。\n\n\
原则：能用最小改动(删一两步 / 改一行参数)解决就别 reexplore；只有确实需要新元素/新路径才 reexplore。每次只调一个工具。";

// ===================== 诊断回放 =====================

/// 诊断回放：整脚本跑一遍(去结尾「关闭」步)，**每步留下页面**，产出富 trace + 是否到达目标。
/// 靠 ScriptRunner 一次带产物的回放就逐步落盘 screenshots/page，回放后离线逐步解析 + OCR 重建。
async fn diagnose(
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    lines: &[String],
    marker: &str,
    verbose: bool,
) -> Diagnosis {
    let tty = std::io::stderr().is_terminal();
    let check = strip_trailing_close(lines);
    if check.is_empty() {
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
            return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: format!("回放出错：{}", e) };
        }
    };
    let run_dir = result.run_dir.clone().map(PathBuf::from).unwrap_or_else(|| log_root.clone());

    // 离线逐步重建页面(结构 + OCR)
    let mut steps: Vec<DiagStep> = Vec::with_capacity(result.steps.len());
    for st in &result.steps {
        let (mut brief_txt, mut full_txt) = (String::new(), String::new());
        if let Some(xml_rel) = &st.xml {
            let xml_abs = run_dir.join(xml_rel);
            if let Ok(mut els) = ctx.fetcher.fetch_elements_from_file(&xml_abs) {
                if let (Some(src), Some(png_rel)) = (ctx.ocr, &st.screenshot) {
                    if let Ok(bytes) = std::fs::read(run_dir.join(png_rel)) {
                        let _ = enrich_with_ocr(&mut els, &bytes, src).await;
                    }
                }
                // 精简摘要：前 6 个元素文本
                brief_txt = els
                    .iter()
                    .take(6)
                    .map(|e| brief(&e.to_ai_text(), 24))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");
                let none = vec![None; els.len()];
                full_txt = render_element_list(&els, &none);
            }
        }
        steps.push(DiagStep {
            no: st.index + 1,
            line: st.command.clone(),
            ok: st.success,
            err: st.error.clone(),
            page_brief: brief_txt,
            page_full: full_txt,
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
    Diagnosis { reached, steps, fail_idx, note }
}

/// 把诊断结果渲染成给医生的提示词：编号脚本 + 逐步 trace(精简) + 失败步前后的完整页面 + 目标。
fn render_trace_prompt(case: &str, marker: &str, diag: &Diagnosis, objective_minimize: bool) -> String {
    let mut s = String::new();
    s.push_str(&format!("整体测试目标：{}\n目标标志(只有真到达目标才会出现的文字)：「{}」\n\n", case, marker));
    s.push_str(&format!("【诊断结论】{}\n\n", diag.note));

    // 逐步 trace(精简)：步号 + 成败 + .tks + 该步后页面前几项
    s.push_str("【逐步 trace】(每步执行后页面的前几个元素，用于看这步实际去到了哪)\n");
    for st in &diag.steps {
        let mark = if st.ok { "✓" } else { "✗" };
        let err = st.err.as_deref().map(|e| format!("  ← 错误：{}", brief(e, 60))).unwrap_or_default();
        let page = if st.page_brief.is_empty() { String::new() } else { format!("\n      页面：{}", st.page_brief) };
        s.push_str(&format!("{}. {} {}{}{}\n", st.no, mark, friendly(&st.line), err, page));
    }

    // 失败步及其前 2 步的完整页面(给足判断根因的细节)
    if let Some(k) = diag.fail_idx {
        let lo = k.saturating_sub(2);
        s.push_str("\n【关键页面详情】(失败步及其前几步的完整元素列表)\n");
        for st in diag.steps.iter().filter(|s| s.no >= lo + 1 && s.no <= k + 1) {
            if !st.page_full.is_empty() {
                s.push_str(&format!("— 第 {} 步「{}」后的页面：\n{}\n", st.no, friendly(&st.line), st.page_full));
            }
        }
    } else if !diag.reached {
        // 全跑通却没到目标：给最后一步的完整页面(看终点错在哪)
        if let Some(st) = diag.steps.last() {
            if !st.page_full.is_empty() {
                s.push_str(&format!("\n【终点页面】脚本跑完停在这个页面，但目标标志没出现：\n{}\n", st.page_full));
            }
        }
    }

    if objective_minimize {
        s.push_str(
            "\n当前脚本**已能到达目标**。现在请尽量**删冗余步**让它最短(重复点击、走错绕回、点了没用的空步)，\
             每删完用 run 确认仍到达目标；删错导致目标丢失会被自动还原。确认没有更多可删时 finish。",
        );
    } else {
        s.push_str(
            "\n请判断**真正开始跑偏的那一步**(常比报错步更早)，用最小改动修好它：能删/改参数就别 reexplore，\
             需要新元素/新路径才 reexplore。改完用 run 看效果。",
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
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    script_path: &Path,
    case: &str,
    marker: &str,
    mut lines: Vec<String>,
    report: &mut VerifyReport,
) -> Option<Vec<String>> {
    let tty = std::io::stderr().is_terminal();

    // 独立医生会话(独立工具集)。其 token 单独累计、最后并入总量(报告 extra_*)。
    let mut editor = match LlmSession::new(ai, EDITOR_SYSTEM, editor_tools()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  {}", paint(tty, "31", &format!("脚本医生会话创建失败：{}", e)));
            return None;
        }
    };

    // 最短的达标版本(护栏：删坏了自动还原到它)
    let mut best: Option<Vec<String>> = None;
    let mut stagnation = 0usize; // 连续「没改动且没达标」次数

    for iter in 1..=MAX_DOCTOR_ITERS {
        eprintln!();
        eprintln!("  {}", paint(tty, "1;36", &format!("▶ 诊断回放（第 {} 轮，重启净化中…）", iter)));
        let diag = diagnose(ctx, params, script_path, case, &lines, marker, true).await;

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
        editor.user(render_trace_prompt(case, marker, &diag, objective_minimize));

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
            // 一行思考
            let say = text.as_deref().map(|s| brief(s, 160)).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("调用 {}", primary.name));
            eprintln!("  {}  {}", say, toks);
            tx.log("doctor_edit", serde_json::json!({ "iter": iter, "tool": primary.name.clone(), "args": primary.arguments.clone() }));

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
        }

        // 收尾：再诊断一次确认是否真达标
        if go_finish {
            let final_diag = diagnose(ctx, params, script_path, case, &lines, marker, true).await;
            if final_diag.reached {
                let shorter = best.as_ref().map(|b| lines.len() < b.len()).unwrap_or(true);
                if shorter {
                    best = Some(lines.clone());
                }
                return best;
            }
            // finish 但其实没到 → 若有 best 就返回 best，否则继续修
            if best.is_some() {
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
                return best;
            }
        } else {
            stagnation = 0;
        }
    }

    eprintln!("  {}", paint(tty, "33", &format!("已达医生诊断轮数上限（{} 轮）", MAX_DOCTOR_ITERS)));
    best
}
