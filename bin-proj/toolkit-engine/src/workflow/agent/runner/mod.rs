// 【编排】AgentRunner：装配各子模块（提示词/会话/感知/执行/记忆/日志）并驱动循环
// 本文件只做"装配 + 收尾"，循环逻辑在 flow.rs。

pub mod doctor;
pub mod flow;
pub mod options;
pub mod verify;

pub use options::{AgentResult, AgentRunOptions};

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::models::Platform;
use crate::{ExecutionResult, Fetcher, LlmSession, Result, RunArtifacts, Workarea};

use super::execution::script::write_script;
use super::knowledge::{Knowledge, KnowledgeOutcome};
use super::prompt::PromptSet;
use super::tools::build_tools;
use super::transcript::Transcript;
use flow::{drive, DriveCtx};

/// AI 探索测试编排器
pub struct AgentRunner;

impl AgentRunner {
    pub async fn run(opts: AgentRunOptions) -> Result<AgentResult> {
        let device = opts.params.device().unwrap_or_default();
        let platform = Platform::from_device(Some(&device));

        // 脚本输出目录确保存在（.tks 文件名由 AI 在 finish 起、落库时去重）
        std::fs::create_dir_all(&opts.script_dir).ok();

        // 元素库（参数层查表；缺省落脚本目录下的 element.json，整目录脚本共享一份库）
        let element_path: PathBuf = opts
            .params
            .element_lib()
            .unwrap_or_else(|| opts.script_dir.join("element.json"));

        // —— 产物目录：复用 RunArtifacts，与 tke run 同构 ——
        // log 根：--log/config 优先；缺省落脚本目录下（harness 始终保留探索记录）。
        // run_dir 名用「用例 slug」（最终 .tks 文件名由 AI 起，两者解耦，run_end 里有脚本路径）
        let stem = slug(&opts.case, 30);
        let log_root = opts.params.log.clone().unwrap_or_else(|| opts.script_dir.clone());
        let artifacts = RunArtifacts::create(&log_root, &stem)?;
        let run_dir = artifacts.run_dir.clone();
        let conversation_path = run_dir.join("conversation.jsonl");

        let mut tx = Transcript::create(conversation_path.clone())?;

        // —— 提示词（可自定义：注入文本 / .md 文件 / 目录覆盖）——
        let prompts = PromptSet::resolve(&opts.prompt)?;

        // —— 记忆 + 知识库（本期：未配置则跳过真实调用，记 skipped）——
        let knowledge = Knowledge::new(&opts.params.knowledge);
        record_knowledge(&mut tx, "memory_query", knowledge.query_memory(&opts.case));
        record_knowledge(&mut tx, "rag_query", knowledge.query_rag(&opts.case));

        // —— 会话 ——
        let system_prompt = prompts.system(&device, platform.name());
        tx.log("system_prompt", serde_json::json!({ "content": system_prompt.clone() }));
        tx.log(
            "case",
            serde_json::json!({ "content": opts.case.clone(), "device": device.clone(), "platform": platform.name() }),
        );
        tx.log(
            "config",
            serde_json::json!({
                "provider": opts.ai.provider.clone().unwrap_or_else(|| "anthropic".into()),
                "model": opts.ai.model.clone().unwrap_or_default(),
                "element_library": element_path.to_string_lossy(),
                "run_dir": run_dir.to_string_lossy(),
            }),
        );

        let tools = build_tools(&prompts);
        // 工具定义全集（AI 每轮实际收到的输入的一部分：名字 + description 提示词 + 参数 schema，
        // 含统一注入的 comment 字段）。记进 transcript，使 conversation.json 不漏 AI 所见的任何输入。
        tx.log(
            "tools",
            serde_json::json!({
                "count": tools.len(),
                "tools": tools.iter().map(|t| serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "schema": t.schema,
                })).collect::<Vec<_>>(),
            }),
        );
        let mut sess = LlmSession::new(&opts.ai, system_prompt, tools)?;
        tx.log("model", serde_json::json!({ "model": sess.model() }));
        sess.user(format!("测试用例：\n{}\n\n请开始探索测试。", opts.case));

        // —— 驱动循环 ——
        let workarea = Workarea::for_device(Some(&device))?;
        let fetcher = Fetcher::new();
        let max_rounds = opts.ai.max_rounds.unwrap_or(40) as usize;
        let start_time = chrono::Local::now().to_rfc3339();
        let ctx = DriveCtx {
            device: &device,
            element_path: &element_path,
            workarea: &workarea,
            fetcher: &fetcher,
            artifacts: &artifacts,
            ocr: opts.ocr.as_ref(),
            max_rounds,
            prompts: &prompts,
        };
        let outcome = drive(&mut sess, &mut tx, &ctx, true, "").await?;
        let end_time = chrono::Local::now().to_rfc3339();

        // —— 脚本文件名：用 AI 在 finish 起的名（概括本次测试），兜底用例 slug；
        //    在脚本目录内去重，绝不覆盖已有脚本 ——
        let base = outcome
            .script_name
            .as_deref()
            .map(|s| slug(s, 50))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| slug(&opts.case, 40));
        let script_path = unique_script_path(&opts.script_dir, &base);

        // 先落探索原始脚本；提炼(删冗余步)在 verify 阶段做——靠"删一步就重跑验证"
        // 来决定能不能删，而不是对着文本想当然，避免删掉必需步把脚本搞断。
        let final_lines = outcome.lines.clone();
        write_script(&script_path, &opts.case, &final_lines)?;
        // 明确标出生成结果，与后续验证阶段分开（不再把生成完成信息混在步骤里）。
        let tty = std::io::stderr().is_terminal();
        let fname = script_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        eprintln!();
        if outcome.success {
            eprintln!("{}", flow::paint(tty, "1;32", &format!("✓ 脚本生成完毕：{}（{} 步）", fname, final_lines.len())));
        } else {
            eprintln!(
                "{}",
                flow::paint(tty, "1;33", &format!("⚠ 探索未达成，脚本不完整：{}（{} 步，已保存当前进度，跳过验证）", fname, final_lines.len()))
            );
        }

        // —— 再验证：回放提炼后的脚本，失败让 AI 续接修复，连过 2 次才算稳定 ——
        // 仅当探索**达成且未被中断**时才验证：脚本没完成就别去验证一个半成品。
        // result_script_lines：最终落盘的完整脚本（验证后可能更短）；用于 run_end 完整记录。
        let mut result_script_lines = final_lines.clone();
        let verified = if opts.verify && outcome.success && !outcome.aborted {
            // 回放产物（标注截图/页面结构）落在 run_dir/verify 下，便于复盘哪步怎么错
            let verify_log = run_dir.join("verify");
            let (verified_lines, rep) = verify::verify_and_repair(
                &mut sess,
                &opts.ai,
                &prompts,
                &mut tx,
                &ctx,
                &opts.params,
                &script_path,
                &opts.case,
                Some(verify_log.as_path()),
                final_lines.clone(),
            )
            .await;
            if !verified_lines.is_empty() {
                result_script_lines = verified_lines;
            }
            Some(rep)
        } else {
            None
        };
        // 最终成败：探索达成 且（未自检 或 自检稳定通过）
        let stable = verified.as_ref().map(|r| r.passed);
        let overall_success = outcome.success && stable.unwrap_or(true);

        // —— log.json(ExecutionResult，与 run 同构) ——
        // 总 token = 探索会话(含活体重探复用的同一会话) + 脚本医生独立会话(report.extra_*)
        let (mut total_prompt, mut total_completion) = sess.total_usage();
        if let Some(r) = &verified {
            total_prompt += r.extra_prompt;
            total_completion += r.extra_completion;
        }
        tx.log(
            "run_end",
            serde_json::json!({
                "success": overall_success,
                "explore_success": outcome.success,
                "reason": outcome.reason.clone(),
                "rounds": outcome.rounds,
                "steps": outcome.steps.len(),
                "script": script_path.to_string_lossy(),
                "final_script": result_script_lines,
                "model": sess.model(),
                "prompt_tokens": total_prompt,
                "completion_tokens": total_completion,
                "total_tokens": total_prompt + total_completion,
                "elements_created": outcome.created.clone(),
                "elements_updated": outcome.updated.clone(),
                "verify": verified.as_ref().map(|r| serde_json::json!({
                    "ran": r.ran, "passed": r.passed, "repairs": r.repairs, "final_steps": r.final_steps
                })),
            }),
        );

        let exec_result = ExecutionResult {
            success: overall_success,
            case_id: String::new(),
            script_name: stem,
            start_time,
            end_time,
            steps: outcome.steps,
            error: if overall_success { None } else { Some(outcome.reason.clone()) },
            script_path: Some(script_path.to_string_lossy().to_string()),
            run_dir: Some(run_dir.to_string_lossy().to_string()),
            launched_packages: Vec::new(),
        };
        let _ = artifacts.write_log(&exec_result);

        // 导出人类可读的 conversation.json（缩进美化数组）；conversation.jsonl 仍在（抗崩溃流式）
        let conversation_json = run_dir.join("conversation.json");
        let _ = tx.finalize(&conversation_json);

        // —— 统一在最末尾渲染「结果 + 元素库」框：放到 verify 之后，
        //    token 总量才覆盖探索+验证+修复全程；并多给一行「验证状态」——
        let mut all_created = outcome.created.clone();
        if let Some(r) = &verified {
            for c in &r.created {
                if !all_created.contains(c) {
                    all_created.push(c.clone());
                }
            }
        }
        // 防污染：运行未成功（探索未达成 / 验证未通过）就**既不留脚本、也回滚元素**——
        // 不稳定的半成品脚本和它造的垃圾元素都不该留下骗下次。成功(稳定通过)才保留、沉淀复用。
        let rolled_back = if !overall_success {
            let _ = std::fs::remove_file(&script_path); // 删掉不稳定的脚本
            if !all_created.is_empty() {
                crate::tools::element::remove_elements(&element_path, &all_created).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };
        render_summary(
            outcome.success,
            outcome.aborted,
            &outcome.reason,
            outcome.rounds,
            final_lines.len(),
            verified.as_ref(),
            sess.model(),
            total_prompt,
            total_completion,
            &all_created,
            rolled_back,
            &element_path,
        );

        Ok(AgentResult {
            success: overall_success,
            rounds: outcome.rounds,
            script: script_path,
            conversation: conversation_json,
            finish_reason: outcome.reason,
            aborted: outcome.aborted,
        })
    }
}

