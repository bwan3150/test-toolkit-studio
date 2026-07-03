// Run 命令处理器（③ 工作流）
// tke run <path>  按扩展名分发: .tks=单脚本 / .toml=flow(多脚本顺序执行)
// --log <dir> 时保存完整产物，否则只输出 NDJSON 事件流

use tke::{Result, ScriptRunner, FlowRunner, JsonOutput};
use std::path::PathBuf;

use super::EventPrinter;

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

            // 元素库解析：-e 显式 > 同名 .tklib 元素包（解包到 cache 后使用）> 无库（仅坐标步）。
            // 共享库默认查找已删除——每个脚本自持 foo.tklib，两个文件拷到任何机器即可回放。
            let params = if params.element_lib().is_none() {
                let tklib = tke::utils::tklib::tklib_path(&path);
                if tklib.is_file() {
                    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let dest = params.cache_root().join("tklib-unpack").join(format!("{}-{}", stem, std::process::id()));
                    let lib_json = tke::utils::tklib::unpack(&tklib, &dest)
                        .unwrap_or_else(|e| JsonOutput::error(format!("解包元素包失败 {}: {}", tklib.display(), e)));
                    std::sync::Arc::new(params.with_element_lib(lib_json))
                } else {
                    params
                }
            } else {
                params
            };

            let runner = ScriptRunner::new(params.clone());
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

            let runner = FlowRunner::new(params.clone());
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
