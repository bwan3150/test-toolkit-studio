// 【断点续探修复 无设备测试】repair_tks 新机制（取代已删除的「医生」文本编辑 agent）：
// 诊断回放停在失败现场 → explorer 从**当前实时页面**接管走完目标 → 前缀+新尾巴 → 复诊达标。
// 全链真实路径：ScriptRunner 回放/元素定位失败/drive 驱动循环/踩实+监督把关/tklib 回包。

use std::sync::Arc;

use crate::drivers::fake;
use crate::workflow::agent::prompt::PromptSpec;
use crate::workflow::agent::provider::{enqueue_fake_role_session, FakeTurn};
use crate::workflow::agent::ui::PlainFrontend;
use crate::{Params, Workarea};

use super::options::AgentRunOptions;
use super::tksops;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("tke-repair-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn opts_for(device: &str, scope: &str, workspace: std::path::PathBuf, cache: std::path::PathBuf) -> AgentRunOptions {
    let ai = crate::utils::AiConfig {
        provider: Some("fake".into()),
        model: Some(scope.into()),
        ..Default::default()
    };
    AgentRunOptions {
        case: "打开设置中心".into(),
        script_dir: workspace.clone(),
        ai,
        prompt: PromptSpec::default(),
        ocr: None,
        verify: false,
        platform: None,
        device: Some(device.to_string()),
        params: Arc::new(Params::resolve(
            Some(device.to_string()),
            None,
            None,
            Some(cache),
            Some(workspace),
            false,
            None,
            None,
            crate::utils::config::TkeConfig::default(),
        )),
        source: None,
    }
}

