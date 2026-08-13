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

/// 缺 `-d` 时，从同名 `.tklib` 的 meta.json 推断该跑哪个平台（Q-6：两件套自包含）。
///
/// 两件套的承诺是「拷到别的机器直接能跑」（INV-7），但 `.tks` 本身不记平台——而元素包
/// 打包时早就记下了。这里把这口气补上：**平台**从包里读，**具体哪台设备**按平台各论：
///   - web    设备无个体差异，`device="web"` 直接可用 → 真正的零参数回放
///   - android 录制那台的序列号换机后必然失效，故只放行、不指定 → 走默认 adb 设备
///             （连了多台仍会撞 adb 自己的「more than one device」，那才是该让用户选的时候）
///   - ios    UDID 同样不可照搬，且 iOS 没有「默认设备」的说法 → 仍要求显式给，
///            但把录制时的 UDID 附在报错里，方便对照
///
/// 推不出来（没有包 / 包里没 meta / 平台不认识）就报原来的缺设备错误。
/// 提示一律走 stderr：stdout 是 NDJSON 事件流，不能污染。
fn infer_device_from_pack(
    params: &std::sync::Arc<tke::Params>,
    tks: &std::path::Path,
) -> std::sync::Arc<tke::Params> {
    const MISSING: &str =
        "tke run 必须指定设备: -d/--device <设备ID>（web / Android 序列号 / iOS UDID）";

    let pack = tke::utils::tklib::tklib_path(tks);
    let Some(meta) = tke::utils::tklib::read_meta(&pack) else {
        JsonOutput::error(MISSING);
    };

    match meta.platform.as_str() {
        "web" => {
            eprintln!("ℹ 未指定设备，按元素包记录的平台回放：web");
            std::sync::Arc::new(params.with_device(Some("web".to_string())))
        }
        "android" => {
            eprintln!("ℹ 未指定设备，按元素包记录的平台回放：android（用默认 adb 设备）");
            params.clone()
        }
        "ios" => JsonOutput::error(format!(
            "{}。该脚本录自 iOS（元素包记录的 UDID: {}），iOS 必须显式指定设备",
            MISSING, meta.device
        )),
        other => JsonOutput::error(format!(
            "{}。元素包记录的平台「{}」无法识别",
            MISSING, other
        )),
    }
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

            // 设备来源：显式 `-d` 优先；没给就从同名 .tklib 的 meta.json 兜底推断平台
            // （两件套自包含，Q-6）。仍推不出来才报错——.tks 不记平台，不给 -d 会被当成
            // Android，web 用例只会得到一句莫名其妙的「adb 缺失」（用户实测踩过）。
            // 放在脚本校验之后：文件不存在/缺元素包是更基础的问题，先报那个。
            // flow(.toml) 不在此校验——每项可自带 device，由 FlowRunner 逐项检查。
            let params = if params.device().is_none() {
                infer_device_from_pack(&params, &path)
            } else {
                params
            };

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
