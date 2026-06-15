// 【编排】AgentRunner：装配各子模块（提示词/会话/感知/执行/记忆/日志）并驱动循环
// 本文件只做"装配 + 收尾"，循环逻辑在 flow.rs。

pub mod flow;
pub mod options;

pub use options::{AgentResult, AgentRunOptions};

use std::path::{Path, PathBuf};

use crate::models::Platform;
use crate::{Fetcher, LlmSession, Result, Workarea};

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
        let device = opts.device.clone();
        let platform = Platform::from_device(Some(&device));

        // —— 产物路径 ——
        let element_path: PathBuf = opts.element.clone().unwrap_or_else(|| {
            opts.script_out
                .parent()
                .unwrap_or(Path::new("."))
                .join("element.json")
        });
        let conversation_path = opts.script_out.with_extension("conversation.jsonl");
        let stem = opts
            .script_out
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "case".to_string());
        let screens_dir = opts
            .script_out
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{}.screens", stem));

        let mut tx = Transcript::create(conversation_path.clone())?;

        // —— 提示词（可自定义：注入文本 / .md 文件 / 目录覆盖）——
        let prompts = PromptSet::resolve(&opts.prompt)?;

        // —— 记忆 + 知识库（本期：未配置则跳过真实调用，记 skipped）——
        let knowledge = Knowledge::new(&opts.knowledge);
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
        let ctx = DriveCtx {
            device: &device,
            element_path: &element_path,
            workarea: &workarea,
            fetcher: &fetcher,
            screens_dir: &screens_dir,
            max_rounds,
        };
        let outcome = drive(&mut sess, &mut tx, &ctx).await?;

        // —— 收尾：写出 .tks ——
        write_script(&opts.script_out, &opts.case, &outcome.lines)?;
        tx.log(
            "run_end",
            serde_json::json!({
                "success": outcome.success,
                "reason": outcome.reason.clone(),
                "rounds": outcome.rounds,
                "steps": outcome.lines.len(),
                "script": opts.script_out.to_string_lossy(),
            }),
        );

        Ok(AgentResult {
            success: outcome.success,
            rounds: outcome.rounds,
            script: opts.script_out.clone(),
            conversation: conversation_path,
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
