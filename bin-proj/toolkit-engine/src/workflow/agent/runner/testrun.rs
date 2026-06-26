// 【TestRun】一条用例的「跨阶段运行态」+ 三个阶段方法（探索 / 验证 / 收尾）。
//
// slice 2 地基：原 run_one_testcase 是一根 330 行的线性函数，探索/验证/收尾的状态全是 local、
// 高度交织（探索会话被 verify 复用、token 账目跨阶段累计、临时元素库定稿才提交）。这里把这些
// local 抽成 TestRun 的字段，把三段切成方法——**行为逐字不变**，只是状态有了载体。
// 这样编排官(orchestrator)下一步就能把 explore/verify/finalize 当**独立工具**分别调度
// （验证三段变 todo、非线性），而不必把整条流水线当一个黑盒 run_testcase。
//
// 借用要点：DriveCtx 借的是各字段的引用，故每个方法里**临时重建 ctx**（借 &self 的离散字段），
// 再配合 &mut self.sess / &mut self.tx 的不相交借用驱动——字段两两不同，NLL 下可过。

use std::path::PathBuf;
use std::sync::Arc;

use crate::models::Platform;
use crate::{ExecutionResult, Fetcher, LlmSession, Result, RunArtifacts, Workarea};

use super::super::execution::script::write_script;
use super::super::knowledge::Knowledge;
use super::super::prompt::{render, PromptSet};
use super::super::tools::build_tools;
use super::super::transcript::Transcript;
use super::super::ui::{Frontend, Level, Phase, SubAgent, Tokens, UiEvent};
use super::flow::{drive, DriveCtx, DriveOutcome};
use super::options::{AgentResult, AgentRunOptions, VerifyReport};
use super::{record_knowledge, referenced_element_names, render_summary, slug, unique_script_path};
use super::{reflect, verify};

/// 一条用例的运行态：探索阶段建好，验证/收尾阶段在其上推进。
pub(crate) struct TestRun {
    // —— 环境（一次 run 内不变）——
    device: String,
    platform: Platform,
    case: String,
    prompts: PromptSet,
    // —— 产物目录 / 元素库 ——
    artifacts: RunArtifacts,
    run_dir: PathBuf,
    element_path: PathBuf,      // 临时库（run_dir 下，探索/修复/验证全程读写）
    real_element_path: PathBuf, // 正式库（仅定稿后提交）
    stem: String,
    tx: Transcript,
    // —— 探索会话（verify 的活体重探复用同一会话）——
    sess: LlmSession,
    workarea: Workarea,
    fetcher: Fetcher,
    max_rounds: usize,
    start_time: String,
    end_time: String,
    // —— 探索结果 ——
    outcome: DriveOutcome,
    script_path: PathBuf,
    final_lines: Vec<String>, // 探索原始脚本（落盘后、验证前）
    // —— 跨阶段 token 账目 ——
    refl_pt: i64,
    refl_ct: i64,
    discarded_pt: i64,
    discarded_ct: i64,
    sup_pt: i64,
    sup_ct: i64,
    explore_created: Vec<String>,
    // —— 验证结果（收尾用）——
    result_lines: Vec<String>, // 最终落盘脚本（验证后可能更短）
    verified: Option<VerifyReport>,
}

/// 在调用点内联展开出一个 DriveCtx（借 $r 的**离散字段**，与 $r.sess/$r.tx 的 &mut 不相交）。
/// 不能写成 `fn ctx(&self)` ——那样 &self 接收者会借走整个 self，和 &mut self.sess 冲突。
macro_rules! drive_ctx {
    ($r:expr, $opts:expr, $ui:expr) => {
        DriveCtx {
            device: &$r.device,
            element_path: &$r.element_path,
            workarea: &$r.workarea,
            fetcher: &$r.fetcher,
            artifacts: &$r.artifacts,
            ocr: $opts.ocr.as_ref(),
            max_rounds: $r.max_rounds,
            prompts: &$r.prompts,
            case: &$r.case,
            ai: &$opts.ai,
            ui: $ui,
        }
    };
}

