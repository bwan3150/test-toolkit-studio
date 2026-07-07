// 【tksops】对工作区里已有的 .tks 脚本做**独立**操作：回放(replay)/修复(repair)/优化(optimize)。
// 拆自原来 verify/finalize 的捆绑流水线——主 AI 按需各自调用（去常驻 TestRun）。
// 每个脚本的元素库是它的 `.tklib` 元素包（`foo.tks ↔ foo.tklib`，zip 容器，见 utils::tklib）：
// setup 时**解包**到 cache 临时目录、element_path 指过去（下游零改动）；repair 落了新元素则
// 结束时**回包**写回。没有共享元素库——脚本的定位宇宙就是它自己的 tklib。
// 内部复用现成的路径化函数：diagnose::diagnose / flow::drive(断点续探) / reflect::optimize，
// 目标标志(marker)经 verify::derive_marker 用一次性会话从 goal+步骤推出（脱离探索会话）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::utils::tklib::{self, TklibMeta};
use crate::{Fetcher, Platform, Result, RunArtifacts, TkeError, Workarea};

use super::super::prompt::{render, PromptSet};
use super::super::transcript::Transcript;
use super::super::ui::{Frontend, Level, Phase, UiEvent};
use super::ctx::DriveCtx;
use super::options::{AgentRunOptions, VerifyReport};
use super::slug;

/// 从 .tks 读出步骤源行（跳过 `步骤:` 头、空行、`#` 注释）。
fn read_tks_lines(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path).map_err(TkeError::IoError)?;
    Ok(content
        .lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && t != "步骤:" && !t.starts_with('#')
        })
        .collect())
}

/// 把步骤行写回 .tks（保持 `步骤:` 头）。**保留注释**：头部注释（`# 用例:`/`# 目标标志:` 等）
/// 与步骤后的尾注（`# 注：…`）原样带回——此前整体丢弃，第一次 repair 就会抹掉用例头和留痕。
fn write_tks_lines(path: &Path, lines: &[String]) -> Result<()> {
    let old = std::fs::read_to_string(path).unwrap_or_default();
    let mut header: Vec<&str> = Vec::new();
    let mut trailer: Vec<&str> = Vec::new();
    let mut in_steps = false;
    for l in old.lines() {
        let t = l.trim();
        if !in_steps {
            if t == "步骤:" {
                in_steps = true;
            } else if t.starts_with('#') {
                header.push(l);
            }
        } else if t.starts_with('#') {
            trailer.push(l);
        }
    }
    let mut out = String::new();
    for h in &header {
        out.push_str(h);
        out.push('\n');
    }
    out.push_str("步骤:\n");
    for l in lines {
        out.push_str(l);
        out.push('\n');
    }
    for tl in &trailer {
        out.push_str(tl);
        out.push('\n');
    }
    std::fs::write(path, out).map_err(TkeError::IoError)
}

/// 目标标志(marker)在 .tks 头注释里的持久化前缀。
const MARKER_PREFIX: &str = "# 目标标志: ";
/// 起始标志（起始前提契约）的头注释前缀：无「启动」步的脚本隐含假设起点（已登录/某页面），
/// 把它显式化——回放前校验当前页匹配，不匹配**快速失败并说清**，而不是闭着眼开跑越跑越乱。
const START_PREFIX: &str = "# 起始标志: ";

fn read_header_value(path: &Path, prefix: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix(prefix))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_header_value(path: &Path, prefix: &str, value: &str) -> Result<()> {
    let content = std::fs::read_to_string(path).map_err(TkeError::IoError)?;
    let line = format!("{}{}", prefix, value);
    let mut out: Vec<String> = Vec::new();
    let mut placed = false;
    for l in content.lines() {
        if l.trim().starts_with(prefix) {
            if !placed {
                out.push(line.clone());
                placed = true;
            }
            continue; // 旧行丢弃（含重复行）
        }
        if !placed && l.trim() == "步骤:" {
            out.push(line.clone());
            placed = true;
        }
        out.push(l.to_string());
    }
    if !placed {
        out.insert(0, line);
    }
    std::fs::write(path, out.join("\n") + "\n").map_err(TkeError::IoError)
}