/// 编排官式修复闭环：replay 拿结构化失败报告（含续探建议）→ resume_explore 从失败现场
/// 续探（保留成功前缀）→ replay 验证达标。脚本写回 + 回包 tklib，marker 头保留。
#[tokio::test]
async fn repair_resumes_from_failure_point() {
    let device = "fake:repair-fix";
    let scope = "repair-fix";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("fix-ws");
    let cache = temp_dir("fix-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    // 坏脚本：启动 → 点幽灵按钮（库里没有，定位必然失败）。marker 已在头（探索产出脚本的常态）
    let tks = ws.join("open-settings.tks");
    std::fs::write(
        &tks,
        "# 用例: 打开设置中心\n# 目标标志: 设置中心\n步骤:\n启动 [\"settings.app\"]\n点击 [{幽灵按钮}]\n",
    )
    .unwrap();

    // 续探的 explorer：点元素 0（进入设置）→ finish；踩实官/监督官照常把关；desc 生成一轮
    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到设置中心了" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "asserter", vec![FakeTurn::tool("report", serde_json::json!({ "index": 0, "reason": "设置中心是该页专属标志" }))]);
    enqueue_fake_role_session(scope, "supervisor", vec![FakeTurn::tool("report", serde_json::json!({ "approved": true, "reason": "已到设置中心" }))]);

    // 编排官式修复流程：replay 拿失败报告 → resume_explore 续探 → replay 验证
    let opts = opts_for(device, scope, ws.clone(), cache);
    let report = tksops::replay_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(report.contains("回放未到达目标"), "实际：{}", report);
    assert!(report.contains("keep_steps: 1"), "失败报告应给出续探建议：{}", report);
    let msg = tksops::resume_explore(&opts, &ui, &tks, "打开设置中心", 1, "第 2 步点击幽灵按钮定位失败").await.unwrap();
    assert!(msg.contains("已续写"), "实际：{}", msg);
    let verify = tksops::replay_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(verify.contains("回放通过"), "续探后应能到达目标：{}", verify);

    // 脚本 = 成功前缀(启动) + 续探新尾巴(点击 + 自动断言)；坏步已消失；marker 头保留
    let content = std::fs::read_to_string(&tks).unwrap();
    assert!(content.contains("# 目标标志: 设置中心"), "marker 头应保留：\n{}", content);
    assert!(!content.contains("幽灵按钮"), "坏步应被替换：\n{}", content);
    assert!(content.contains("启动"), "成功前缀应保留：\n{}", content);
    assert!(content.contains("进入设置"), "续探的点击应写入：\n{}", content);
    // 续探落的新元素随包写回（两件套自包含）
    assert!(crate::utils::tklib::tklib_path(&tks).is_file(), "修复后应生成/更新 .tklib");

    fake::remove(device);
}

/// 续探走不到目标（explorer 判失败）→ **脚本一字不改**——绝不把还能跑一半的脚本改坏。
#[tokio::test]
async fn repair_failure_preserves_original_script() {
    let device = "fake:repair-stall";
    let scope = "repair-stall";
    fake::install(device, vec![fake::page(&[&fake::node("首页", 10, 10, 110, 60)])]);

    let ws = temp_dir("stall-ws");
    let cache = temp_dir("stall-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    let tks = ws.join("case.tks");
    let original = "# 用例: 到一个不存在的页面\n# 目标标志: 乌托邦\n步骤:\n启动 [\"some.app\"]\n点击 [{幽灵按钮}]\n";
    std::fs::write(&tks, original).unwrap();

    // 续探 explorer 直接认输
    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![FakeTurn::tool("finish", serde_json::json!({ "success": false, "reason": "找不到通往目标的路" }))],
    );

    let opts = opts_for(device, scope, ws.clone(), cache);
    let msg = tksops::resume_explore(&opts, &ui, &tks, "到一个不存在的页面", 1, "第 2 步失败").await.unwrap();
    assert!(msg.contains("续探未能走到目标"), "实际：{}", msg);
    assert!(msg.contains("未改动"), "实际：{}", msg);
    assert_eq!(std::fs::read_to_string(&tks).unwrap(), original, "失败时脚本必须原样保留");

    fake::remove(device);
}

/// 起始前提契约：无「启动」步的脚本带 `# 起始标志:`，当前页面不匹配 → 回放**快速失败**
/// 并说清原因，一步都不执行（此前会闭着眼开跑、越跑越乱）。
#[tokio::test]
async fn replay_fails_fast_on_start_precondition_mismatch() {
    let device = "fake:start-mismatch";
    // 当前页面是「错误页」——脚本期望从含「设备列表」的页面开始
    fake::install(device, vec![fake::page(&[&fake::node("错误页", 10, 10, 110, 60)])]);

    let ws = temp_dir("start-ws");
    let cache = temp_dir("start-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    let tks = ws.join("case.tks");
    std::fs::write(
        &tks,
        "# 用例: 删除第一个设备\n# 目标标志: 删除成功\n# 起始标志: 设备列表\n步骤:\n点击 [{某设备@1_2}]\n",
    )
    .unwrap();

    let opts = opts_for(device, "start-mismatch", ws.clone(), cache);
    let msg = tksops::replay_tks(&opts, &ui, &tks, "删除第一个设备").await.unwrap();
    assert!(msg.contains("起始页不符"), "应快速失败并说清起始前提：{}", msg);
    assert!(msg.contains("设备列表"), "应指出期望的起始标志：{}", msg);
    // 一步都不该执行
    assert!(fake::events(device).iter().all(|e| !e.starts_with("tap")), "不应执行任何点击：{:?}", fake::events(device));

    fake::remove(device);
}

/// 定位自愈（Healenium 式）：应用"改版"后按钮换了文字和位置 → 回放定位失败 →
/// healer 基于当前实时页面单次挑选出"其实就是它"→ 当场救活本步 + 修正持久化进 tklib。
#[tokio::test]
async fn replay_heals_relocated_element_in_place() {
    let device = "fake:heal";
    let scope = "heal-test";
    // 版本 A：旧按钮——先照生产路径产出脚本+元素包
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("旧按钮", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("heal-ws");
    let cache = temp_dir("heal-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "点旧按钮" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]);

    let opts = opts_for(device, scope, ws.clone(), cache);
    let run = super::testrun::TestRun::explore(&opts, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false).await.unwrap();
    let tks = ws.join("open.tks");
    let tklib = crate::utils::tklib::tklib_path(&tks);
    let result = run.finalize(&opts, &ui, &tks, &tklib).await.unwrap();
    assert!(result.success);
    tksops::write_marker(&tks, "设置中心").unwrap();

    // 版本 B："改版"：文字变了、位置挪了、**控件类型和页面结构也变了**——
    // 结构通道(text/xpath)全失效，逼出自愈路径（轻微改版由 recognizer 结构容错自己扛，
    // 那不触发 heal——上一版测试就是这样"意外通过"的）
    fake::install(
        device,
        vec![
            fake::page(&[
                "  <node index=\"0\" text=\"顶部横幅\" class=\"android.widget.ImageView\" clickable=\"false\" bounds=\"[0,0][720,120]\"/>",
                "  <node index=\"1\" text=\"新按钮\" class=\"android.widget.TextView\" clickable=\"true\" bounds=\"[100,300][300,360]\"/>",
            ]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );
    // healer 单次挑选：当前页元素 1（新按钮）就是它
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "index": 1, "reason": "同一功能位，文字与控件类型微调" }))]);

    let opts = opts_for(device, scope, ws.clone(), temp_dir("heal-cache2"));
    let msg = tksops::replay_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(msg.contains("回放通过"), "自愈后应到达目标：{}", msg);

    // 点击落在了新位置（新按钮中心 200,330——不是旧的 200,230）
    let taps: Vec<_> = fake::events(device).into_iter().filter(|e| e.starts_with("tap")).collect();
    assert_eq!(taps.last().map(String::as_str), Some("tap 200,330"), "应点到新位置：{:?}", taps);

    // 修正已持久化：重新解包 tklib，条目文字线索已更新为「新按钮」
    let peek = temp_dir("heal-peek");
    let lib_json = crate::utils::tklib::unpack(&tklib, &peek).unwrap();
    let lib = std::fs::read_to_string(&lib_json).unwrap();
    assert!(lib.contains("新按钮"), "自愈修正应持久化进元素包：\n{}", lib);

    fake::remove(device);
}

/// 导航原语：把设备开到目标状态——**不产脚本、不写元素包、不出结果框**,只返回状态+页面摘要。
/// 这是修复/稳定性测试前"复原"的专用能力,不再拿重型 explore 凑合(那会产垃圾脚本)。
#[tokio::test]
async fn navigate_reaches_state_without_artifacts() {
    let device = "fake:navigate";
    let scope = "navigate";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("nav-ws");
    let cache = temp_dir("nav-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "已停在设置中心" })),
            FakeTurn::text("{}"),
        ],
    );

    let opts = opts_for(device, scope, ws.clone(), cache);
    let msg = super::testrun::navigate(&opts, &ui, "进入设置中心页面", None).await.unwrap();
    assert!(msg.contains("已到达目标状态"), "实际：{}", msg);
    assert!(msg.contains("设置中心"), "应带当前页面摘要：{}", msg);
    // 工作区不产任何文件（无 .tks/.tklib——导航不是探索）
    let files: Vec<_> = std::fs::read_dir(&ws).unwrap().filter_map(|e| e.ok()).collect();
    assert!(files.is_empty(), "导航不应产出文件：{:?}", files.iter().map(|f| f.file_name()).collect::<Vec<_>>());

    fake::remove(device);
}

