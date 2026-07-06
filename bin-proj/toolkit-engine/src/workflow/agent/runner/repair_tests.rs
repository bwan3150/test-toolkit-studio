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
            crate::utils::config::TkeConfig::default(),
        )),
    }
}

/// 坏脚本（第 2 步点不存在的元素）→ 诊断停在失败现场 → explorer 续探点对按钮 →
/// 前缀+新尾巴复诊达标 → 写回脚本 + 回包 tklib，marker 头保留。
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
    enqueue_fake_role_session(scope, "asserter", vec![FakeTurn::text(r#"{"index": 0, "reason": "设置中心是该页专属标志"}"#)]);
    enqueue_fake_role_session(scope, "supervisor", vec![FakeTurn::text(r#"{"approved": true, "reason": "已到设置中心"}"#)]);

    let opts = opts_for(device, scope, ws.clone(), cache);
    let msg = tksops::repair_tks(&opts, &ui, &tks, "打开设置中心").await.unwrap();
    assert!(msg.contains("已修复"), "实际：{}", msg);
    assert!(msg.contains("断点续探 1 次"), "实际：{}", msg);

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

/// 续探也走不到目标（explorer 判失败）→ 修复放弃，**脚本一字不改**——绝不把还能跑一半的脚本改坏。
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
    let msg = tksops::repair_tks(&opts, &ui, &tks, "到一个不存在的页面").await.unwrap();
    assert!(msg.contains("修复失败"), "实际：{}", msg);
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