/// 从 .tks 头注释读已持久化的目标标志。
pub(crate) fn read_marker(path: &Path) -> Option<String> {
    read_header_value(path, MARKER_PREFIX)
}

/// 把目标标志写进 .tks 头注释（已有则更新，没有则插在 `步骤:` 之前）。
pub(crate) fn write_marker(path: &Path, marker: &str) -> Result<()> {
    write_header_value(path, MARKER_PREFIX, marker)
}

/// 读起始标志（起始前提契约）。
pub(crate) fn read_start_marker(path: &Path) -> Option<String> {
    read_header_value(path, START_PREFIX)
}

/// 写起始标志。
pub(crate) fn write_start_marker(path: &Path, marker: &str) -> Result<()> {
    write_header_value(path, START_PREFIX, marker)
}


/// 目标标志一次推导 + 持久化：优先用 .tks 头里已存的 marker（同一脚本 replay/repair/optimize
/// **共用同一判定基线**）；没有才推导一次并写回脚本头。此前三个工具各自独立推一次，同一脚本
/// 连续操作会推出三个可能不同的 marker——验收基线在流水线中途漂移。
/// 推导失败（空 marker）emit Warn：此时 doctor 的目标校验退化为"跑通即到达"，绝不静默。
async fn ensure_marker(
    opts: &AgentRunOptions,
    ui: &dyn Frontend,
    tx: &mut Transcript,
    prompts: &PromptSet,
    tks: &Path,
    goal: &str,
    lines: &[String],
) -> String {
    if let Some(m) = read_marker(tks) {
        return m;
    }
    ui.emit(UiEvent::Notice { level: Level::Dim, text: "▶ 脚本头无目标标志，按目标推导中…（探索产出的脚本会自带；手写/老脚本走此兜底）".to_string() });
    let m = super::verify::derive_marker(&opts.ai, prompts, tx, lines, goal).await;
    if m.trim().is_empty() {
        ui.emit(UiEvent::Notice {
            level: Level::Warn,
            text: "未能推出目标标志——本次只验证脚本能跑通，不校验是否真到达目标".to_string(),
        });
    } else if write_marker(tks, &m).is_ok() {
        ui.emit(UiEvent::Notice {
            level: Level::Dim,
            text: format!("目标标志已写入脚本头（回放/修复/优化共用）：{}", super::fmt::brief(&m, 40)),
        });
    }
    m
}

/// 回放开场括注：有「启动」步=会重启净化；没有=从当前页面直接开始（别谎称"重启净化中"）。
fn replay_prelude(lines: &[String]) -> &'static str {
    if lines.first().map(|l| super::fmt::is_launch_line(l)).unwrap_or(false) {
        "，重启净化中…"
    } else {
        "，无启动步——从当前页面直接开始"
    }
}

/// "闭眼起跑"警告：脚本既无「启动」步（没法重启净化）又无「# 起始标志:」（没法校验起点）——
/// 回放起点完全取决于设备当前停在哪，必须让用户/编排官知道。
fn warn_blind_start(ui: &dyn Frontend, lines: &[String], start_marker: &str) {
    let has_launch = lines.first().map(|l| super::fmt::is_launch_line(l)).unwrap_or(false);
    if !has_launch && start_marker.is_empty() {
        ui.emit(UiEvent::Notice {
            level: Level::Warn,
            text: "脚本无「启动」步且无起始标志——回放将从设备**当前页面**直接开始，请先确认已处于脚本的起始状态（登录态/所在页）".to_string(),
        });
    }
}

/// 一次操作所需的环境（拥有所有权，ctx 借用其字段）。
struct OpEnv {
    device: String,
    elem_lib: PathBuf, // 该脚本 .tklib 解包后的 element.json（无包则空临时库）
    tklib: PathBuf,    // 该脚本的 .tklib 元素包路径（repair 落新元素后回包写回这里）
    case: String,      // = goal
    params: Arc<crate::Params>,
    prompts: PromptSet,
    workarea: Workarea,
    fetcher: Fetcher,
    artifacts: RunArtifacts,
    max_rounds: usize,
    lines: Vec<String>,
}

