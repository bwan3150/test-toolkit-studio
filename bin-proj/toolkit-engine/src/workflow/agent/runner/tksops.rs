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
/// 起始页**描述**("这个页面是什么"一句话)的头注释前缀——回放对齐起始态时,
/// 人和编排官都靠它知道该导航到哪(标志本身可能只是一个词,如"KonecHome")。
const START_DESC_PREFIX: &str = "# 起始页: ";

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

/// 读起始页描述。
pub(crate) fn read_start_desc(path: &Path) -> Option<String> {
    read_header_value(path, START_DESC_PREFIX)
}

/// 写起始页描述。
pub(crate) fn write_start_desc(path: &Path, desc: &str) -> Result<()> {
    write_header_value(path, START_DESC_PREFIX, desc)
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
    let start_desc = read_start_desc(tks).unwrap_or_default();
    warn_blind_start(ui, &env.lines, &start);
    // 定位自愈默认开：元素定位失败时基于当前实时页面单次挑选修正（详见 runner::healer）
    let healer = std::sync::Arc::new(super::healer::LlmElementHealer::new(
        opts.ai.clone(),
        env.prompts.clone(),
        env.device.clone(),
        env.elem_lib.clone(),
    ));
    let diag = super::diagnose::diagnose(&mut tx, &ctx, &env.params, goal, &env.lines, &marker, &start, &start_desc, Some(healer.clone()), "replay_tks", 0, true).await;
    let healed = healer.healed_names();
    if !healed.is_empty() {
        ui.emit(UiEvent::Notice {
            level: Level::Ok,
            text: format!("🩹 定位自愈 {} 处（{}），元素包已更新", healed.len(), healed.join("、")),
        });
        env.repack(ui); // 自愈修正持久化进 .tklib——以后的回放直接命中
    }
    let n = env.lines.len();
    // 起始不符时附起始页快照摘要(探索时打进 .tklib 的 start_page.txt)——给编排官具体画面
    let start_ref = if !diag.reached && (diag.note.contains("起始页不符") || diag.note.contains("页面断言失败：期望在「起始页」")) {
        crate::tools::element::get_page(&env.elem_lib, "起始页")
            .map(|(desc, sig)| {
                let head = sig.iter().take(6).cloned().collect::<Vec<_>>().join("、");
                format!(
                    "\n- 起始页参考（探索时的样子）：{}{}",
                    if desc.is_empty() { String::new() } else { format!("{}；元素：", desc) },
                    super::fmt::brief(&head, 160)
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let msg = if diag.reached {
        format!("回放通过：脚本能跑通并到达目标（{} 步）。", n)
    } else {
        // 完整测量结果：逐步轨迹（步骤→页面链，判断哪步走偏必需）+ 失败点页面详情 + 建议——
        // 决策（续探/先导航/问用户）在编排官；本报告经 bulky 提交，后续轮次自动滚动省略
        let keep = diag.fail_idx.unwrap_or(n);
        let where_ = diag.fail_idx.map(|i| format!("第 {} 步", i + 1)).unwrap_or_else(|| "目标判定".into());
        format!(
            "回放未到达目标（{}）：{}{}\n\n{}\n- 可选下一步：resume_explore{{keep_steps: {}}} 从当前页面续探修复；若轨迹显示**起始态就没对齐**（第 1 步落的页面就不对/登录态不对），先 navigate 导航对齐再重放；拿不准就问用户。设备现停在失败现场。",
            where_, diag.note, start_ref, diag.trace_report(), keep
        )
    };
    Ok(msg)
}

/// 断点续探（编排官的修复**原语**，取代旧的一键黑盒 repair）：设备**此刻停在失败/中断现场**
/// （通常刚 replay_tks 失败），保留脚本前 keep_steps 步，explorer 从当前实时页面把 goal 走完，
/// 脚本 = 成功前缀 + 新尾巴，写回 .tks + 回包 .tklib。失败**不写回**。
/// 本工具**不做诊断回放、不做验证**——什么时候续探、保留几步、要不要先导航对齐起始态、
/// 修完何时 replay 确认，全部由编排官结合对话上下文决策（用户的指导经 note 直达 explorer）。
pub(crate) async fn resume_explore(
    opts: &AgentRunOptions,
    ui: &dyn Frontend,
    tks: &Path,
    goal: &str,
    keep_steps: usize,
    note: &str,
) -> Result<String> {
    let (env, mut tx) = setup(opts, tks, goal).await?;
    let keep = keep_steps.min(env.lines.len());
    ui.emit(UiEvent::Phase { phase: Phase::Reexplore, n: None });
    ui.emit(UiEvent::Notice {
        level: Level::Warn,
        text: format!("◆ 断点续探：保留前 {} 步，explorer 从设备当前页面继续走目标", keep),
    });
    let platform = Platform::from_device(Some(&env.device));
    let mut sess = crate::LlmSession::new_for_role(
        &opts.ai,
        "explorer",
        env.prompts.role_system("explorer", &env.device, platform.name()),
        super::super::tools::build_tools(&env.prompts, platform),
    )?;
    let failure = if note.trim().is_empty() {
        "回放在保留步之后失败/未到达目标（具体原因见上一次回放报告）。".to_string()
    } else {
        note.trim().to_string()
    };
    let resume_msg = render(
        &env.prompts.message("explorer", "repair_resume"),
        &[("case", goal), ("n", &keep.to_string()), ("failure", &failure)],
    );
    tx.log("llm_message", serde_json::json!({ "content": resume_msg.clone() }));
    sess.user(resume_msg);
    let ctx = op_ctx!(env, opts, ui);
    let outcome = super::flow::drive(&mut sess, &mut tx, &ctx, false, "").await?;
    if outcome.aborted {
        return Ok("已中断（用户），脚本未改动。".into());
    }
    if !outcome.success {
        return Ok(format!("续探未能走到目标（{}）。脚本保持原样未改动。", outcome.reason));
    }
    let mut lines: Vec<String> = env.lines[..keep].to_vec();
    let added = outcome.lines.len();
    lines.extend(outcome.lines);
    write_tks_lines(tks, &lines)?;
    env.repack(ui); // 续探落的新元素随包写回，保持两件套自包含
    Ok(format!(
        "已续写：保留前 {} 步 + 新增 {} 步（共 {} 步），元素包已更新。**尚未验证**——请 replay_tks 确认能稳定到达目标。",
        keep, added, lines.len()
    ))
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

// ============ 【AI 辅助驾驶 · 起始态对齐】tke run 开跑前的起点校准 ============

/// 起始态对齐结果（见 align_start）
pub enum AlignOutcome {
    /// 无需/无法对齐（有启动步/无起始参照/缺前提），照旧开跑——对齐是增强，绝不拦住本可运行的脚本
    Skipped(&'static str),
    /// 当前已在起始页（零 AI 成本，本地匹配命中）
    AlreadyThere,
    /// AI 导航后到达起始页
    Aligned,
    /// AI 导航后仍不在起始页——**不该开跑**（在错误页面上回放可能产生副作用），带诊断报告
    Failed(String),
}

/// 【起始态对齐】把设备带回脚本的起始页，防止"从当前页面闭眼开跑"：
/// - 脚本有「启动」步 → 跳过（冷启动自会对齐，剩余偏差由步内自愈/分诊兜底）
/// - 起始参照 = .tklib pages「起始页」（desc+特征集）；老脚本退回「# 起始标志:」头注释
/// - 当前页**本地匹配**（page_match_score，零 AI 成本）命中 → 直接跑
/// - 不匹配 → navigate 轻导航（纪律：最短路径 + 禁改账号/数据状态；**绝不代替登录**）→ 实测复验
/// - 复验仍不匹配 → Failed：登录态/权限/特定数据类前提查得出、说得清，但只能人工处理——
///   自动登录既要凭据又改变账号状态，是纪律红线（同不幂等操作的"起始警告+与用户商量"出口）
pub async fn align_start(params: &Arc<crate::Params>, ui: &dyn Frontend, tks: &Path) -> AlignOutcome {
    // —— 前提收集：任何一环缺失都 Skipped，让 ScriptRunner 照旧跑/报它自己的错 ——
    let Ok(lines) = read_tks_lines(tks) else { return AlignOutcome::Skipped("脚本不可读") };
    if lines.first().map(|l| super::fmt::is_launch_line(l)).unwrap_or(false) {
        return AlignOutcome::Skipped("脚本有启动步，冷启动自会对齐");
    }
    let Some(device) = params.device() else { return AlignOutcome::Skipped("未指定设备") };
    let tklib_file = tklib::tklib_path(tks);
    if !tklib_file.is_file() {
        return AlignOutcome::Skipped("缺元素包"); // run 会报缺包错，这里不抢话
    }
    // 解一次包读起始参照（ScriptRunner 稍后自会再解包运行，互不干扰）
    let stem = tks.file_stem().and_then(|s| s.to_str()).unwrap_or("script");
    let dest = params
        .cache_root()
        .join("tklib-unpack")
        .join(format!("align-{}-{}", stem, std::process::id()));
    let Ok(lib_json) = tklib::unpack(&tklib_file, &dest) else {
        return AlignOutcome::Skipped("元素包不可读");
    };
    let (desc, sig) = match crate::tools::element::get_page(&lib_json, "起始页") {
        Some((d, s)) if !s.is_empty() => (d, s),
        _ => {
            // 老脚本兜底：头注释起始标志（单词）
            let marker = read_start_marker(tks).unwrap_or_default();
            if marker.is_empty() {
                ui.emit(UiEvent::Notice {
                    level: Level::Warn,
                    text: "脚本无「启动」步且无起始页参照——将从设备当前页面直接开始，请确认已处于起始状态（登录态/所在页）".into(),
                });
                return AlignOutcome::Skipped("无起始参照");
            }
            (read_start_desc(tks).unwrap_or_default(), vec![marker])
        }
    };

    // —— 当前页本地匹配（零 AI 成本）——
    let Ok(workarea) = Workarea::for_device(Some(&device)) else {
        return AlignOutcome::Skipped("工作区不可用");
    };
    let Ok(mut controller) = crate::Controller::new(Some(device.clone())) else {
        return AlignOutcome::Skipped("设备不可用");
    };
    match on_start_page(&mut controller, &workarea, &sig).await {
        Some(true) => return AlignOutcome::AlreadyThere,
        Some(false) => {}
        None => return AlignOutcome::Skipped("当前页面采集失败"),
    }

    // —— AI 导航对齐 ——
    let shown = if desc.is_empty() { String::new() } else { format!("（{}）", desc) };
    ui.emit(UiEvent::Notice {
        level: Level::Warn,
        text: format!("当前不在脚本起始页{}——AI 辅助驾驶导航对齐中…", shown),
    });
    let goal = format!(
        "回到该测试脚本的起始页面：{}。页面特征参考：{}",
        if desc.is_empty() { "见特征列表" } else { &desc },
        sig.iter().take(6).cloned().collect::<Vec<_>>().join("、")
    );
    // 导航预算收紧：对齐是回原地，不是探索（max_rounds 封顶 12）
    let mut ai = params.ai.clone();
    ai.max_rounds = Some(ai.max_rounds.map(|r| r.min(12)).unwrap_or(12));
    let opts = AgentRunOptions {
        case: goal.clone(),
        script_dir: params.workspace_root(),
        ai,
        prompt: super::super::prompt::PromptSpec {
            prompts_dir: params.ai.prompts_dir.clone().map(PathBuf::from),
            ..Default::default()
        },
        ocr: None,
        verify: false,
        platform: None,
        device: Some(device.clone()),
        params: params.clone(),
    };
    let report = match super::testrun::navigate(&opts, ui, &goal, None).await {
        Ok(r) => r,
        Err(e) => format!("导航执行失败：{}", e),
    };

    // —— 实测复验（不信导航自述，以当前页面匹配为准）——
    match on_start_page(&mut controller, &workarea, &sig).await {
        Some(true) => AlignOutcome::Aligned,
        _ => AlignOutcome::Failed(format!(
            "AI 导航后仍未到达起始页{}。{}\n若起始前提是登录态/权限/特定数据，辅助驾驶不会代替登录或改动账号状态——请人工把设备调到起始状态后重跑，或用 tke harness 重探更新脚本。",
            shown, report
        )),
    }
}

/// 采集当前页并与起始页特征集做命中率匹配（复用「断言页面」同一套 page_match_score/阈值）。
/// None = 采集失败（设备不可用等）。
async fn on_start_page(
    controller: &mut crate::Controller,
    workarea: &Workarea,
    sig: &[String],
) -> Option<bool> {
    controller.capture_ui_state(workarea).await.ok()?;
    let elements = Fetcher::new().fetch_elements_from_file(&workarea.ui_tree_path()).ok()?;
    let texts: Vec<String> = elements
        .iter()
        .filter_map(|e| e.text.clone().or_else(|| e.content_desc.clone()))
        .filter(|t| !t.trim().is_empty())
        .collect();
    let (hit, total) = crate::tools::element::page_match_score(sig, &texts);
    Some(total > 0 && hit as f32 / total as f32 >= crate::tools::element::PAGE_MATCH_THRESHOLD)
}
