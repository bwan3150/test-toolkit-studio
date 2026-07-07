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

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::models::Platform;
use crate::{ControlAction, ExecutionResult, Fetcher, LlmSession, Point, Result, RunArtifacts, Workarea};

use super::super::perception::capture;

use super::super::execution::script::write_script;
use super::super::knowledge::Knowledge;
use super::super::prompt::{render, PromptSet};
use super::super::tools::build_tools;
use super::super::transcript::Transcript;
use super::super::ui::{Frontend, Level, Phase, SubAgent, Tokens, UiEvent};
use super::ctx::{AskMode, DriveCtx, DriveOutcome};
use super::flow::drive;
use super::options::{AgentResult, AgentRunOptions, VerifyReport};
use super::{record_knowledge, referenced_element_names, render_summary, slug, unique_script_path};
use super::reflect;

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
    element_path: PathBuf, // 临时库（run_dir 下，探索/修复/验证全程读写）
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

    // —— 模式 + 通用任务交付物 ——
    /// 轻模式（通用任务/不要求校验）：驱动时跳过踩实官(自动断言)与监督官(finish 把关)；
    /// 仍照常产 .tks/log/元素。false=完整测试（开断言+把关，脚本可验证）。
    task_mode: bool,
    /// explorer 提问模式（编排官按用户意愿下发：Ask=转给用户/Auto=参谋托管代答）
    ask_mode: AskMode,
    /// 末页可读文字（驱动结束后捕获，read_full 时滚动逐屏收集）——供"读内容/存 md/答问"。
    final_text: String,
    /// 末页截图路径——供"截图/下载头像"交付。
    final_shot: PathBuf,
    /// **起始页**可读文字（驱动开始前捕获）——finalize 摘取「起始标志」写脚本头（起始前提契约）。
    start_text: String,
    /// 非设备动作（存文件等）的留痕——finalize 时作为 `# 注：…` 注释追加到 .tks 末尾（回放跳过）。
    notes: Vec<String>,
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
            task_mode: $r.task_mode,
            ask_mode: $r.ask_mode,
        }
    };
}

impl TestRun {
    /// 阶段一：朝目标驱动设备 + 失败重探 + 落 .tks/log/元素，再捕获末页文字/截图。返回运行态。
    /// 统一路径：测试与通用任务同一套（同提示词、同产物）。
    /// `goal`=任务目标；`note`=编排官下发的额外约束；
    /// `full_test`=true 走完整测试（开踩实官自动断言+监督官把关，脚本可验证）；false=轻模式
    ///   （通用任务/不要求校验：跳过断言+把关，仍照常产 .tks/元素）。
    /// `read_full`=true 时驱动结束后滚动逐屏收集全部文字（读长内容）；false 只取末页一屏。
    pub(crate) async fn explore(
        opts: &AgentRunOptions,
        ui: &dyn Frontend,
        goal: &str,
        note: Option<&str>,
        full_test: bool,
        read_full: bool,
        ask_mode: AskMode,
    ) -> Result<TestRun> {
        let case = goal; // 目标即用例（统一）
        // 设备/平台优先级：显式覆盖（向导/--platform/-d）> params.device() > 设备推断
        let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
        let platform = opts.platform.unwrap_or_else(|| Platform::from_device(Some(&device)));

        // —— 运行中间文件目录：落到 cache（--cache 或系统临时目录）；与脚本/交付文件分开、不展示给用户 ——
        let stem = slug(case, 30);
        let cache_root = opts.params.cache_root();
        let artifacts = RunArtifacts::create(&cache_root, &stem)?;
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
        let mut sess = LlmSession::new_for_role(&opts.ai, "explorer", system_prompt, tools)?;
        tx.log("model", serde_json::json!({ "model": sess.model() }));
        let case_msg = render(&prompts.message("explorer", "case_intro"), &[("case", case)]);
        tx.log("llm_message", serde_json::json!({ "content": case_msg.clone() }));
        sess.user(case_msg);
        // read_full（读长内容）分工：explorer 只负责**导航到内容页**，到达即 finish；
        // 全文由系统逐屏读取——绝不要为"读完/确认到底"反复滚动（那会到底页面不变、被误判打转）。
        if read_full {
            let nav_only = "【读取长内容模式】你的任务**只是导航到目标内容页并停在那里**——一看到目标内容（如政策正文开头）就**立刻 finish**。全文会由系统自动逐屏读取，你**不要**为了“读完整篇/确认到底”反复向下滚动；到达内容页即视为完成。";
            tx.log("llm_message", serde_json::json!({ "content": nav_only }));
            sess.user(nav_only.to_string());
        }
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
            task_mode: !full_test,
            ask_mode,
            final_text: String::new(),
            final_shot: PathBuf::new(),
            start_text: String::new(),
            notes: Vec::new(),
        };