impl OpEnv {
    /// 回包：把（可能已被 repair 写入新元素的）解包库重新打成 .tklib 写回脚本旁。
    fn repack(&self, ui: &dyn Frontend) {
        let platform = Platform::from_device(Some(&self.device));
        let meta = TklibMeta::new(platform.name(), &self.device);
        if let Err(e) = tklib::pack(&self.elem_lib, &self.tklib, &meta) {
            // 回包失败必须可见：脚本已修好但元素包没更新，拷走会缺新元素
            ui.emit(UiEvent::Notice {
                level: Level::Warn,
                text: format!("元素包回写失败（{}）：脚本已更新，但 {} 未更新", e, self.tklib.display()),
            });
        }
    }
}

/// 从 DriveCtx 各字段构造（借用 env 的字段，互不冲突；tx 单独传）。
macro_rules! op_ctx {
    ($env:expr, $opts:expr, $ui:expr) => {
        DriveCtx {
            device: &$env.device,
            element_path: &$env.elem_lib,
            workarea: &$env.workarea,
            fetcher: &$env.fetcher,
            artifacts: &$env.artifacts,
            ocr: $opts.ocr.as_ref(),
            max_rounds: $env.max_rounds,
            prompts: &$env.prompts,
            case: &$env.case,
            ai: &$opts.ai,
            ui: $ui,
            task_mode: false,
            ask_mode: super::ctx::AskMode::Ask,
        }
    };
}

/// 装配环境 + transcript（中间产物落 cache）。
/// 元素库 = 脚本自持的 `.tklib` 元素包：解包到本次运行目录下使用；没有包就给一个空临时库
/// （仅坐标步可回放；repair 落的新元素会随回包生成/更新 .tklib）。**没有共享库回退**。
async fn setup(opts: &AgentRunOptions, tks: &Path, goal: &str) -> Result<(OpEnv, Transcript)> {
    let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
    let stem = slug(
        tks.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default().as_str(),
        30,
    );
    let artifacts = RunArtifacts::create(&opts.params.cache_root(), &stem)?;
    let tklib_file = tklib::tklib_path(tks);
    let unpack_dir = artifacts.run_dir.join("tklib");
    let elem_lib = if tklib_file.is_file() {
        tklib::unpack(&tklib_file, &unpack_dir)?
    } else {
        std::fs::create_dir_all(&unpack_dir).map_err(TkeError::IoError)?;
        let p = unpack_dir.join("element.json");
        std::fs::write(&p, "{\"elements\":{}}").map_err(TkeError::IoError)?;
        p
    };
    let params = Arc::new(
        opts.params
            .with_element_lib(elem_lib.clone())
            .with_device(Some(device.clone())),
    );
    let prompts = PromptSet::resolve(&opts.prompt)?;
    let workarea = Workarea::for_device(Some(&device))?;
    let fetcher = Fetcher::new();
    let max_rounds = opts.ai.max_rounds.unwrap_or(40) as usize;
    let lines = read_tks_lines(tks)?;
    let tx = Transcript::create(artifacts.run_dir.join("conversation.jsonl"))?;
    Ok((
        OpEnv { device, elem_lib, tklib: tklib_file, case: goal.to_string(), params, prompts, workarea, fetcher, artifacts, max_rounds, lines },
        tx,
    ))
}