/// 末尾统一渲染「结果 + 元素库更新」框（在 verify 之后调用，token 覆盖全程）。
/// 结果框分**三层状态**：探索（探索 agent 是否完成 + 原始步数/轮数）、
/// 诊断（脚本医生复跑修复：优化完成 / 优化达上限仍可跑 / 修复失败 + 修复次数 + 最终步数）、
/// 验证（稳定性测试：连续通过几次 / 未通过 / 未验证）。
#[allow(clippy::too_many_arguments)]
fn render_summary(
    success: bool,
    aborted: bool,
    reason: &str,
    rounds: usize,
    explore_steps: usize,
    verified: Option<&options::VerifyReport>,
    model: &str,
    tp: i64,
    tc: i64,
    created: &[String],
    rolled_back: usize,
    element_path: &Path,
) {
    use flow::{brief, fmt_tokens, paint};
    let tty = std::io::stderr().is_terminal();

    // ① 探索状态
    let explore_status = if aborted {
        paint(tty, "33", &format!("■ 已终止 · 原始 {} 步（{} 轮）", explore_steps, rounds))
    } else if success {
        paint(tty, "32", &format!("✓ 达成 · 原始 {} 步（{} 轮）", explore_steps, rounds))
    } else {
        paint(tty, "31", &format!("✗ 未达成 · {} 步（{} 轮）", explore_steps, rounds))
    };
    // ② 诊断状态（脚本医生复跑修复）
    let diagnose_status = match verified {
        None => paint(tty, "2", "未运行（未开启 --verify 或探索未达成）"),
        Some(r) if !r.reached => paint(tty, "31", "✗ 修复失败（脚本仍跑不到目标）"),
        Some(r) if r.hit_iter_limit => paint(tty, "33", &format!("■ 优化达上限·仍可跑（修复 {} 次 · 最终 {} 步）", r.repairs, r.final_steps)),
        Some(r) => paint(tty, "32", &format!("✓ 优化完成（修复 {} 次 · 最终 {} 步）", r.repairs, r.final_steps)),
    };
    // ③ 验证状态（稳定性测试）
    let verify_status = match verified {
        None => paint(tty, "2", "未验证"),
        Some(r) if r.passed => paint(tty, "32", &format!("✓ 验证通过（连续 {} 次到达目标）", r.stability_passes)),
        Some(r) if !r.reached => paint(tty, "2", "未验证（诊断未修复，未进入稳定性测试）"),
        Some(r) => paint(tty, "31", &format!("✗ 验证未通过（连续到达 {} 次）", r.stability_passes)),
    };

    eprintln!();
    eprintln!("{}", paint(tty, "1", "╭─ 结果 ──────────────────────────────"));
    eprintln!("  {}   {}", paint(tty, "2", "探索"), explore_status);
    eprintln!("  {}   {}", paint(tty, "2", "诊断"), diagnose_status);
    eprintln!("  {}   {}", paint(tty, "2", "验证"), verify_status);
    eprintln!("  {}   {}", paint(tty, "2", "依据"), brief(reason, 200));
    eprintln!("  {}   {}", paint(tty, "2", "模型"), model);
    eprintln!(
        "  {}  {}",
        paint(tty, "2", "Token"),
        paint(tty, "2", &format!("↑{} ↓{} · 合计 {}", fmt_tokens(tp), fmt_tokens(tc), fmt_tokens(tp + tc)))
    );
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));

    // 元素库更新：合并探索+修复阶段新增，desc 从库里读（探索/修复各自的 desc-pass 已写入）
    eprintln!();
    eprintln!("{}", paint(tty, "1", "╭─ 元素库更新 ────────────────────────"));
    let overall = !aborted && success && verified.map(|r| r.passed).unwrap_or(true);
    if !overall {
        // 运行未成功（未稳定通过）：脚本与本次新建元素都已清理，不留半成品、不污染
        eprintln!(
            "  {}",
            paint(tty, "33", &format!("未稳定通过——脚本未保存，本次新建的 {} 个元素已清理（不留半成品、不污染元素库）", rolled_back))
        );
        eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
        return;
    }
    let descs = read_descs(element_path, created);
    if created.is_empty() {
        eprintln!("  {}   {}", paint(tty, "2", "新增"), paint(tty, "2", "（无）"));
    } else {
        let line_for = |c: &String| match descs.get(c) {
            Some(Some(d)) => format!("{} · {}", c, brief(d, 80)),
            _ => c.clone(),
        };
        eprintln!("  {}   {}", paint(tty, "2", "新增"), paint(tty, "32", &line_for(&created[0])));
        for c in &created[1..] {
            eprintln!("         {}", paint(tty, "32", &line_for(c)));
        }
        eprintln!("  {}", paint(tty, "2", "（已新增，desc 据实际作用生成，请人工二次审核）"));
    }
    eprintln!("{}", paint(tty, "1", "╰─────────────────────────────────────"));
}

/// 从 element.json 读出若干元素的 desc（用于最终汇总展示）
fn read_descs(lib_path: &Path, names: &[String]) -> std::collections::HashMap<String, Option<String>> {
    let lib: serde_json::Value = std::fs::read_to_string(lib_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    names
        .iter()
        .map(|name| (name.clone(), lib["elements"][name]["desc"].as_str().map(|s| s.to_string())))
        .collect()
}

/// 记录一次知识检索结果
fn record_knowledge(tx: &mut Transcript, kind: &str, outcome: KnowledgeOutcome) {
    match outcome {
        KnowledgeOutcome::Hit(ctx) => tx.log(kind, serde_json::json!({ "hit": true, "context": ctx })),
        KnowledgeOutcome::Skipped(why) => tx.log(kind, serde_json::json!({ "hit": false, "skipped": why })),
    }
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