        // —— 开局净化：关掉可能残留的旧会话（如上次没关的浏览器）——
        if matches!(run.platform, Platform::Web) {
            let _ = super::super::execution::device::exec(&run.device, crate::ControlAction::Close { package: String::new() }).await;
        }

        // 起始页快照：驱动开始前捕获一次（起始前提契约的素材——无「启动」步的脚本回放
        // 必须从这个状态开始）。采集失败不致命（web 冷启动无会话），起始标志就不写。
        run.start_text = match capture(&run.device, &run.workarea, &run.fetcher, opts.ocr.as_ref()).await {
            Ok(p) => p
                .elements
                .iter()
                .filter_map(|e| e.text.as_deref().map(str::trim).filter(|s| !s.is_empty()))
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => String::new(),
        };

        // 顶栏状态：进入初探
        ui.emit(UiEvent::Phase { phase: Phase::Explore, n: None });
        let ctx = drive_ctx!(run, opts, ui);
        let mut outcome = drive(&mut run.sess, &mut run.tx, &ctx, true, "").await?;
        drop(ctx);

        run.sup_pt = outcome.subagent_pt;
        run.sup_ct = outcome.subagent_ct;
        run.explore_created = outcome.created.clone();

        // —— 失败/卡住 → 反思官给「重探指导」，带指导从头重探 ——
        // 仅**测试任务**(full_test)重探：通用任务不从头重跑，失败也直接带着已有结果收尾交付
        // （从头重探是测试为"走出可回放脚本"才需要的；通用任务该一次到位、不浪费）。
        let max_reexplore = if full_test { opts.params.harness.reexplore } else { 0 };
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
            run.sess = LlmSession::new_for_role(&opts.ai, "explorer", run.prompts.role_system("explorer", &run.device, run.platform.name()), build_tools(&run.prompts, run.platform))?;
            let case_msg = render(&run.prompts.message("explorer", "case_intro"), &[("case", run.case.as_str())]);
            run.tx.log("llm_message", serde_json::json!({ "content": case_msg.clone() }));
            run.sess.user(case_msg);
            let guide_msg = format!(
                "【上一轮没到达目标。反思官复盘了它走过的每个页面，给出下面这份**修正后的重探计划**】\n{}\n\n请**照这份计划一步步走**，直接走对的路、别再重蹈覆辙。",
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

        // —— 探索草稿脚本：落 cache（run_dir 下）；最终对外的 .tks 由 finalize 写到工作区 ——
        let base = outcome
            .script_name
            .as_deref()
            .map(|s| slug(s, 50))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| slug(&run.case, 40));
        let script_path = unique_script_path(&run.run_dir, &base);

        // 先落探索原始脚本（草稿）；最终路径/命名在 finalize 决定。
        let final_lines = outcome.lines.clone();
        write_script(&script_path, &run.case, &final_lines)?;
        let fname = script_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        ui.emit(UiEvent::ScriptGenerated { name: fname.clone(), steps: final_lines.len(), success: outcome.success });

        run.result_lines = final_lines.clone();
        run.final_lines = final_lines;
        run.script_path = script_path;
        run.outcome = outcome;

