// 【operate】通用设备任务：朝任意目标驱动设备——**不产 .tks、不 verify、不提交元素库**。
//
// 复用同一个驱动循环 flow::drive（看页面→动作→再看），但走 task_mode：跳过测试专属的
// 踩实官(自动断言)与监督官(finish 把关)。干完捕获**末页可读文字 + 截图路径**返回，
// 供编排官交付给用户（答问 / 存 md / 给截图）。测试任务仍走 TestRun(explore/verify/finalize)。

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::models::Platform;
use crate::{ControlAction, Fetcher, LlmSession, Point, Result, RunArtifacts, Workarea};

use super::super::perception::capture;
use super::super::prompt::PromptSet;
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
/// `read_full=true`：任务是"读长内容"（如整页 policy）——驱动结束后**滚动逐屏收集全部文字**；
/// `false`：只取末页一屏文字 + 截图（如"截个图"，不要乱滚走开）。
pub(crate) async fn operate(opts: &AgentRunOptions, ui: &dyn Frontend, goal: &str, read_full: bool) -> Result<OperateResult> {
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

    // 驱动会话：专用 operator 系统提示词（通用设备操作，非测试）。
    let system_prompt = prompts.role_system("operator", &device, platform.name());
    let tools = build_tools(&prompts, platform);
    let mut sess = LlmSession::new(&opts.ai, system_prompt, tools)?;
    sess.user(format!(
        "你的任务目标：\n{}\n\n请朝它在设备上操作，完成后用 finish 说清你做了什么、看到的关键信息/结论。",
        goal
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

    // 文字/截图捕获：read_full → 滚动逐屏收集全部文字（长内容）；否则只取末页一屏（截图类任务别乱滚）。
    let (final_text, screenshot) = if read_full {
        extract_scrolling(&device, &workarea, &fetcher, opts.ocr.as_ref()).await
    } else {
        match capture(&device, &workarea, &fetcher, opts.ocr.as_ref()).await {
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
        }
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
        // 本屏文字（按出现顺序去重收集）+ 算屏幕尺寸（元素最大边界）
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
        // 本屏结构签名与上屏相同 → 已到底，停。
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
