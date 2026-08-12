// 【.tklib 可移植性无设备测试】方案的核心承诺——"foo.tks + foo.tklib 两件套复制到别的
// 机器直接能跑"——变成一条 CI 断言：探索照生产路径产出两件套 → 整体拷到另一个目录
// （模拟另一台机器，原目录/临时库全部不可见）→ replay_tks 解包邻居 tklib 回放 → 到达目标标志。

use std::sync::Arc;

use crate::drivers::fake;
use crate::utils::tklib;
use crate::workflow::agent::provider::{enqueue_fake_role_session, FakeTurn};
use crate::workflow::agent::prompt::PromptSpec;
use crate::workflow::agent::ui::PlainFrontend;
use crate::{Params, Workarea};

use super::options::AgentRunOptions;
use super::testrun::TestRun;
use super::tksops;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("tke-tklib-it-{}-{}", std::process::id(), name));
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
    }
}

/// 两件套可移植：TestRun 探索+finalize 产出 .tks/.tklib → 拷到另一个目录 → replay_tks 达标。
#[tokio::test]
async fn tks_and_tklib_are_portable_across_directories() {
    let device = "fake:tklib-port";
    let scope = "tklib-port";
    fake::install(
        device,
        vec![
            fake::page(&[&fake::node("进入设置", 100, 200, 300, 260)]),
            fake::page(&[&fake::node("设置中心", 100, 100, 400, 160)]),
        ],
    );

    let machine_a = temp_dir("machine-a");
    let machine_b = temp_dir("machine-b");
    let cache = temp_dir("cache");
    let ui = PlainFrontend::new();
    let _workarea = Workarea::for_device(Some(device)).unwrap();

    // —— 机器 A：探索产出两件套（探索官/收尾语义命名的反思官都按角色注入）——
    enqueue_fake_role_session(
        scope,
        "explorer",
        vec![
            FakeTurn::tool("launch", serde_json::json!({ "target": "settings.app", "comment": "打开应用" })),
            FakeTurn::tool("click", serde_json::json!({ "element_id": 0, "comment": "进设置" })),
            FakeTurn::tool("finish", serde_json::json!({ "success": true, "reason": "到了设置中心" })),
            FakeTurn::text("{}"), // desc 生成轮
        ],
    );
    enqueue_fake_role_session(scope, "reflector", vec![FakeTurn::tool("report", serde_json::json!({}))]); // 语义命名：全保留特征名

    let opts_a = opts_for(device, scope, machine_a.clone(), cache.clone());
    let run = TestRun::explore(&opts_a, &ui, "打开设置中心", None, false, false, super::ctx::AskMode::Ask, false).await.unwrap();
    let tks_a = machine_a.join("open-settings.tks");
    let tklib_a = tklib::tklib_path(&tks_a);
    let result = run.finalize(&opts_a, &ui, &tks_a, &tklib_a).await.unwrap();
    assert!(result.success, "探索应达成：{}", result.finish_reason);
    assert!(tks_a.is_file(), ".tks 应落盘");
    assert!(tklib_a.is_file(), ".tklib 元素包应落盘");

    // 元素包自查：确实带了 element.json + 引用的模板图
    let peek = temp_dir("peek");
    let lib_json = tklib::unpack(&tklib_a, &peek).unwrap();
    let lib: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&lib_json).unwrap()).unwrap();
    let elements = lib["elements"].as_object().unwrap();
    assert!(!elements.is_empty(), "元素包应含点击引用的元素");
    for (name, entry) in elements {
        if let Some(img_rel) = entry["img"].as_str() {
            assert!(peek.join(img_rel).is_file(), "元素「{}」的模板图应随包携带", name);
        }
    }

    // —— "复制到另一台机器"：只拷两个文件，原目录/缓存里的任何东西都不该被依赖 ——
    let tks_b = machine_b.join("open-settings.tks");
    std::fs::copy(&tks_a, &tks_b).unwrap();
    std::fs::copy(&tklib_a, tklib::tklib_path(&tks_b)).unwrap();

    // —— 机器 B：回放。marker 由 verify 角色一次性会话推导（随后持久化进 .tks 头）——
    enqueue_fake_role_session(scope, "verify", vec![FakeTurn::tool("report", serde_json::json!({ "goal_marker": "设置中心" }))]);
    let opts_b = opts_for(device, scope, machine_b.clone(), cache.clone());
    let msg = tksops::replay_tks(&opts_b, &ui, &tks_b, "打开设置中心").await.unwrap();
    assert!(msg.contains("回放通过"), "两件套拷走后应能直接回放到达目标，实际：{}", msg);

    // marker 已持久化进脚本头（下次回放不再推导）
    let tks_content = std::fs::read_to_string(&tks_b).unwrap();
    assert!(tks_content.contains("# 目标标志: 设置中心"), "marker 应写入脚本头：\n{}", tks_content);

    fake::remove(device);
}