        // 末页交付物捕获：read_full→滚动逐屏收全文（读长内容）；否则只取末页一屏。
        let (text, shot) = if read_full {
            extract_scrolling(&run.device, &run.workarea, &run.fetcher, opts.ocr.as_ref()).await
        } else {
            match capture(&run.device, &run.workarea, &run.fetcher, opts.ocr.as_ref()).await {
                Ok(p) => {
                    let t = p.elements.iter()
                        .filter_map(|e| e.text.as_deref().map(str::trim).filter(|s| !s.is_empty()))
                        .collect::<Vec<_>>().join("\n");
                    (t, p.shot_path)
                }
                Err(_) => (String::new(), run.workarea.screenshot_path()),
            }
        };
        run.final_text = text;
        run.final_shot = shot;
        Ok(run)
    }

    /// 末页可读文字（通用任务交付：存 md / 答问取材）。
    pub(crate) fn final_text(&self) -> &str {
        &self.final_text
    }

    /// 末页截图路径（通用任务交付：截图 / 下载头像）。
    pub(crate) fn final_shot(&self) -> &std::path::Path {
        &self.final_shot
    }

    /// 给编排官看的探索结果摘要（一句话）。
    pub(crate) fn explore_brief(&self) -> String {
        let head = if self.outcome.aborted {
            "被中断"
        } else if self.outcome.success {
            "达成"
        } else {
            "未达成"
        };
        let tail = if self.outcome.success {
            String::new()
        } else {
            format!("（{}）", self.outcome.reason)
        };
        format!(
            "探索{}：{} 轮，脚本草稿 {} 步，路径 {}{}",
            head,
            self.outcome.rounds,
            self.final_lines.len(),
            self.script_path.display(),
            tail
        )
    }

    /// 收尾：把 .tks 落到 `out_tks`（工作区）、引用元素+模板图打包成 `out_tklib`（.tklib 元素包，
    /// 两件套复制即跑），写 run_end / log / conversation，渲染结果框，返回结果。
    /// **总是落盘**——哪怕没完全达成，也留下可被 repair_tks 修的脚本（去掉了原来"失败就删脚本"）。
    pub(crate) async fn finalize(
        mut self,
        opts: &AgentRunOptions,
        ui: &dyn Frontend,
        out_tks: &std::path::Path,
        out_tklib: &std::path::Path,
    ) -> Result<AgentResult> {
        // 输出目标改成调用方给的工作区路径 + .tklib 元素包
        self.script_path = out_tks.to_path_buf();
        // 最终成败：探索达成 且（未自检 或 自检稳定通过）
        let stable = self.verified.as_ref().map(|r| r.passed);
        let overall_success = self.outcome.success && stable.unwrap_or(true);

        // 目标标志：探索成功且是完整测试时，从**实际末页文字**里摘取（机械校验必须是原文片段）——
        // 之后写进脚本头，replay/repair 直接复用真实基线，不再凭 goal 瞎猜（真机教训：猜出的
        // 标志指向中途页面，错误基线诱导医生把正确的收尾步骤当"多余流程"删掉）。
        let mut goal_marker = String::new();
        let mut start_marker = String::new();
        if overall_success && !self.task_mode {
            let (m, mpt, mct) = super::verify::derive_marker_from_page(&opts.ai, &self.prompts, &mut self.tx, &self.case, "goal_marker_from_page", &self.final_text).await;
            self.sup_pt += mpt;
            self.sup_ct += mct;
            goal_marker = m;
            // 起始前提：只有**无「启动」步**的脚本需要（有启动步的由重启净化保证起点确定）
            let first_is_launch = self.result_lines.first().map(|l| super::fmt::is_launch_line(l)).unwrap_or(false);
            if !first_is_launch && !self.start_text.trim().is_empty() {
                let (sm, spt2, sct2) = super::verify::derive_marker_from_page(&opts.ai, &self.prompts, &mut self.tx, &self.case, "start_marker_from_page", &self.start_text).await;
                self.sup_pt += spt2;
                self.sup_ct += sct2;
                start_marker = sm;
            }
        }

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
        // 防污染（临时库架构）：只把**终稿引用到的元素**从临时库提交出去，孤儿元素随临时库丢弃。
        let referenced = referenced_element_names(&self.result_lines);
        let feat_names: Vec<String> = all_created.iter().filter(|n| referenced.contains(n.as_str())).cloned().collect();
        // 成功才做语义命名，未达成就用特征名原样落（留给 repair_tks 修）。
        let (renamed, semantic) = if overall_success {
            reflect::finalize_names(&opts.ai, &self.prompts, &mut self.tx, ui, &self.element_path, &self.result_lines, &feat_names).await
        } else {
            (self.result_lines.clone(), feat_names.clone())
        };
        self.result_lines = renamed;
        let _ = write_script(&self.script_path, &self.case, &self.result_lines);
        if !goal_marker.is_empty() && super::tksops::write_marker(&self.script_path, &goal_marker).is_ok() {
            ui.emit(UiEvent::Notice {
                level: Level::Dim,
                text: format!("目标标志（取自实际末页）已写入脚本头：{}", super::fmt::brief(&goal_marker, 40)),
            });
        }
        if !start_marker.is_empty() && super::tksops::write_start_marker(&self.script_path, &start_marker).is_ok() {
            ui.emit(UiEvent::Notice {
                level: Level::Dim,
                text: format!("起始标志（起始前提，取自开跑前页面）已写入脚本头：{}", super::fmt::brief(&start_marker, 40)),
            });
        }
        // 引用元素 → staging 库 → 打包 .tklib（两件套：foo.tks + foo.tklib，复制即跑；无共享库）
        let stage_json = self.run_dir.join("tklib-stage").join("element.json");
        let committed = crate::tools::element::commit_elements(&self.element_path, &stage_json, &semantic).unwrap_or(0);
        let meta = crate::utils::tklib::TklibMeta::new(self.platform.name(), &self.device);
        if !stage_json.is_file() {
            // 一个元素都没有（如纯坐标/等待脚本）也要产出合法的空包，保证两件套完整
            let _ = std::fs::create_dir_all(stage_json.parent().unwrap_or(&self.run_dir));
            let _ = std::fs::write(&stage_json, "{\"elements\":{}}");
        }
        if let Err(e) = crate::utils::tklib::pack(&stage_json, out_tklib, &meta) {
            ui.emit(UiEvent::Notice {
                level: Level::Warn,
                text: format!("元素包打包失败（{}）：脚本已保存，但 {} 缺失——回放将只剩坐标步可用", e, out_tklib.display()),
            });
        }
        // 非设备动作留痕：把 notes 作为 `# 注：…` 注释追加到 .tks 末尾（回放跳过；脚本已写完才追加，不被覆盖）。
        if !self.notes.is_empty() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&self.script_path) {
                for note in &self.notes {
                    let _ = writeln!(f, "# 注：{}", note);
                }
            }
        }
        let _ = committed; // 元素随 .tklib 两件套走，不再单列
        render_summary(
            self.outcome.success,
            self.outcome.aborted,
            &self.outcome.reason,
            self.outcome.rounds,
            self.verified.as_ref(),
            &model,
            total_prompt,
            total_completion,
            &self.script_path,
            out_tklib,
            &self.result_lines,
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

/// 多屏提取：从当前页起**向下滚动逐屏收集可读文字**，到底（与上屏文字相同）或达上限即停。
/// 按出现顺序去重拼接；返回 (全文, 最后一屏截图)。中断(Ctrl+C)则提前结束。
async fn extract_scrolling(
    device: &str,
    workarea: &Workarea,
    fetcher: &Fetcher,
    ocr: Option<&crate::engines::ocr::OcrSource>,
) -> (String, PathBuf) {
    const MAX_SCROLLS: usize = 12;
    let mut ordered: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut last_sig: Option<u64> = None;
    let mut last_shot = workarea.screenshot_path();

    for _ in 0..MAX_SCROLLS {
        if super::interrupt::aborted() {
            break;
        }
        let p = match capture(device, workarea, fetcher, ocr).await {
            Ok(p) => p,
            Err(_) => break,
        };
        last_shot = p.shot_path.clone();
        let mut sig_src = String::new();
        let (mut w, mut h) = (1080i32, 1920i32);
        for e in &p.elements {
            w = w.max(e.bounds.x2);
            h = h.max(e.bounds.y2);
            if let Some(t) = e.text.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                sig_src.push_str(t);
                sig_src.push('|');
                if seen.insert(t.to_string()) {
                    ordered.push(t.to_string());
                }
            }
        }
        // 本屏结构与上屏相同 → 到底，停。
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        sig_src.hash(&mut hasher);
        let sig = hasher.finish();
        if Some(sig) == last_sig {
            break;
        }
        last_sig = Some(sig);
        // 向下滚一屏（内容上移=swipe up）
        let from = Point { x: w / 2, y: (h as f32 * 0.65) as i32 };
        let _ = super::super::execution::device::exec(
            device,
            ControlAction::SwipeDir { from, direction: "up".to_string(), distance: (h as f32 * 0.5) as i32, duration_ms: 400 },
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    }

    (ordered.join("\n"), last_shot)
}
