// Run 命令处理器（③ 工作流）
// tke run <path>  按扩展名分发: .tks=单脚本 / .toml=flow(多脚本顺序执行)
// --log <dir> 时保存完整产物，否则只输出 NDJSON 事件流

use tke::{Result, ScriptRunner, FlowRunner, JsonOutput};
use std::path::PathBuf;
use std::sync::Arc;

use super::EventPrinter;

/// AI 辅助驾驶（定位自愈）的装配：copilot 开启时构造 healer 工厂——回放中某步元素按原
/// 定位找不到（App 小改版/文案微调），AI 依当前实时页面找回并救活本步，然后继续无 AI 运行。
/// 修正只写解包出的临时副本 + 在报告里标注，**不改原 .tks / .tklib**。
/// 默认开启；--copilot false 或 config copilot = false 关闭；未配置 [ai] 时自愈调用会
/// 静默失败、回放按原路径报错（行为同关闭）。
fn healer_factory(params: &Arc<tke::Params>) -> Option<tke::workflow::script_runner::HealerFactory> {
    if !params.copilot {
        return None;
    }
    let p = params.clone();
    Some(Arc::new(move |lib_json, script_text: &str| {
        tke::workflow::agent::runner::healer::copilot_healer(&p, lib_json, script_text)
    }))
}

/// Run 命令参数
#[derive(clap::Args)]
pub struct RunArgs {
    /// 执行的文件路径: .tks 单脚本 / .toml flow
    pub path: PathBuf,
    /// OCR 来源（回放时 ocr/断言元素的识别方式）：
    /// online=用默认在线服务地址(配置 ocr_url) / offline=本地离线 tesseract /
    /// http(s)://... =指定在线服务 URL。不传则沿用「在线 + 配置地址」。
    #[arg(long)]
    pub ocr: Option<String>,
}

/// 处理 Run 命令
pub async fn handle(
    run_args: RunArgs,
    params: std::sync::Arc<tke::Params>,
) -> Result<()> {
    let path = run_args.path;

    // --ocr：CLI > config.ocr；设置进程级 OCR 来源，供回放时 recognizer 解析 ocr 通道元素 / 断言
    if let Some(spec) = run_args.ocr.clone().or_else(|| params.ocr.clone()).as_deref() {
        match tke::engines::ocr::resolve_ocr(spec, &params.ocr_url) {
            Some(src) => tke::engines::ocr::set_ocr_source(src),
            None => JsonOutput::error(format!("无法解析 --ocr 值「{}」（用 online/offline/http(s):// 或确认 ocr_url 已配置）", spec)),
        }
    }
    let mut printer = EventPrinter::auto(params.json);
    let mut emit = move |e: &tke::RunEvent| printer.print(e);

    match path.extension().and_then(|s| s.to_str()) {
        Some("tks") => {
            tke::workflow::script_runner::validate_script_path(&path)
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            // AI 辅助驾驶 · 起始态对齐：无启动步的脚本开跑前把设备带回起始页（防止"从
            // 当前页面闭眼开跑"）。已在起始页/有启动步/无参照 → 零成本跳过；导航后仍
            // 不在起始页 → 不开跑（在错误页面上回放可能产生副作用），报告说清前提
            // （登录态/权限类只诊断不代办）。UiEvent 走 stderr，不污染 stdout 的 NDJSON。
            // flow(.toml) 不做：脚本间连续性是有意设计（web 会话保留可测联动）。
            if params.copilot {
                use tke::workflow::agent::runner::tksops::{align_start, AlignOutcome};
                let ui = tke::PlainFrontend::compact(); // 紧凑输出：不打阶段大标题，Notice 顶格
                match align_start(&params, &ui, &path).await {
                    AlignOutcome::Failed(report) => {
                        JsonOutput::error(format!("起始态对齐失败，未开始回放。{}", report))
                    }
                    // 有过导航输出 → 空一行再开脚本执行（与对齐过程视觉分段）
                    AlignOutcome::Aligned => eprintln!(),
                    AlignOutcome::AlreadyThere | AlignOutcome::Skipped(_) => {}
                }
            }

            // 元素库：ScriptRunner 内部按「同名 .tklib 两件套」解析，缺包直接报错（无共享库）
            let mut runner = ScriptRunner::new(params.clone());
            if let Some(factory) = healer_factory(&params) {
                runner = runner.with_healer_factory(factory);
            }
            let result = runner
                .run(&path, params.log.as_deref(), &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            // 退出码反映执行结果（事件流中已包含完整信息）
            std::process::exit(if result.success { 0 } else { 1 });
        }
        Some("toml") => {
            if !path.exists() {
                JsonOutput::error(format!("flow 文件不存在: {}", path.display()));
            }

            let mut runner = FlowRunner::new(params.clone());
            if let Some(factory) = healer_factory(&params) {
                runner = runner.with_healer_factory(factory);
            }
            let result = runner
                .run(&path, params.log.as_deref(), &mut emit)
                .await
                .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

            std::process::exit(if result.success { 0 } else { 1 });
        }
        _ => JsonOutput::error(format!(
            "无法识别的文件类型: {} (支持 .tks 单脚本 / .toml flow)",
            path.display()
        )),
    }
}