/// 回放一遍脚本，报告是否跑通并到达目标、在哪失败。不改脚本。
pub(crate) async fn replay_tks(opts: &AgentRunOptions, ui: &dyn Frontend, tks: &Path, goal: &str) -> Result<String> {
    let (env, mut tx) = setup(opts, tks, goal).await?;
    if env.lines.is_empty() {
        return Ok("脚本为空，没有可回放的步骤。".into());
    }
    ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
    let marker = ensure_marker(opts, ui, &mut tx, &env.prompts, tks, goal, &env.lines).await;
    ui.emit(UiEvent::Notice { level: Level::Info, text: format!("▶ 回放开始（{} 步{}）", env.lines.len(), replay_prelude(&env.lines)) });
    let ctx = op_ctx!(env, opts, ui);
    // verbose=true：回放逐步可见——此前静默跑完全程，用户只看到"卡了很久"
    let start = read_start_marker(tks).unwrap_or_default();
    warn_blind_start(ui, &env.lines, &start);
    // 定位自愈默认开：元素定位失败时基于当前实时页面单次挑选修正（详见 runner::healer）
    let healer = std::sync::Arc::new(super::healer::LlmElementHealer::new(
        opts.ai.clone(),
        env.prompts.clone(),
        env.device.clone(),
        env.elem_lib.clone(),
    ));
    let diag = super::diagnose::diagnose(&mut tx, &ctx, &env.params, goal, &env.lines, &marker, &start, Some(healer.clone()), "replay_tks", 0, true).await;
    let healed = healer.healed_names();
    if !healed.is_empty() {
        ui.emit(UiEvent::Notice {
            level: Level::Ok,
            text: format!("🩹 定位自愈 {} 处（{}），元素包已更新", healed.len(), healed.join("、")),
        });
        env.repack(ui); // 自愈修正持久化进 .tklib——以后的回放直接命中
    }
    let n = env.lines.len();
    let msg = if diag.reached {
        format!("回放通过：脚本能跑通并到达目标（{} 步）。", n)
    } else {
        let where_ = diag.fail_idx.map(|i| format!("第 {} 步", i + 1)).unwrap_or_else(|| "目标判定".into());
        format!("回放未到达目标（{}）：{}", where_, diag.note)
    };
    Ok(msg)
}

/// 回放并修复脚本——**断点续探**（取代已删除的「医生」文本编辑 agent）：
/// ① 诊断回放：整脚本跑一遍，停在失败步/错误终点——设备**此刻就停在真实的失败现场**；
/// ② 失败 → 把控制权交给 explorer：从当前实时页面出发把剩下的目标走完（drive 驱动循环，
///    每轮看真实页面——这是唯一"接地"的修复方式，不再对着过期 trace 做文本手术）；
/// ③ 脚本 = 成功前缀 + 续探出的新尾巴 → 再诊断验证，直到达标或用尽预算。
/// 修好后写回 .tks + 回包 .tklib。失败**不写回**——绝不把还能跑一半的脚本改坏。
pub(crate) async fn repair_tks(opts: &AgentRunOptions, ui: &dyn Frontend, tks: &Path, goal: &str) -> Result<String> {
    let (env, mut tx) = setup(opts, tks, goal).await?;
    if env.lines.is_empty() {
        return Ok("脚本为空，无可修复。".into());
    }
    ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
    let marker = ensure_marker(opts, ui, &mut tx, &env.prompts, tks, goal, &env.lines).await;
    let mut report = VerifyReport { ran: true, ..Default::default() };
    let mut lines = env.lines.clone();
    let start = read_start_marker(tks).unwrap_or_default();
    warn_blind_start(ui, &env.lines, &start);
    let max_resumes = env.params.harness.repairs; // 续探预算（config [harness].repairs）
    let mut created: Vec<String> = Vec::new();

    loop {
        if super::interrupt::aborted() {
            return Ok("已中断（用户 Ctrl+C），修复未完成，脚本未改动。".into());
        }
        // ① 诊断回放（逐步可见，定位自愈默认开）：失败即停，设备停在失败现场
        ui.emit(UiEvent::Notice { level: Level::Info, text: format!("▶ 诊断回放（{} 步{}）", lines.len(), replay_prelude(&lines)) });
        let healer = std::sync::Arc::new(super::healer::LlmElementHealer::new(
            opts.ai.clone(),
            env.prompts.clone(),
            env.device.clone(),
            env.elem_lib.clone(),
        ));
        let ctx = op_ctx!(env, opts, ui);
        let diag = super::diagnose::diagnose(&mut tx, &ctx, &env.params, goal, &lines, &marker, &start, Some(healer.clone()), "repair_diagnose", report.repairs, true).await;
        let healed = healer.healed_names();
        if !healed.is_empty() {
            ui.emit(UiEvent::Notice {
                level: Level::Ok,
                text: format!("🩹 定位自愈 {} 处（{}）", healed.len(), healed.join("、")),
            });
        }
        if diag.reached {
            report.reached = true;
            report.created = created.clone();
            report.final_steps = lines.len();
            write_tks_lines(tks, &lines)?;
            env.repack(ui); // 续探落的新元素随包写回，保持两件套自包含
            return Ok(format!("已修复：脚本能跑通并到达目标（断点续探 {} 次 · {} 步）。", report.repairs, lines.len()));
        }
        if report.repairs >= max_resumes {
            return Ok(format!(
                "修复失败：断点续探 {} 次后仍未到达目标（{}）。脚本保持原样未改动。",
                report.repairs, diag.note
            ));
        }
        report.repairs += 1;

        // ② 断点续探：保留成功前缀，explorer 从**当前实时页面**接管走完目标
        let keep = diag.fail_idx.unwrap_or(lines.len()); // 失败步之前保留；全跑通但终点不符 → 全保留
        let failure = match diag.fail_idx {
            Some(k) => format!(
                "第 {} 步「{}」执行失败：{}",
                k + 1,
                super::fmt::friendly(lines.get(k).map(String::as_str).unwrap_or("")),
                diag.note
            ),
            None => format!("脚本全部跑通，但终点校验未通过——{}。当前页面不是目标终点，请继续走到真正的目标。", diag.note),
        };
        ui.emit(UiEvent::Phase { phase: Phase::Reexplore, n: Some(report.repairs as u32) });
        ui.emit(UiEvent::Notice {
            level: Level::Warn,
            text: format!("◆ 断点续探（第 {} 次）：保留前 {} 步，explorer 从失败现场继续", report.repairs, keep),
        });
        let platform = Platform::from_device(Some(&env.device));
        let mut sess = crate::LlmSession::new_for_role(
            &opts.ai,
            "explorer",
            env.prompts.role_system("explorer", &env.device, platform.name()),
            super::super::tools::build_tools(&env.prompts, platform),
        )?;
        let resume_msg = render(
            &env.prompts.message("explorer", "repair_resume"),
            &[("case", goal), ("n", &keep.to_string()), ("failure", &failure)],
        );
        tx.log("llm_message", serde_json::json!({ "content": resume_msg.clone() }));
        sess.user(resume_msg);
        let ctx = op_ctx!(env, opts, ui);
        let outcome = super::flow::drive(&mut sess, &mut tx, &ctx, false, "").await?;
        let (spt, sct) = sess.total_usage();
        report.extra_prompt += spt + outcome.subagent_pt;
        report.extra_completion += sct + outcome.subagent_ct;
        for c in &outcome.created {
            if !created.contains(c) {
                created.push(c.clone());
            }
        }
        if outcome.aborted {
            return Ok("已中断（用户），修复未完成，脚本未改动。".into());
        }
        if !outcome.success {
            return Ok(format!("修复失败：断点续探未能走到目标（{}）。脚本保持原样未改动。", outcome.reason));
        }
        // ③ 拼接：成功前缀 + 新尾巴 → 回到循环顶再诊断验证
        let mut next: Vec<String> = lines[..keep.min(lines.len())].to_vec();
        next.extend(outcome.lines);
        lines = next;
    }
}