/// 页面实体+断言页面指令(规范化起始/终点校验):探索自动落"起始页/完成页"进 .tklib、
/// 首尾插「断言页面」步;回放起点不对→首步页面断言失败(信息带命中率);
/// 复位后回放全通(终点判据=尾部页面断言真实执行,不再靠头注释 marker)。
#[tokio::test]
async fn page_assertions_guard_start_and_end() {
    let device = "fake:page-assert";
    let scope = "page-assert";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("page-ws");
    let cache = temp_dir("page-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    // 探索(无启动步,click 起步)→ finalize 应:pages 进包 + 首插起始页断言 + 尾插完成页断言
    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到设置中心" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]);

    let opts = opts_for(device, scope, ws.clone(), cache);
    let run = super::testrun::TestRun::explore(&opts, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false)
        .await
        .unwrap();
    let tks = ws.join("page-case.tks");
    let tklib = crate::utils::tklib::tklib_path(&tks);
    run.finalize(&opts, &ui, &tks, &tklib).await.unwrap();

    let content = std::fs::read_to_string(&tks).unwrap();
    assert!(content.contains("断言页面 [\"起始页\"]"), "应首插起始页断言：\n{}", content);
    assert!(content.contains("断言页面 [\"完成页\"]"), "应尾插完成页断言：\n{}", content);

    // 探索后设备停在 P1(完成页)——回放首步「断言页面 起始页」应失败,信息带页面级细节
    let msg = tksops::replay_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(msg.contains("页面断言失败") && msg.contains("起始页"), "起点不对应被页面断言拦住：{}", msg);

    // 复位到起始页(P0)→ 回放全通(终点判据=尾部完成页断言真实执行)
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );
    let msg = tksops::replay_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(msg.contains("回放通过"), "复位后应全通：{}", msg);

    fake::remove(device);
}

