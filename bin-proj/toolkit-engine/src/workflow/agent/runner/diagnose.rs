// 【诊断回放】整脚本跑一遍、每步留下页面(结构+OCR)，产出富 trace(Diagnosis)：
// 是否真到达目标 / 第几步失败 / 每步页面 / 无效步分析(优化官用) / 页面重复分析。
// 这是验证与修复共用的**测量仪器**：修复(断点续探)在 tksops::repair_tks——旧的
// 「医生」文本编辑 agent 已删除(对着看不见的设备做脑内手术,越修越坏)。

use std::path::PathBuf;
use std::sync::Arc;

use crate::engines::ocr::enrich_with_ocr;
use crate::{Params, RunEvent, ScriptRunner, UIElement};

use super::super::execution::script::write_script;
use super::super::perception::{capture, render_element_list};
use super::super::transcript::Transcript;
use super::super::ui::{Level, StepState, UiEvent};
use super::ctx::DriveCtx;
use super::fmt::{brief, friendly};
use super::verify::{page_contains, reset_state, strip_trailing_close};

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
pub(crate) struct Diagnosis {
    /// 是否真到达目标(脚本全跑通 且 目标标志出现)
    pub(crate) reached: bool,
    steps: Vec<DiagStep>,
    /// 第一个失败步的下标(0-based)；None=全跑通
    pub(crate) fail_idx: Option<usize>,
    /// 一句话结论(给修复流程 & CLI)
    pub(crate) note: String,
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
    pub(crate) fn noop_step_nos(&self) -> std::collections::HashSet<usize> {
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
    pub(crate) fn trace_lines(&self) -> String {
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
    pub(crate) fn page_groups(&self) -> String {
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

/// 诊断回放：整脚本跑一遍(去结尾「关闭」步)，**每步留下页面**，产出富 trace + 是否到达目标。
/// 靠 ScriptRunner 一次带产物的回放就逐步落盘 screenshots/page，回放后离线逐步解析 + OCR 重建。
///
/// 同时把**结构化执行轨迹**记进 conversation.json（事件类型 = `phase`）：本轮跑的完整脚本、
/// 逐步成败/错误/截图/页面结构路径/页面元素、是否到达目标——格式与探索阶段对称，供事无巨细复盘。
/// phase 区分阶段：诊断用 "doctor_diagnose"、稳定性测试用 "verify_stability"。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn diagnose(
    tx: &mut Transcript,
    ctx: &DriveCtx<'_>,
    params: &Arc<Params>,
    case: &str,
    lines: &[String],
    marker: &str,
    start_marker: &str,
    healer: Option<std::sync::Arc<dyn crate::workflow::tks::ElementHealer>>,
    phase: &str,
    iter: usize,
    verbose: bool,
) -> Diagnosis {
    let check = strip_trailing_close(lines);
    if check.is_empty() {
        tx.log(phase, serde_json::json!({ "iter": iter, "script": check, "reached": false, "note": "空脚本", "steps": [] }));
        return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note: "空脚本".into() };
    }
    // 起始前提校验：无「启动」步的脚本隐含假设起点（已登录/某页面）——有起始标志时先核对
    // 当前页面，不匹配**快速失败并说清**，而不是闭着眼开跑越跑越乱（有启动步的脚本
    // 由重启净化保证确定性，跳过此检查）。
    let has_launch = check.first().map(|l| super::fmt::is_launch_line(l)).unwrap_or(false);
    if !start_marker.is_empty() && !has_launch {
        let ok = match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
            Ok(p) => page_contains(&p, start_marker),
            Err(_) => false,
        };
        if !ok {
            let note = format!(
                "起始页不符：脚本期望从含「{}」的页面开始（起始前提），当前页面没有它——请先把设备导航到脚本的起始状态",
                brief(start_marker, 40)
            );
            if verbose {
                ctx.ui.emit(UiEvent::Notice { level: Level::Err, text: format!("✗ {}", note) });
            }
            tx.log(phase, serde_json::json!({ "iter": iter, "reached": false, "note": note.clone(), "steps": [] }));
            return Diagnosis { reached: false, steps: Vec::new(), fail_idx: Some(0), note };
        }
    }
    // 回放版本写 cache 临时文件，**绝不写用户的 .tks**——此前直接覆盖 script_path，
    // 会把脚本头的 `# 目标标志:`、尾部关闭步、`# 注：` 全部抹掉（tklib 可移植性测试抓出的 bug）。
    let replay_path = ctx.artifacts.run_dir.join(format!("{}-replay.tks", phase));
    let _ = write_script(&replay_path, case, &check);
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
        let mut runner = ScriptRunner::new(params.clone());
    if let Some(h) = healer {
        runner = runner.with_healer(h);
    }
    let result = runner.run(&replay_path, Some(&log_root), &mut sink).await;
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
    let reached = if result.success {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
            Ok(p) => marker.is_empty() || page_contains(&p, marker),
            Err(_) => marker.is_empty(),
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
