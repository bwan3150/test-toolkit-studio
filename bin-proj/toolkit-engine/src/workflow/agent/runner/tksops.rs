// 【tksops】对工作区里已有的 .tks 脚本做**独立**操作：回放(replay)/修复(repair)/优化(optimize)。
// 拆自原来 verify/finalize 的捆绑流水线——主 AI 按需各自调用（去常驻 TestRun）。
// 每个脚本的元素库是它的 `.tklib` 元素包（`foo.tks ↔ foo.tklib`，zip 容器，见 utils::tklib）：
// setup 时**解包**到 cache 临时目录、element_path 指过去（下游零改动）；repair 落了新元素则
// 结束时**回包**写回。没有共享元素库——脚本的定位宇宙就是它自己的 tklib。
// 内部复用现成的路径化函数：doctor::diagnose / doctor::doctor_repair / reflect::optimize，
// 目标标志(marker)经 verify::derive_marker 用一次性会话从 goal+步骤推出（脱离探索会话）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::utils::tklib::{self, TklibMeta};
use crate::{Fetcher, Platform, Result, RunArtifacts, TkeError, Workarea};

use super::super::prompt::PromptSet;
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

/// 从 .tks 头注释读已持久化的目标标志。
pub(crate) fn read_marker(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix(MARKER_PREFIX))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 把目标标志写进 .tks 头注释（已有则更新，没有则插在 `步骤:` 之前）。
pub(crate) fn write_marker(path: &Path, marker: &str) -> Result<()> {
    let content = std::fs::read_to_string(path).map_err(TkeError::IoError)?;
    let line = format!("{}{}", MARKER_PREFIX, marker);
    let mut out: Vec<String> = Vec::new();
    let mut placed = false;
    for l in content.lines() {
        if l.trim().starts_with(MARKER_PREFIX) {
            if !placed {
                out.push(line.clone());
                placed = true;
            }
            continue; // 旧 marker 行丢弃（含重复行）
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
    ui.emit(UiEvent::Notice { level: Level::Info, text: format!("▶ 回放开始（{} 步）", env.lines.len()) });
    let ctx = op_ctx!(env, opts, ui);
    // verbose=true：回放逐步可见——此前静默跑完全程，用户只看到"卡了很久"
    let diag = super::doctor::diagnose(&mut tx, &ctx, &env.params, goal, &env.lines, &marker, "replay_tks", 0, true).await;
    let n = env.lines.len();
    let msg = if diag.reached {
        format!("回放通过：脚本能跑通并到达目标（{} 步）。", n)
    } else {
        let where_ = diag.fail_idx.map(|i| format!("第 {} 步", i + 1)).unwrap_or_else(|| "目标判定".into());
        format!("回放未到达目标（{}）：{}", where_, diag.note)
    };
    Ok(msg)
}

/// 回放并修复脚本（医生：修到能跑通+到达目标）。修好后写回 .tks。
pub(crate) async fn repair_tks(opts: &AgentRunOptions, ui: &dyn Frontend, tks: &Path, goal: &str) -> Result<String> {
    let (env, mut tx) = setup(opts, tks, goal).await?;
    if env.lines.is_empty() {
        return Ok("脚本为空，无可修复。".into());
    }
    ui.emit(UiEvent::Phase { phase: Phase::Diagnose, n: None });
    let marker = ensure_marker(opts, ui, &mut tx, &env.prompts, tks, goal, &env.lines).await;
    let mut report = VerifyReport { ran: true, ..Default::default() };
    let ctx = op_ctx!(env, opts, ui);
    let fixed =
        super::doctor::doctor_repair(&opts.ai, &env.prompts, &mut tx, &ctx, &env.params, goal, &marker, env.lines.clone(), &mut report).await;
    match fixed {
        Some(lines) => {
            write_tks_lines(tks, &lines)?;
            // 医生可能落了新元素（pick/pick_visual）——回包写回 .tklib，保持两件套自包含
            env.repack(ui);
            Ok(format!("已修复：脚本现在能跑通并到达目标（修复 {} 次 · {} 步）。", report.repairs, lines.len()))
        }
        None => Ok("修复失败：未能把脚本修到可跑通并到达目标。".into()),
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
    let opt = super::reflect::optimize(&opts.ai, &env.prompts, &mut tx, &ctx, &env.params, goal, &marker, &env.lines, &mut report).await;
    match opt {
        Some(lines) if lines != env.lines => {
            let n0 = env.lines.len();
            write_tks_lines(tks, &lines)?;
            Ok(format!("已优化：删冗余后 {} 步（原 {} 步）。建议再 replay_tks 确认仍到达目标。", lines.len(), n0),)
        }
        _ => Ok("无可优化（没有可删的绕路/冗余步）。".into()),
    }
}