/// 【AI 辅助驾驶 · tke run 路径】纯回放装配 healer 工厂：App"改版"后元素原定位失效 →
/// AI 依当前页面找回、救活当次执行并继续跑完；报告（StepResult.healed / run_end 汇总）标注
/// 自愈；**原 .tks / .tklib 一个字节不动**（修正只落解包出的临时副本——与 harness 的
/// replay_tks 回包行为相反，这是两条路径的语义分界）。
#[tokio::test]
async fn run_copilot_heals_without_touching_assets() {
    let device = "fake:copilot";
    let scope = "copilot-test";
    // 版本 A：照生产路径产出 foo.tks + foo.tklib 两件套
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("旧按钮", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("copilot-ws");
    let cache = temp_dir("copilot-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "点旧按钮" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]);

    let opts = opts_for(device, scope, ws.clone(), cache);
    let run = super::testrun::TestRun::explore(&opts, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false).await.unwrap();
    let tks = ws.join("open.tks");
    let tklib = crate::utils::tklib::tklib_path(&tks);
    let result = run.finalize(&opts, &ui, &tks, &tklib).await.unwrap();
    assert!(result.success);

    // 版本 B："改版"——结构通道全失效，逼出自愈路径（同 replay_heals 测试的改版形状）
    fake::install(
        device,
        vec![
            fake::page(&[
                "  <node index=\"0\" text=\"顶部横幅\" class=\"android.widget.ImageView\" clickable=\"false\" bounds=\"[0,0][720,120]\"/>",
                "  <node index=\"1\" text=\"新按钮\" class=\"android.widget.TextView\" clickable=\"true\" bounds=\"[100,300][300,360]\"/>",
            ]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "index": 1, "reason": "同一功能位，文字与控件类型微调" }))]);

    // tke run 的装配方式（同 cli/workflow/run.rs::healer_factory）：copilot 开 → 工厂延迟构造
    let mut p = (*opts.params).clone();
    p.ai = opts.ai.clone(); // 测试里 fake ai 挂在 opts.ai，生产路径本来就在 params.ai
    let params = Arc::new(p);
    let p2 = params.clone();
    let runner = crate::workflow::script_runner::ScriptRunner::new(params)
        .with_healer_factory(Arc::new(move |lib_json, script_text: &str| super::healer::copilot_healer(&p2, lib_json, script_text)));

    let tklib_before = std::fs::read(&tklib).unwrap();
    let tks_before = std::fs::read(&tks).unwrap();
    let mut events: Vec<String> = Vec::new();
    let exec = runner
        .run(&tks, None, &mut |e| {
            if let Ok(j) = serde_json::to_string(e) {
                events.push(j);
            }
        })
        .await
        .unwrap();

    // 跑通 + 报告标注自愈
    assert!(exec.success, "自愈后应跑通：{:?}", exec.error);
    let healed_steps: Vec<_> = exec.steps.iter().filter(|s| s.healed.is_some()).collect();
    assert_eq!(healed_steps.len(), 1, "应有且只有一步被自愈：{:?}", exec.steps);
    // run_end 事件带自愈汇总（NDJSON 消费方/终端结尾报告都吃这个）
    let run_end = events.iter().find(|e| e.contains("\"run_end\"")).expect("应有 run_end 事件");
    assert!(run_end.contains("\"healed\""), "run_end 应带自愈汇总：{}", run_end);
    // 点击落在了新位置（新按钮中心 200,330）
    let taps: Vec<_> = fake::events(device).into_iter().filter(|e| e.starts_with("tap")).collect();
    assert!(taps.iter().any(|t| t == "tap 200,330"), "应点到新位置：{:?}", taps);

    // 资产零改动：tke run 的自愈只救活当次执行，不回写脚本/元素包
    assert_eq!(std::fs::read(&tks).unwrap(), tks_before, "tke run 不得改 .tks");
    assert_eq!(std::fs::read(&tklib).unwrap(), tklib_before, "tke run 的自愈不得回写 .tklib");

    fake::remove(device);
}