/// 优化脚本（反思官：删绕路/冗余步）。有改动写回 .tks；建议改完再 replay 确认。
pub(crate) async fn optimize_tks(opts: &AgentRunOptions, ui: &dyn Frontend, tks: &Path, goal: &str) -> Result<String> {
    let (env, mut tx) = setup(opts, tks, goal).await?;
    if env.lines.is_empty() {
        return Ok("脚本为空，无可优化。".into());
    }
    ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
    let marker = ensure_marker(opts, ui, &mut tx, &env.prompts, tks, goal, &env.lines).await;
    let mut report = VerifyReport { ran: true, ..Default::default() };
    let ctx = op_ctx!(env, opts, ui);
    let start = read_start_marker(tks).unwrap_or_default();
    let opt = super::reflect::optimize(&opts.ai, &env.prompts, &mut tx, &ctx, &env.params, goal, &marker, &start, &env.lines, &mut report).await;
    match opt {
        Some(lines) if lines != env.lines => {
            let n0 = env.lines.len();
            write_tks_lines(tks, &lines)?;
            Ok(format!("已优化：删冗余后 {} 步（原 {} 步）。建议再 replay_tks 确认仍到达目标。", lines.len(), n0),)
        }
        _ => Ok("无可优化（没有可删的绕路/冗余步）。".into()),
    }
}
