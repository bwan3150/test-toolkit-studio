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

/// 把关时序（full_test）：踩实官每次导航后插断言；监督官第一次 finish 打回、
/// 探索被迫继续、第二次 finish 放行。asserter/supervisor 在深层自建会话，
/// 经 provider="fake" + enqueue_fake_role_session 按角色注入脚本。
#[tokio::test]
async fn full_test_asserter_inserts_and_supervisor_gates() {
    let device = "fake:drive-gate";
    let scope = "gate-test"; // 角色脚本注册表的隔离键（= AiConfig.model）
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160), &fake::node("打开详情", 100, 400, 300, 460)]),
            fake::page(&[&fake::node("详情页", 100, 100, 400, 160)]),
        ],
    );

    let tmp = temp_dir("gate");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig {
        provider: Some("fake".into()),
        model: Some(scope.into()),
        ..Default::default()
    };
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "drive-gate").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    // 探索官（直接注入）：点A → finish(被打回) → 点B → finish(放行) → desc
    let mut sess = LlmSession::new_fake(
        "你是测试探索官",
        Vec::new(),
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到设置中心了" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 1, "comment": "打开详情" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到详情页了", "script_name": "gate-flow" })),
            FakeTurn::text("{}"),
        ],
    );
    // 踩实官：两次导航各挑一个标志元素（返回当前页真实序号）
    crate::workflow::agent::provider::enqueue_fake_role_session(
        scope, "asserter",
        vec![FakeTurn::text(r#"{"index": 0, "reason": "设置中心是该页专属标志"}"#)],
    );
    crate::workflow::agent::provider::enqueue_fake_role_session(
        scope, "asserter",
        vec![FakeTurn::text(r#"{"index": 0, "reason": "详情页是该页专属标志"}"#)],
    );
    // 监督官：第一次打回、第二次放行
    crate::workflow::agent::provider::enqueue_fake_role_session(
        scope, "supervisor",
        vec![FakeTurn::text(r#"{"approved": false, "reason": "还没打开详情，继续"}"#)],
    );
    crate::workflow::agent::provider::enqueue_fake_role_session(
        scope, "supervisor",
        vec![FakeTurn::text(r#"{"approved": true, "reason": "已到详情页"}"#)],
    );

    let ctx = DriveCtx {
        device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: None,
        max_rounds: 8,
        prompts: &prompts,
        case: "打开设置里的详情页",
        ai: &ai,
        ui: &ui,
        task_mode: false, // 完整测试：开踩实官 + 监督官
    };

    let outcome = drive(&mut sess, &mut tx, &ctx, false, "").await.unwrap();

    assert!(outcome.success, "监督官第二次应放行：{}", outcome.reason);
    assert_eq!(outcome.script_name.as_deref(), Some("gate-flow"));
    // 脚本时序：点击A → 自动断言(设置中心) → 点击B → 自动断言(详情页)
    assert_eq!(outcome.lines.len(), 4, "实际脚本：{:?}", outcome.lines);
    assert!(outcome.lines[0].starts_with("点击") && outcome.lines[0].contains("进入设置"));
    assert!(outcome.lines[1].starts_with("断言") && outcome.lines[1].contains("设置中心"), "实际：{}", outcome.lines[1]);
    assert!(outcome.lines[2].starts_with("点击") && outcome.lines[2].contains("打开详情"));
    assert!(outcome.lines[3].starts_with("断言") && outcome.lines[3].contains("详情页"), "实际：{}", outcome.lines[3]);
    // 断言步的理由来自踩实官
    assert!(outcome.step_comments[1].contains("设置中心是该页专属标志"));
    // 子 agent token 入账：踩实官×2 + 监督官×2，每轮 (10,5)
    assert_eq!((outcome.subagent_pt, outcome.subagent_ct), (40, 20));
    // 设备只发生两次 tap（断言不操作设备）
    let taps: Vec<_> = fake::events(device).into_iter().filter(|e| e.starts_with("tap")).collect();
    assert_eq!(taps, vec!["tap 200,230".to_string(), "tap 200,430".to_string()]);

    fake::remove(device);
}

/// 监督官连续打回 3 次 → 判定探索失败收场（防"AI 硬 finish"死循环）。
#[tokio::test]
async fn supervisor_reject_limit_fails_the_run() {
    let device = "fake:drive-reject";
    let scope = "reject-test";
    fake::install(device, vec![fake::page(&[&fake::node("首页", 10, 10, 110, 60)])]);

    let tmp = temp_dir("reject");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig {
        provider: Some("fake".into()),
        model: Some(scope.into()),
        ..Default::default()
    };
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "drive-reject").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    // 探索官什么都不干、硬 finish 四次（第 4 次时打回额度已满，直接判失败）
    let finishes = (0..4)
        .map(|_| FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "我觉得做完了" })))
        .collect();
    let mut sess = LlmSession::new_fake("你是测试探索官", Vec::new(), finishes);
    for _ in 0..3 {
        crate::workflow::agent::provider::enqueue_fake_role_session(
            scope, "supervisor",
            vec![FakeTurn::text(r#"{"approved": false, "reason": "什么都没做"}"#)],
        );
    }

    let ctx = DriveCtx {
        device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: None,
        max_rounds: 5,
        prompts: &prompts,
        case: "完成某个任务",
        ai: &ai,
        ui: &ui,
        task_mode: false,
    };

    let outcome = drive(&mut sess, &mut tx, &ctx, false, "").await.unwrap();

    assert!(!outcome.success, "连续打回 3 次应判失败");
    assert!(outcome.reason.contains("监督官"), "失败原因应指明监督官打回：{}", outcome.reason);
    assert!(outcome.lines.is_empty(), "全程没有设备动作，脚本应为空");
    assert!(fake::events(device).is_empty(), "设备不应有任何动作");

    fake::remove(device);
}