/// 【分诊 · 层2 同页替代】改版后原元素没有对应物（pick 层 null），但同页有**功能等价**的
/// 替代入口 → triage 判 replace → 点替代元素救活当次执行；记账标注"→替代"；**不落库**
/// （替代元素不是"它"，写进原元素名会污染元素库——tklib 字节级不变照旧成立）。
#[tokio::test]
async fn run_copilot_triage_replaces_with_equivalent_element() {
    let device = "fake:triage-replace";
    let scope = "triage-replace";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("旧按钮", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("triage-replace-ws");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "点旧按钮" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]);

    let opts = opts_for(device, scope, ws.clone(), temp_dir("triage-replace-cache"));
    let run = super::testrun::TestRun::explore(&opts, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false).await.unwrap();
    let tks = ws.join("open.tks");
    let tklib = crate::utils::tklib::tklib_path(&tks);
    assert!(run.finalize(&opts, &ui, &tks, &tklib).await.unwrap().success);

    // 版本 B："旧按钮"彻底没了，同页有功能等价的"进入新设置"入口。
    // 注意 class 必须≠库条目的（fake::node 全是 Button）——否则 recognizer 的 class_name
    // 结构容错会"意外命中"，heal 根本不触发（原 heal 测试踩过同一个坑）
    fake::install(
        device,
        vec![
            fake::page(&[
                "  <node index=\"0\" text=\"顶部横幅\" class=\"android.widget.ImageView\" clickable=\"false\" bounds=\"[0,0][720,120]\"/>",
                "  <node index=\"1\" text=\"进入新设置\" class=\"android.widget.TextView\" clickable=\"true\" bounds=\"[100,400][300,460]\"/>",
            ]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );
    // 第一段 pick：没有"就是它"的对应物 → null；第二段 triage：同页替代
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "index": null, "reason": "页面上没有对应物" }))]);
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "verdict": "replace", "index": 1, "diagnosis": "入口疑改名为「进入新设置」，功能等价" }))]);

    let mut p = (*opts.params).clone();
    p.ai = opts.ai.clone();
    let params = Arc::new(p);
    let p2 = params.clone();
    let runner = crate::workflow::script_runner::ScriptRunner::new(params)
        .with_healer_factory(Arc::new(move |lib_json, script_text: &str| super::healer::copilot_healer(&p2, lib_json, script_text)));

    let tklib_before = std::fs::read(&tklib).unwrap();
    let exec = runner.run(&tks, None, &mut |_| {}).await.unwrap();

    assert!(exec.success, "替代救活后应跑通：{:?}", exec.error);
    let healed: Vec<_> = exec.steps.iter().filter_map(|s| s.healed.clone()).collect();
    assert_eq!(healed.len(), 1, "应有一步替代记账：{:?}", exec.steps);
    assert!(healed[0].contains("→替代"), "记账应标注替代路径：{}", healed[0]);
    // 点击落在替代元素中心（200,430）
    let taps: Vec<_> = fake::events(device).into_iter().filter(|e| e.starts_with("tap")).collect();
    assert!(taps.iter().any(|t| t == "tap 200,430"), "应点到替代元素：{:?}", taps);
    // 替代不落库：tklib（乃至解包副本的来源）字节级不变
    assert_eq!(std::fs::read(&tklib).unwrap(), tklib_before, "替代救活不得回写 .tklib");

    fake::remove(device);
}