impl TestRun {
    /// 阶段一：装配环境 + 初探 + 失败重探 + 落探索原始脚本。返回带完整运行态的 TestRun。
    pub(crate) async fn explore(
        opts: &AgentRunOptions,
        ui: &dyn Frontend,
        case: &str,
        note: Option<&str>,
    ) -> Result<TestRun> {
        // 设备/平台优先级：显式覆盖（向导/--platform/-d）> params.device() > 设备推断
        let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
        let platform = opts.platform.unwrap_or_else(|| Platform::from_device(Some(&device)));

        // 脚本输出目录确保存在（.tks 文件名由 AI 在 finish 起、落库时去重）
        std::fs::create_dir_all(&opts.script_dir).ok();

        // 正式元素库（参数层查表；缺省落脚本目录下的 element.json，整目录脚本共享一份库）。
        // **探索/修复/验证全程不直接写它**——只在脚本定稿(稳定通过)后，把最终脚本实际用到的元素提交进来。
        let real_element_path: PathBuf = opts
            .params
            .element_lib()
            .unwrap_or_else(|| opts.script_dir.join("element.json"));

        // —— 产物目录：复用 RunArtifacts，与 tke run 同构 ——
        let stem = slug(case, 30);
        let log_root = opts.params.log.clone().unwrap_or_else(|| opts.script_dir.clone());
        let artifacts = RunArtifacts::create(&log_root, &stem)?;
        let run_dir = artifacts.run_dir.clone();
        // 临时元素库：本次运行隔离（run_dir 下）。
        let element_path = run_dir.join("element.json");
        let conversation_path = run_dir.join("conversation.jsonl");

        let mut tx = Transcript::create(conversation_path.clone())?;

        // —— 提示词（可自定义：注入文本 / .md 文件 / 目录覆盖）——
        let prompts = PromptSet::resolve(&opts.prompt)?;

        // —— 记忆 + 知识库（本期：未配置则跳过真实调用，记 skipped）——
        let knowledge = Knowledge::new(&opts.params.knowledge);
        record_knowledge(&mut tx, "memory_query", knowledge.query_memory(case));
        record_knowledge(&mut tx, "rag_query", knowledge.query_rag(case));

        // —— 会话 ——
        let system_prompt = prompts.role_system("explorer", &device, platform.name());
        tx.log("system_prompt", serde_json::json!({ "content": system_prompt.clone() }));
        tx.log(
            "case",
            serde_json::json!({ "content": case, "device": device.clone(), "platform": platform.name() }),
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

        let tools = build_tools(&prompts, platform);
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
        let case_msg = render(&prompts.message("explorer", "case_intro"), &[("case", case)]);
        tx.log("llm_message", serde_json::json!({ "content": case_msg.clone() }));
        sess.user(case_msg);
        // 编排官下发的额外约束（用户纠偏/已否定路径等）：作为硬约束追加在用例之后
        if let Some(n) = note.map(str::trim).filter(|s| !s.is_empty()) {
            let guide = format!("【编排官下发的约束/提示，请务必遵守】\n{}", n);
            tx.log("llm_message", serde_json::json!({ "content": guide.clone() }));
            sess.user(guide);
        }

        // —— 驱动循环准备 ——
        let workarea = Workarea::for_device(Some(&device))?;
        let fetcher = Fetcher::new();
        let max_rounds = opts.ai.max_rounds.unwrap_or(40) as usize;
        let start_time = chrono::Local::now().to_rfc3339();

        // 先把运行态装好（ctx 在驱动时临时重建）
        let mut run = TestRun {
            device,
            platform,
            case: case.to_string(),
            prompts,
            artifacts,
            run_dir,
            element_path,
            real_element_path,
            stem,
            tx,
            sess,
            workarea,
            fetcher,
            max_rounds,
            start_time,
            end_time: String::new(),
            outcome: DriveOutcome::default(),
            script_path: PathBuf::new(),
            final_lines: Vec::new(),
            refl_pt: 0,
            refl_ct: 0,
            discarded_pt: 0,
            discarded_ct: 0,
            sup_pt: 0,
            sup_ct: 0,
            explore_created: Vec::new(),
            result_lines: Vec::new(),
            verified: None,
        };

        // —— 开局净化：关掉可能残留的旧会话（如上次没关的浏览器）——
        if matches!(run.platform, Platform::Web) {
            let _ = super::super::execution::device::exec(&run.device, crate::ControlAction::Close { package: String::new() }).await;
        }

        // 顶栏状态：进入初探
        ui.emit(UiEvent::Phase { phase: Phase::Explore, n: None });
        let ctx = drive_ctx!(run, opts, ui);
        let mut outcome = drive(&mut run.sess, &mut run.tx, &ctx, true, "").await?;
        drop(ctx);

        run.sup_pt = outcome.subagent_pt;
        run.sup_ct = outcome.subagent_ct;
        run.explore_created = outcome.created.clone();

        // —— 失败/卡住 → 反思官给「重探指导」，带指导从头重探 ——
        let max_reexplore = opts.params.harness.reexplore;
        let mut reexplore_n = 0;
        while !outcome.success && !outcome.aborted && reexplore_n < max_reexplore {
            reexplore_n += 1;
            let refl_pt_this;
            let refl_ct_this;
            let guidance = match reflect::reflect(&opts.ai, &run.prompts, &mut run.tx, &run.device, &run.fetcher, &run.run_dir, &run.case, &outcome, false).await {
                Some(r) => {
                    run.refl_pt += r.prompt_tokens;
                    run.refl_ct += r.completion_tokens;
                    refl_pt_this = r.prompt_tokens;
                    refl_ct_this = r.completion_tokens;
                    r.report
                }
                None => break,
            };
            ui.emit(UiEvent::Phase { phase: Phase::Reexplore, n: Some(reexplore_n as u32) });
            ui.emit(UiEvent::SubAgent {
                kind: SubAgent::Reflector,
                level: Level::Info,
                text: guidance.clone(),
                tokens: Tokens::new(refl_pt_this, refl_ct_this),
            });
            // 旧探索会话即将弃用，先记下它的 token
            let (dp, dc) = run.sess.total_usage();
            run.discarded_pt += dp;
            run.discarded_ct += dc;
            // 全新探索会话 + 用例 + 重探指导，从头带指导重探
            run.sess = LlmSession::new(&opts.ai, run.prompts.role_system("explorer", &run.device, run.platform.name()), build_tools(&run.prompts, run.platform))?;
            let case_msg = render(&run.prompts.message("explorer", "case_intro"), &[("case", run.case.as_str())]);
            run.tx.log("llm_message", serde_json::json!({ "content": case_msg.clone() }));
            run.sess.user(case_msg);
            let guide_msg = format!(
                "【上一轮探索没找到目标。探索反思官给的重探指导】\n{}\n\n请据此从头重新探索，直接走对的路、别再重蹈覆辙。",
                guidance
            );
            run.tx.log("llm_message", serde_json::json!({ "content": guide_msg.clone() }));
            run.sess.user(guide_msg);
            let ctx = drive_ctx!(run, opts, ui);
            outcome = drive(&mut run.sess, &mut run.tx, &ctx, true, &format!("重探{}·", reexplore_n)).await?;
            drop(ctx);
            run.sup_pt += outcome.subagent_pt;
            run.sup_ct += outcome.subagent_ct;
            for c in &outcome.created {
                if !run.explore_created.contains(c) {
                    run.explore_created.push(c.clone());
                }
            }
        }

        run.end_time = chrono::Local::now().to_rfc3339();

        // —— 脚本文件名：用 AI 在 finish 起的名，兜底用例 slug；目录内去重 ——
        let base = outcome
            .script_name
            .as_deref()
            .map(|s| slug(s, 50))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| slug(&run.case, 40));
        let script_path = unique_script_path(&opts.script_dir, &base);

        // 先落探索原始脚本；提炼(删冗余步)在 verify 阶段做。
        let final_lines = outcome.lines.clone();
        write_script(&script_path, &run.case, &final_lines)?;
        let fname = script_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        ui.emit(UiEvent::ScriptGenerated { name: fname.clone(), steps: final_lines.len(), success: outcome.success });

        run.result_lines = final_lines.clone();
        run.final_lines = final_lines;
        run.script_path = script_path;
        run.outcome = outcome;
        Ok(run)
    }

