// 【编排】AgentRunner：装配各子模块（提示词/会话/感知/执行/记忆/日志）并驱动循环
// 本文件只做"装配 + 收尾"，循环逻辑在 flow.rs。

pub mod flow;
pub mod options;

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

        // 元素库（参数层查表；缺省落 script 同级 element.json）
        let element_path: PathBuf = opts.params.element_lib().unwrap_or_else(|| {
            opts.script_out
                .parent()
                .unwrap_or(Path::new("."))
                .join("element.json")
        });

        // —— 产物目录：复用 RunArtifacts，与 tke run 同构 ——
        // log 根：--log/config 优先；缺省落 script 同级（case 始终保留探索记录）
        let stem = opts
            .script_out
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "case".to_string());
        let log_root = opts.params.log.clone().unwrap_or_else(|| {
            opts.script_out.parent().unwrap_or(Path::new(".")).to_path_buf()
        });
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
            max_rounds,
        };
        let outcome = drive(&mut sess, &mut tx, &ctx).await?;
        let end_time = chrono::Local::now().to_rfc3339();

        // —— 收尾：写 .tks 脚本(--script) + log.json(ExecutionResult，与 run 同构) ——
        write_script(&opts.script_out, &opts.case, &outcome.lines)?;
        tx.log(
            "run_end",
            serde_json::json!({
                "success": outcome.success,
                "reason": outcome.reason.clone(),
                "rounds": outcome.rounds,
                "steps": outcome.steps.len(),
                "script": opts.script_out.to_string_lossy(),
            }),
        );

        let exec_result = ExecutionResult {
            success: outcome.success,
            case_id: String::new(),
            script_name: stem,
            start_time,
            end_time,
            steps: outcome.steps,
            error: if outcome.success { None } else { Some(outcome.reason.clone()) },
            script_path: Some(opts.script_out.to_string_lossy().to_string()),
            run_dir: Some(run_dir.to_string_lossy().to_string()),
            launched_packages: Vec::new(),
        };
        let _ = artifacts.write_log(&exec_result);

        // 导出人类可读的 conversation.json（缩进美化数组）；conversation.jsonl 仍在（抗崩溃流式）
        let conversation_json = run_dir.join("conversation.json");
        let _ = tx.finalize(&conversation_json);

        Ok(AgentResult {
            success: outcome.success,
            rounds: outcome.rounds,
            script: opts.script_out.clone(),
            conversation: conversation_json,
            finish_reason: outcome.reason,
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
