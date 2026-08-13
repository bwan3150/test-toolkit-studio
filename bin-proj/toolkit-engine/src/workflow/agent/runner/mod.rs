// 【编排】AgentRunner：装配 + 开一场主 AI(orchestrator) 会话。
// 本文件只做"装配 + 收尾渲染辅助"；主 AI 会话循环在 orchestrator.rs，驱动循环在 flow.rs。

pub mod advisor;
pub mod asserter;
pub mod ctx;
pub mod diagnose;
#[cfg(test)]
mod drive_tests;
#[cfg(test)]
mod repair_tests;
#[cfg(test)]
mod tklib_tests;
pub mod flow;
pub mod fmt;
pub mod healer;
pub mod interrupt;
pub mod oneshot;
pub mod options;
pub mod orchestrator;
pub mod reflect;
pub mod supervisor;
pub mod testrun;
pub mod tksops;
pub mod verify;

pub use options::{AgentResult, AgentRunOptions};

// explore 驱动+收尾在 runner::testrun；replay/repair/optimize 在 runner::tksops；主 AI 调度在
// orchestrator。本文件只剩 run() 装配 + 收尾渲染辅助（render_summary / slug / unique_script_path 等）。
use std::path::{Path, PathBuf};

use crate::Result;

use super::knowledge::KnowledgeOutcome;
use super::transcript::Transcript;
use super::ui::{Frontend, Level, StatusLine, Tokens, UiEvent};

/// AI 探索测试编排器
pub struct AgentRunner;

impl AgentRunner {
    /// 入口：安装统一中断 + 开一场编排官会话。
    /// 编排官(orchestrator)与用户对话、把 explore/verify/finalize 当独立工具分别调度（见 runner::testrun）。
    pub async fn run(opts: AgentRunOptions, ui: &dyn Frontend) -> Result<AgentResult> {
        // 统一中断：安装进程级 Ctrl+C 监听，探索/诊断/验证/医生各阶段共用同一中断标志
        interrupt::install();
        // 编排官接管整场会话：以 opts.case 为开场用例（脚本落工作区、中间文件落 cache，无需预建 script_dir）
        orchestrator::serve(&opts, ui).await
    }
}

/// 末尾统一渲染结果框：标题即状态（探索完成/未达成/已终止 + 步数轮数），框前由前端
/// 展示染色脚本；框内=依据 + 两件套路径 + 用量。回放/稳定只在真跑过时显示。
#[allow(clippy::too_many_arguments)]
fn render_summary(
    success: bool,
    aborted: bool,
    reason: &str,
    rounds: usize,
    verified: Option<&options::VerifyReport>,
    model: &str,
    tp: i64,
    tc: i64,
    script_path: &Path,
    tklib_path: &Path,
    steps: &[String],
    ui: &dyn Frontend,
) {
    let n = steps.len();
    let status = if aborted {
        StatusLine::new(Level::Warn, format!("已终止 · {} 步（{} 轮）", n, rounds))
    } else if success {
        StatusLine::new(Level::Ok, format!("探索完成 · {} 步（{} 轮）", n, rounds))
    } else {
        StatusLine::new(Level::Err, format!("探索未达成 · {} 步（{} 轮）", n, rounds))
    };
    // 回放/稳定：只在真跑过时给行（None 不渲染）
    let diagnose = verified.map(|r| {
        if !r.reached {
            StatusLine::new(Level::Err, "✗ 修复失败（脚本仍跑不到目标）".to_string())
        } else if r.hit_iter_limit {
            StatusLine::new(Level::Warn, format!("■ 优化达上限·仍可跑（修复 {} 次 · 最终 {} 步）", r.repairs, r.final_steps))
        } else {
            StatusLine::new(Level::Ok, format!("✓ 回放可达目标（修复 {} 次 · 最终 {} 步）", r.repairs, r.final_steps))
        }
    });
    let verify = verified.map(|r| {
        if r.passed {
            StatusLine::new(Level::Ok, format!("✓ 稳定（连续 {} 次到达目标）", r.stability_passes))
        } else if !r.reached {
            StatusLine::new(Level::Dim, "未进入稳定性测试（诊断未修复）".to_string())
        } else {
            StatusLine::new(Level::Err, format!("✗ 不稳定（连续到达仅 {} 次）", r.stability_passes))
        }
    });
    let script = Some(crate::workflow::agent::ui::ScriptInfo {
        tks: script_path.to_string_lossy().to_string(),
        tklib: tklib_path.to_string_lossy().to_string(),
        steps: steps.to_vec(),
    });

    ui.emit(UiEvent::Summary {
        status,
        diagnose,
        verify,
        reason: reason.to_string(),
        script,
        model: model.to_string(),
        tokens: Tokens::new(tp, tc),
    });
}

/// 记录一次知识检索结果
fn record_knowledge(tx: &mut Transcript, kind: &str, outcome: KnowledgeOutcome) {
    match outcome {
        KnowledgeOutcome::Hit(ctx) => tx.log(kind, serde_json::json!({ "hit": true, "context": ctx })),
        KnowledgeOutcome::Skipped(why) => tx.log(kind, serde_json::json!({ "hit": false, "skipped": why })),
    }
}

/// 提取脚本里实际引用到的元素名集合（解析 .tks，收集 `TksParam::Element` 的 name）。
/// 用于判定本次新建元素里哪些是孤儿（最终脚本没用到）。
fn referenced_element_names(lines: &[String]) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    let content = format!("步骤:\n{}", lines.join("\n"));
    if let Ok(script) = crate::ScriptParser::new().parse(&content) {
        for step in &script.steps {
            for p in &step.params {
                if let crate::TksParam::Element { name, .. } = p {
                    set.insert(name.clone());
                }
            }
        }
    }
    set
}

/// 把任意文字压成适合做文件名的 slug：保留字母数字（含中日韩），其余转连字符，小写、去重、限长。
fn slug(s: &str, max: usize) -> String {
    let s = s.trim().lines().next().unwrap_or("").trim();
    let s = s.strip_suffix(".tks").unwrap_or(s);
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if out.chars().count() >= max {
            break;
        }
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "harness".to_string()
    } else {
        out
    }
}

/// 在目录内生成不冲突的 .tks 路径：base.tks 被占则 base-2.tks、base-3.tks…，绝不覆盖旧脚本。
fn unique_script_path(dir: &Path, base: &str) -> PathBuf {
    let first = dir.join(format!("{}.tks", base));
    if !first.exists() {
        return first;
    }
    let mut n = 2;
    loop {
        let p = dir.join(format!("{}-{}.tks", base, n));
        if !p.exists() {
            return p;
        }
        n += 1;
    }
}
