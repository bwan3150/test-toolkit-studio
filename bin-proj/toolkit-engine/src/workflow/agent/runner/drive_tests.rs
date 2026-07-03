// 【drive() 无设备集成测试】FakeLlm(脚本化回复) + FakeDriver(脚本化页面/事件记录)
// 跑**完整的探索驱动循环**——感知解析、AI 决策、动作执行、.tks 落行、finish 收束全走真实路径
// （仿 Maestro 的 IntegrationTest：真 Maestro + FakeDriver，不 mock 自家逻辑）。
// task_mode=true 跳过踩实官/监督官（它们各起独立 LLM 会话，属于另一层的测试对象）。

use crate::drivers::fake;
use crate::workflow::agent::provider::FakeTurn;
use crate::workflow::agent::prompt::{PromptSet, PromptSpec};
use crate::workflow::agent::transcript::Transcript;
use crate::workflow::agent::ui::PlainFrontend;
use crate::{Fetcher, LlmSession, RunArtifacts, Workarea};

use super::ctx::DriveCtx;
use super::flow::drive;

/// 每个测试独立的临时目录（进程号 + 名字隔离，避免并行测试互踩）
fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("tke-drive-test-{}-{}", std::process::id(), name));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// 基础闭环：AI 点击 → 页面切换 → AI finish。断言脚本行、事件序列、结果状态。
#[tokio::test]
async fn drive_click_then_finish() {
    let device = "fake:drive-basic";
    // 两页脚本：第 1 页有「进入设置」按钮；点击(tap 推进)后到第 2 页「设置中心」
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let tmp = temp_dir("basic");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig::default();
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "drive-basic").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    // 脚本化的 AI：第 1 轮点元素 0；第 2 轮 finish；最后一轮是 desc 生成（回非 JSON 走告警路径）
    let mut sess = LlmSession::new_fake(
        "你是测试探索官",
        Vec::new(),
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "点进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "已到设置中心", "script_name": "test-flow" })),
            FakeTurn::text("{}"),
        ],
    );

    let ctx = DriveCtx {
        device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: None,
        max_rounds: 5,
        prompts: &prompts,
        case: "打开设置页",
        ai: &ai,
        ui: &ui,
        task_mode: true, // 跳过踩实官/监督官（它们各起独立 LLM 会话）
    };

    let outcome = drive(&mut sess, &mut tx, &ctx, false, "").await.unwrap();

    // 结果状态
    assert!(outcome.success, "finish(success=true) 应收束为成功：{}", outcome.reason);
    assert_eq!(outcome.script_name.as_deref(), Some("test-flow"));
    assert_eq!(outcome.rounds, 2, "第 1 轮点击、第 2 轮 finish");
    assert!(!outcome.aborted);

    // .tks 脚本行：一步点击，引用「进入设置」元素
    assert_eq!(outcome.lines.len(), 1, "应只落一行点击：{:?}", outcome.lines);
    assert!(outcome.lines[0].starts_with("点击"), "实际：{}", outcome.lines[0]);
    assert!(outcome.lines[0].contains("进入设置"), "实际：{}", outcome.lines[0]);
    assert_eq!(outcome.step_comments[0], "点进设置");

    // 设备事件：恰好一次 tap，落在按钮中心 (200, 230)
    let evs = fake::events(device);
    assert_eq!(evs, vec!["tap 200,230".to_string()], "实际事件：{:?}", evs);

    // 新建元素已落临时库（点击即收录）
    assert_eq!(outcome.created.len(), 1, "点击的按钮应被收录：{:?}", outcome.created);

    fake::remove(device);
}

/// 卡死止损：AI 反复点同一个不起作用的元素（页面从不变化）→ 自动停止、判失败。
#[tokio::test]
async fn drive_autostops_on_repeated_noop_click() {
    let device = "fake:drive-stuck";
    // 单页脚本：tap 推进被钳在最后一页 → 页面永远不变
    fake::install(device, vec![fake::page(&[&fake::node("死按钮", 10, 10, 110, 60)])]);

    let tmp = temp_dir("stuck");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig::default();
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "drive-stuck").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    // AI 一直点同一个元素（比止损阈值多备几轮脚本）
    let clicks = (0..8)
        .map(|_| FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "再点一次" })))
        .collect();
    let mut sess = LlmSession::new_fake("你是测试探索官", Vec::new(), clicks);

    let ctx = DriveCtx {
        device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: None,
        max_rounds: 10,
        prompts: &prompts,
        case: "点一个没反应的按钮",
        ai: &ai,
        ui: &ui,
        task_mode: true,
    };

    let outcome = drive(&mut sess, &mut tx, &ctx, false, "").await.unwrap();

    assert!(!outcome.success, "反复无效点击应止损失败，reason={}", outcome.reason);
    assert!(
        outcome.reason.contains("自动停止") || outcome.reason.contains("无前进"),
        "止损原因应可读：{}",
        outcome.reason
    );
    assert!(outcome.rounds < 10, "应在打满轮数前止损（实际 {} 轮）", outcome.rounds);

    fake::remove(device);
}
