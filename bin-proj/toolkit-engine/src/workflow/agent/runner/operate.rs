// 【operate】通用设备任务：朝任意目标驱动设备——**不产 .tks、不 verify、不提交元素库**。
//
// 复用同一个驱动循环 flow::drive（看页面→动作→再看），但走 task_mode：跳过测试专属的
// 踩实官(自动断言)与监督官(finish 把关)。干完捕获**末页可读文字 + 截图路径**返回，
// 供编排官交付给用户（答问 / 存 md / 给截图）。测试任务仍走 TestRun(explore/verify/finalize)。

use std::path::PathBuf;

use crate::models::Platform;
use crate::{Fetcher, LlmSession, Result, RunArtifacts, Workarea};

use super::super::perception::capture;
use super::super::prompt::{render, PromptSet};
use super::super::tools::build_tools;
use super::super::transcript::Transcript;
use super::super::ui::{Frontend, Phase, UiEvent};
use super::flow::{drive, DriveCtx};
use super::options::AgentRunOptions;
use super::slug;

/// 一次通用任务的结果（给编排官交付用）。
pub(crate) struct OperateResult {
    pub success: bool,
    pub summary: String,     // 驱动 agent 的 finish 依据（做了什么 / 结论）
    pub final_text: String,  // 末页可读文字（拼接元素 text），供"存 md / 答问"取材
    pub screenshot: PathBuf, // 末页截图路径，供"给我看截图 / 下载头像"交付
    pub rounds: usize,
}

/// 朝 `goal` 驱动设备干一件通用任务。
pub(crate) async fn operate(opts: &AgentRunOptions, ui: &dyn Frontend, goal: &str) -> Result<OperateResult> {
    let device = opts.device.clone().or_else(|| opts.params.device()).unwrap_or_default();
    let platform = opts.platform.unwrap_or_else(|| Platform::from_device(Some(&device)));
    std::fs::create_dir_all(&opts.script_dir).ok();
    let prompts = PromptSet::resolve(&opts.prompt)?;

    // 产物目录（隔离，跑完保留供复盘；不提交正式库）
    let stem = slug(goal, 30);
    let log_root = opts.params.log.clone().unwrap_or_else(|| opts.script_dir.clone());
    let artifacts = RunArtifacts::create(&log_root, &stem)?;
    let run_dir = artifacts.run_dir.clone();
    let element_path = run_dir.join("element.json");
    let mut tx = Transcript::create(run_dir.join("conversation.jsonl"))?;
    tx.log(
        "operate_goal",
        serde_json::json!({ "goal": goal, "device": device.clone(), "platform": platform.name() }),
    );

    // 驱动会话：复用 explorer（它会朝目标导航/操作）；前导讲清"这是一般任务不是测试"。
    let system_prompt = prompts.role_system("explorer", &device, platform.name());
    let tools = build_tools(&prompts, platform);
    let mut sess = LlmSession::new(&opts.ai, system_prompt, tools)?;
    let case_msg = render(&prompts.message("explorer", "case_intro"), &[("case", goal)]);
    sess.user(format!(
        "【这是一个一般设备操作任务，不是测试用例】请朝下面这个目标在设备上操作，完成后用 finish 简述结果/你看到的关键信息：\n{}",
        case_msg
    ));

    let workarea = Workarea::for_device(Some(&device))?;
    let fetcher = Fetcher::new();
    let max_rounds = opts.ai.max_rounds.unwrap_or(40) as usize;
    // web 开局净化（同测试路径）
    if matches!(platform, Platform::Web) {
        let _ = super::super::execution::device::exec(&device, crate::ControlAction::Close { package: String::new() }).await;
    }

    ui.emit(UiEvent::Phase { phase: Phase::Explore, n: None });
    let ctx = DriveCtx {
        device: &device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: opts.ocr.as_ref(),
        max_rounds,
        prompts: &prompts,
        case: goal,
        ai: &opts.ai,
        ui,
        task_mode: true,
    };
    let outcome = drive(&mut sess, &mut tx, &ctx, true, "").await?;
    drop(ctx);

    // 末页捕获：拼接可读文字（供存 md / 答问）+ 截图路径（供交付截图）。
    let (final_text, screenshot) = match capture(&device, &workarea, &fetcher, opts.ocr.as_ref()).await {
        Ok(p) => {
            let text = p
                .elements
                .iter()
                .filter_map(|e| e.text.as_deref().map(str::trim).filter(|s| !s.is_empty()))
                .collect::<Vec<_>>()
                .join("\n");
            (text, p.shot_path)
        }
        Err(_) => (String::new(), workarea.screenshot_path()),
    };

    let _ = tx.finalize(&run_dir.join("conversation.json"));
    Ok(OperateResult {
        success: outcome.success,
        summary: outcome.reason,
        final_text,
        screenshot,
        rounds: outcome.rounds,
    })
}