    /// 阶段二：回放提炼后的脚本，失败让 AI 续接修复，连过 2 次才算稳定。
    /// 仅当探索**达成且未被中断**且开启 --verify 时才真正验证；否则只是空过。
    pub(crate) async fn verify(&mut self, opts: &AgentRunOptions, ui: &dyn Frontend) {
        // 诊断/验证回放必须读**临时库**，且用探索同一个设备（含向导选的 web）。
        let tmp_params = Arc::new(
            opts.params
                .with_element_lib(self.element_path.clone())
                .with_device(Some(self.device.clone())),
        );
        if opts.verify && self.outcome.success && !self.outcome.aborted {
            let verify_log = self.run_dir.join("verify");
            let ctx = drive_ctx!(self, opts, ui);
            let (verified_lines, rep) = verify::verify_and_repair(
                &mut self.sess,
                &opts.ai,
                &self.prompts,
                &mut self.tx,
                &ctx,
                &tmp_params,
                &self.script_path,
                &self.case,
                Some(verify_log.as_path()),
                self.final_lines.clone(),
            )
            .await;
            if !verified_lines.is_empty() {
                self.result_lines = verified_lines;
            }
            self.verified = Some(rep);
        }
    }

    /// 阶段三：写 run_end / log.json / conversation.json，定稿命名 + 提交元素库，渲染结果框，返回结果。
    pub(crate) async fn finalize(mut self, opts: &AgentRunOptions, ui: &dyn Frontend) -> Result<AgentResult> {
        // 最终成败：探索达成 且（未自检 或 自检稳定通过）
        let stable = self.verified.as_ref().map(|r| r.passed);
        let overall_success = self.outcome.success && stable.unwrap_or(true);

        // 总 token = 探索会话 + 被弃用重探旧会话 + 反思官 + 监督官 + 脚本医生独立会话
        let (mut total_prompt, mut total_completion) = self.sess.total_usage();
        total_prompt += self.discarded_pt + self.refl_pt + self.sup_pt;
        total_completion += self.discarded_ct + self.refl_ct + self.sup_ct;
        if let Some(r) = &self.verified {
            total_prompt += r.extra_prompt;
            total_completion += r.extra_completion;
        }
        let model = self.sess.model().to_string();
        self.tx.log(
            "run_end",
            serde_json::json!({
                "success": overall_success,
                "explore_success": self.outcome.success,
                "reason": self.outcome.reason.clone(),
                "rounds": self.outcome.rounds,
                "steps": self.outcome.steps.len(),
                "script": self.script_path.to_string_lossy(),
                "final_script": self.result_lines,
                "model": model,
                "prompt_tokens": total_prompt,
                "completion_tokens": total_completion,
                "total_tokens": total_prompt + total_completion,
                "elements_created": self.outcome.created.clone(),
                "elements_updated": self.outcome.updated.clone(),
                "verify": self.verified.as_ref().map(|r| serde_json::json!({
                    "ran": r.ran, "passed": r.passed, "repairs": r.repairs, "final_steps": r.final_steps
                })),
            }),
        );

        let exec_result = ExecutionResult {
            success: overall_success,
            case_id: String::new(),
            script_name: self.stem.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            steps: std::mem::take(&mut self.outcome.steps),
            error: if overall_success { None } else { Some(self.outcome.reason.clone()) },
            script_path: Some(self.script_path.to_string_lossy().to_string()),
            run_dir: Some(self.run_dir.to_string_lossy().to_string()),
            launched_packages: Vec::new(),
        };
        let _ = self.artifacts.write_log(&exec_result);

        // 导出人类可读的 conversation.json（缩进美化数组）；conversation.jsonl 仍在（抗崩溃流式）
        let conversation_json = self.run_dir.join("conversation.json");
        let _ = self.tx.finalize(&conversation_json);

        // 本次运行新建的所有元素（跨重探累计 + 验证/修复阶段）
        let mut all_created = self.explore_created.clone();
        if let Some(r) = &self.verified {
            for c in &r.created {
                if !all_created.contains(c) {
                    all_created.push(c.clone());
                }
            }
        }
        // 防污染（临时库架构）：成功→把终稿引用到的元素从临时库提交正式库；失败→删脚本、丢临时库即可。
        let referenced = referenced_element_names(&self.result_lines);
        let feat_names: Vec<String> = all_created.iter().filter(|n| referenced.contains(n.as_str())).cloned().collect();
        let (display_created, committed) = if overall_success {
            // 定稿语义命名：把终稿用到的「特征自动名」元素改成语义名(改临时库 key + 脚本引用)，再提交正式库
            let (renamed, semantic) =
                reflect::finalize_names(&opts.ai, &self.prompts, &mut self.tx, &self.element_path, &self.result_lines, &feat_names).await;
            self.result_lines = renamed;
            let _ = write_script(&self.script_path, &self.case, &self.result_lines); // 落盘改名后的脚本
            let n = crate::tools::element::commit_elements(&self.element_path, &self.real_element_path, &semantic).unwrap_or(0);
            (semantic, n)
        } else {
            let _ = std::fs::remove_file(&self.script_path);
            (feat_names, 0)
        };
        render_summary(
            self.outcome.success,
            self.outcome.aborted,
            &self.outcome.reason,
            self.outcome.rounds,
            self.final_lines.len(),
            self.verified.as_ref(),
            &model,
            total_prompt,
            total_completion,
            &display_created,
            committed,
            &self.real_element_path,
            ui,
        );

        Ok(AgentResult {
            success: overall_success,
            rounds: self.outcome.rounds,
            script: self.script_path,
            conversation: conversation_json,
            finish_reason: self.outcome.reason,
            aborted: self.outcome.aborted,
        })
    }
}