/// 【分诊 · 层3 前面走偏】回放一开始就落在无关页面（前步没生效的典型现场）→ pick null →
/// triage 判 wrong_page → **不救**，该步失败，报错里带 AI 分诊结论（果不是因，指向前面步骤）。
#[tokio::test]
async fn run_copilot_triage_diagnoses_wrong_page() {
    let device = "fake:triage-wrongpage";
    let scope = "triage-wrongpage";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("旧按钮", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let ws = temp_dir("triage-wp-ws");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "点旧按钮" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了" })),
            FakeTurn::text("{}"),
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]);

    let opts = opts_for(device, scope, ws.clone(), temp_dir("triage-wp-cache"));
    let run = super::testrun::TestRun::explore(&opts, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false).await.unwrap();
    let tks = ws.join("open.tks");
    let tklib = crate::utils::tklib::tklib_path(&tks);
    assert!(run.finalize(&opts, &ui, &tks, &tklib).await.unwrap().success);

    // 版本 B：启动后落在完全无关的页面（前步走偏/没生效的现场）。
    // 手写非 Button class：防 recognizer class_name 结构容错"意外命中"同 class 节点
    fake::install(
        device,
        vec![fake::page(&[
            "  <node index=\"0\" text=\"每日推荐\" class=\"android.widget.TextView\" clickable=\"true\" bounds=\"[100,200][300,260]\"/>",
        ])],
    );
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "index": null, "reason": "页面上没有对应物" }))]);
    enqueue_fake_role_session(scope, "healer", vec![FakeTurn::tool("report", serde_json::json!({ "verdict": "wrong_page", "diagnosis": "当前是推荐页而非设置入口页，疑第 1 步启动后未落到预期页面" }))]);

    let mut p = (*opts.params).clone();
    p.ai = opts.ai.clone();
    let params = Arc::new(p);
    let p2 = params.clone();
    let runner = crate::workflow::script_runner::ScriptRunner::new(params)
        .with_healer_factory(Arc::new(move |lib_json, script_text: &str| super::healer::copilot_healer(&p2, lib_json, script_text)));

    let exec = runner.run(&tks, None, &mut |_| {}).await.unwrap();

    assert!(!exec.success, "走偏场景不应救活");
    let err = exec.error.clone().unwrap_or_default();
    assert!(err.contains("AI 分诊"), "报错应带分诊结论：{}", err);
    assert!(err.contains("走偏"), "分诊应指向前面步骤：{}", err);
    // 没有任何自愈记账（诊断不是救活）
    assert!(exec.steps.iter().all(|s| s.healed.is_none()), "诊断路径不应有自愈记账");

    fake::remove(device);
}

/// 复用 opts_for 拼一份带 fake ai 的 Params（align_start / copilot 装配都吃 Params）
fn params_with_fake_ai(device: &str, scope: &str, ws: std::path::PathBuf, cache: std::path::PathBuf) -> Arc<Params> {
    let opts = opts_for(device, scope, ws, cache);
    let mut p = (*opts.params).clone();
    p.ai = opts.ai.clone();
    Arc::new(p)
}

