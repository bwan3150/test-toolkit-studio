// 【编排】AgentRunner：装配各子模块（提示词/会话/感知/执行/记忆/日志）并驱动循环
// 本文件只做"装配 + 收尾"，循环逻辑在 flow.rs。

pub mod flow;
pub mod options;
pub mod verify;

pub use options::{AgentResult, AgentRunOptions};

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
        };
        let outcome = drive(&mut sess, &mut tx, &ctx, true).await?;
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

        // 写 .tks（先落初版，供 --verify 回放）
        write_script(&script_path, &opts.case, &outcome.lines)?;

        // —— --verify：生成后自检 + 自修复（重启净化→整脚本回放→失败让 AI 续接修复→连过 2 次）——
        // 未被中断时才跑；verify 内部会把最终脚本写回 script_path。
        let verified = if opts.verify && !outcome.aborted {
            let (_final_lines, rep) = verify::verify_and_repair(
                &mut sess,
                &mut tx,
                &ctx,
                &opts.params,
                &script_path,
                &opts.case,
                None, // 回放产物不另存（失败信息已记入 conversation 的 verify_replay）
                outcome.lines.clone(),
            )
            .await;
            Some(rep)
        } else {
            None
        };
        // 最终成败：探索达成 且（未自检 或 自检稳定通过）
        let stable = verified.as_ref().map(|r| r.passed);
        let overall_success = outcome.success && stable.unwrap_or(true);

        // —— log.json(ExecutionResult，与 run 同构) ——
        let (total_prompt, total_completion) = sess.total_usage();
        tx.log(
            "run_end",
            serde_json::json!({
                "success": overall_success,
                "explore_success": outcome.success,
                "reason": outcome.reason.clone(),
                "rounds": outcome.rounds,
                "steps": outcome.steps.len(),
                "script": script_path.to_string_lossy(),
                "model": sess.model(),
                "prompt_tokens": total_prompt,
                "completion_tokens": total_completion,
                "total_tokens": total_prompt + total_completion,
                "elements_created": outcome.created.clone(),
                "elements_updated": outcome.updated.clone(),
                "verify": verified.as_ref().map(|r| serde_json::json!({
                    "ran": r.ran, "passed": r.passed, "repairs": r.repairs
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
