// 【doctor 修复流程无设备测试】链路最深的一条：doctor_repair → diagnose → ScriptRunner
// 整脚本回放 → tks 解释器 → 元素定位(真实 recognizer + 元素库) → FakeDriver。
// 脚本与元素库由一轮 FakeLlm 探索**照生产路径产出**（dogfood，不手写库格式），
// 再人为注入坏步骤，验证医生「诊断→删坏步→run→复诊达标」的完整修复闭环。

use std::sync::Arc;

use crate::drivers::fake;
use crate::workflow::agent::provider::{enqueue_fake_role_session, FakeTurn};
use crate::workflow::agent::prompt::{PromptSet, PromptSpec};
use crate::workflow::agent::transcript::Transcript;
use crate::workflow::agent::ui::PlainFrontend;
use crate::{Fetcher, LlmSession, Params, RunArtifacts, Workarea};

use super::ctx::DriveCtx;
use super::doctor;
use super::flow::drive;
use super::options::VerifyReport;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("tke-doctor-test-{}-{}", std::process::id(), name));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// 医生修复闭环：探索产出「启动+点击」正确脚本 → 注入一步点不存在的元素 →
/// 诊断回放在坏步失败 → 医生 delete_lines 删掉坏步 → run 复诊 → 到达目标标志 → 返回修好的脚本。
#[tokio::test]
async fn doctor_deletes_bad_step_and_reaches_marker() {
    let device = "fake:doctor-fix";
    let scope = "doctor-fix-test";
    // 页面脚本：launch 回第 0 页(P1 有「进入设置」)；tap 进 P2(有「设置中心」= 目标标志)
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let tmp = temp_dir("fix");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig {
        provider: Some("fake".into()),
        model: Some(scope.into()),
        ..Default::default()
    };
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "doctor-fix").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    // —— 第一阶段：用一轮探索照生产路径产出脚本 + 元素库（启动 → 点击 → finish）——
    let mut sess = LlmSession::new_fake(
        "你是测试探索官",
        Vec::new(),
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开应用" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了" })),
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
        case: "打开设置中心",
        ai: &ai,
        ui: &ui,
        task_mode: true,
    };
    let explored = drive(&mut sess, &mut tx, &ctx, false, "").await.unwrap();
    assert!(explored.success);
    assert_eq!(explored.lines.len(), 2, "启动 + 点击：{:?}", explored.lines);

    // —— 注入坏步骤：点一个元素库里根本没有的元素（回放时定位必然失败）——
    let mut broken = explored.lines.clone();
    broken.push("点击 [{幽灵按钮}]".to_string());

    // —— 第二阶段：医生修复。医生脚本会话：删第 3 行（坏步）→ run 复诊 ——
    enqueue_fake_role_session(
        scope,
        "doctor",
        vec![
            FakeTurn::tool("delete_lines", serde_json::json!({ "from": 3, "to": 3, "reason": "第 3 步元素不存在，回放在此失败" })),
            FakeTurn::tool("run", serde_json::json!({})),
        ],
    );

    let params = Arc::new(
        Params::resolve(
            Some(device.to_string()),
            None,
            None,
            Some(tmp.clone()),
            None,
            false,
            crate::utils::config::TkeConfig::default(),
        )
        .with_element_lib(element_path.clone()),
    );
    let mut report = VerifyReport { ran: true, ..Default::default() };

    let fixed = doctor::doctor_repair(
        &ai,
        &prompts,
        &mut tx,
        &ctx,
        &params,
        "打开设置中心",
        "设置中心", // 目标标志：P2 专属文字
        broken,
        &mut report,
    )
    .await;

    let fixed = fixed.expect("医生应把脚本修到达标");
    assert_eq!(fixed.len(), 2, "坏步被删，剩 启动+点击：{:?}", fixed);
    assert!(fixed[0].starts_with("启动"));
    assert!(fixed[1].starts_with("点击") && fixed[1].contains("进入设置"));

    // 设备事件核对：探索 1 次 tap + 两轮诊断回放各 1 次 tap = 3 次；
    // 每轮诊断前 reset_state(关闭+启动) + 脚本内启动步，launch 事件应 ≥ 4 次
    let evs = fake::events(device);
    let taps = evs.iter().filter(|e| e.starts_with("tap")).count();
    let launches = evs.iter().filter(|e| e.starts_with("launch")).count();
    assert_eq!(taps, 3, "探索1 + 诊断2 次点击：{:?}", evs);
    assert!(launches >= 4, "reset_state 与脚本启动步应多次 launch：{:?}", evs);

    fake::remove(device);
}

/// 医生停滞止损：脚本坏、医生只空转（不做任何有效编辑就 run）→ 连续两轮无改动判放弃，返回 None。
#[tokio::test]
async fn doctor_gives_up_after_stagnation() {
    let device = "fake:doctor-stall";
    let scope = "doctor-stall-test";
    fake::install(device, vec![fake::page(&[&fake::node("首页", 10, 10, 110, 60)])]);

    let tmp = temp_dir("stall");
    let prompts = PromptSet::resolve(&PromptSpec::default()).unwrap();
    let ai = crate::utils::AiConfig {
        provider: Some("fake".into()),
        model: Some(scope.into()),
        ..Default::default()
    };
    let ui = PlainFrontend::new();
    let workarea = Workarea::for_device(Some(device)).unwrap();
    let fetcher = Fetcher::new();
    let artifacts = RunArtifacts::create(&tmp, "doctor-stall").unwrap();
    let element_path = tmp.join("element.json");
    let mut tx = Transcript::create(tmp.join("conversation.jsonl")).unwrap();

    let ctx = DriveCtx {
        device,
        element_path: &element_path,
        workarea: &workarea,
        fetcher: &fetcher,
        artifacts: &artifacts,
        ocr: None,
        max_rounds: 5,
        prompts: &prompts,
        case: "到一个不存在的页面",
        ai: &ai,
        ui: &ui,
        task_mode: true,
    };

    // 医生连续两轮都只 run（无任何编辑）→ 停滞判定应放弃
    enqueue_fake_role_session(scope, "doctor", vec![
        FakeTurn::tool("run", serde_json::json!({})),
        FakeTurn::tool("run", serde_json::json!({})),
        FakeTurn::tool("run", serde_json::json!({})),
    ]);

    let params = Arc::new(
        Params::resolve(
            Some(device.to_string()),
            None,
            None,
            Some(tmp.clone()),
            None,
            false,
            crate::utils::config::TkeConfig::default(),
        )
        .with_element_lib(element_path.clone()),
    );
    let mut report = VerifyReport { ran: true, ..Default::default() };

    // 脚本只有一步「等待」——能跑通但目标标志「乌托邦」永远不出现
    let fixed = doctor::doctor_repair(
        &ai,
        &prompts,
        &mut tx,
        &ctx,
        &params,
        "到一个不存在的页面",
        "乌托邦",
        vec!["等待 [500ms]".to_string()],
        &mut report,
    )
    .await;

    assert!(fixed.is_none(), "医生空转两轮应放弃修复");

    fake::remove(device);
}