/// 手造两件套：无启动步脚本 + 带「起始页」页面实体的 .tklib
fn make_pair_with_start_page(ws: &std::path::Path, cache: &std::path::Path) -> std::path::PathBuf {
    let tks = ws.join("case.tks");
    std::fs::write(&tks, "# 用例: 对齐测试\n步骤:\n点击 [{某按钮}]\n").unwrap();
    let lib_dir = cache.join("mklib");
    std::fs::create_dir_all(&lib_dir).unwrap();
    let lib_json = lib_dir.join("element.json");
    std::fs::write(
        &lib_json,
        r#"{"elements":{},"pages":{"起始页":{"desc":"设置入口页","signature":["进入设置"]}}}"#,
    )
    .unwrap();
    crate::utils::tklib::pack(&lib_json, &crate::utils::tklib::tklib_path(&tks), &crate::utils::tklib::TklibMeta::new("android", "fake")).unwrap();
    tks
}

/// 【起始态对齐 · 导航成功】无启动步脚本 + 当前停在无关页 → 本地匹配不中 → AI navigate
/// 回到起始页 → 实测复验命中 → Aligned。
#[tokio::test]
async fn align_start_navigates_back_to_start_page() {
    let device = "fake:align-ok";
    let scope = "align-ok";
    // 页0=无关页（可点），页1=起始页（含特征「进入设置」）
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("每日推荐", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
        ],
    );

    let ws = temp_dir("align-ok-ws");
    let cache = temp_dir("align-ok-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();
    let tks = make_pair_with_start_page(&ws, &cache);

    // 导航 explorer：点一下（推进到起始页）→ finish
    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "回起始页" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "已回到起始页" })),
            FakeTurn::text("{}"),
        ],
    );

    let params = params_with_fake_ai(device, scope, ws.clone(), cache);
    let out = tksops::align_start(&params, &ui, &tks).await;
    assert!(matches!(out, tksops::AlignOutcome::Aligned), "应导航对齐成功");

    fake::remove(device);
}

/// 【起始态对齐 · 有启动步跳过】首步「启动」→ 冷启动自会对齐，零 LLM 调用直接 Skipped。
#[tokio::test]
async fn align_start_skips_scripts_with_launch_step() {
    let device = "fake:align-skip";
    fake::install(device, vec![fake::page(&[&fake::node("随便", 0, 0, 100, 50)])]);

    let ws = temp_dir("align-skip-ws");
    let cache = temp_dir("align-skip-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();

    let tks = ws.join("case.tks");
    std::fs::write(&tks, "步骤:\n启动 [\"x.app\"]\n点击 [{某按钮}]\n").unwrap();

    let params = params_with_fake_ai(device, "align-skip", ws.clone(), cache);
    // 未 enqueue 任何 fake 会话——走到 LLM 就会 panic/报错，Skipped 证明零 AI 成本
    let out = tksops::align_start(&params, &ui, &tks).await;
    assert!(matches!(out, tksops::AlignOutcome::Skipped(_)), "有启动步应跳过对齐");

    fake::remove(device);
}

/// 【起始态对齐 · 导航失败】navigate 认输且页面未变 → Failed，不该开跑；
/// 报告说明前提须人工处理（登录态/权限类：查得出、说得清、不代办）。
#[tokio::test]
async fn align_start_fails_cleanly_when_navigation_cannot_reach() {
    let device = "fake:align-fail";
    let scope = "align-fail";
    fake::install(device, vec![fake::page(&[&fake::node("每日推荐", 100, 200, 300, 260)])]);

    let ws = temp_dir("align-fail-ws");
    let cache = temp_dir("align-fail-cache");
    let ui = PlainFrontend::new();
    let _wa = Workarea::for_device(Some(device)).unwrap();
    let tks = make_pair_with_start_page(&ws, &cache);

    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![FakeTurn::tool("finish", serde_json::json!({ "success": false, "reason": "找不到回起始页的路" }))],
    );

    let params = params_with_fake_ai(device, scope, ws.clone(), cache);
    let out = tksops::align_start(&params, &ui, &tks).await;
    match out {
        tksops::AlignOutcome::Failed(report) => {
            assert!(report.contains("登录态"), "报告应提示登录态类前提须人工处理：{}", report);
            assert!(report.contains("未到达起始页"), "报告应说清对齐失败：{}", report);
        }
        _ => panic!("导航失败应返回 Failed"),
    }

    fake::remove(device);
}
