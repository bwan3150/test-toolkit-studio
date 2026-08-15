// Fetch 命令处理器（② 原子方法）
// 提取当前页面的元素列表（含 xpath），直接输出 JSON 数组
// 默认先刷新页面状态；--cached 用工作区已有状态

use tke::engines::ocr::resolve_ocr;
use tke::{Result, Fetch, JsonOutput};

/// Fetch 命令参数
#[derive(clap::Args)]
pub struct FetchArgs {
    /// 使用工作区中已有的页面状态，跳过重新采集（先 tke refresh 后使用）
    #[arg(long)]
    pub cached: bool,

    /// 只输出可交互元素
    #[arg(long)]
    pub interactive: bool,

    /// 用 OCR 文字增强元素表（给无 text/content-desc 的图标补可读文字）：
    /// offline=本地 tesseract；online=配置的在线服务；http(s)://...=指定在线服务 URL
    #[arg(long)]
    pub ocr: Option<String>,

    /// 等这段文字出现再返回（轮询采集，出现即刻返回；`|` 分隔多个候选，任一命中即算）。
    /// 等异步结果（后台下发、跨设备同步）用它，别自己写 shell 轮询——
    /// 手写循环最容易忘超时、忘判命中，"跑完"被当成"通过"（ADR-0010：护栏做成子命令）。
    #[arg(long, value_name = "文本")]
    pub wait_text: Option<String>,

    /// `--wait-text` 的超时秒数（默认 30）。超时即**非零退出**，`||` 分支能直接接住
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,
}

/// 处理 Fetch 命令（必须指定 -d/--device）
pub async fn handle(args: FetchArgs, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("fetch 必须指定设备: -d/--device <设备ID>"));

    let fetch = Fetch::new(device)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    // --ocr：解析来源（"online" 用配置的 ocr_url 兜底）
    let ocr_src = args
        .ocr
        .as_deref()
        .and_then(|spec| resolve_ocr(spec, &tke::utils::params::ocr_url()));

    // 标签页信息打到 stderr（人可见，不污染给 Electron 的 stdout JSON）
    let tabs = fetch.list_tabs();
    let tabs_text = tke::format_tabs(&tabs);
    if !tabs_text.is_empty() {
        eprintln!("{}", tabs_text);
    }

    // --wait-text：轮询到文字出现再输出。命中即刻返回（不是死等满），超时非零退出。
    if let Some(ref want) = args.wait_text {
        let cands = tke::utils::scroll::targets(want);
        if cands.is_empty() {
            JsonOutput::error("--wait-text 需要非空文本");
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(args.timeout);
        loop {
            if tke::utils::interrupt::aborted() {
                JsonOutput::error("已中断（用户 Ctrl+C）");
            }
            // 每轮都重新采集——等的就是页面变化，用 --cached 等于永远等不到
            if let Ok(elements) = fetch.elements(false, ocr_src.as_ref()).await {
                let texts: Vec<String> = elements.iter().map(|e| e.to_ai_text()).collect();
                if let Some(hit) = tke::utils::scroll::first_hit(&texts, &cands) {
                    eprintln!("等到了「{}」", hit);
                    let output = if args.interactive {
                        elements.into_iter().filter(|e| e.clickable).collect::<Vec<_>>()
                    } else {
                        elements
                    };
                    JsonOutput::print(&output);
                    return Ok(());
                }
            }
            if std::time::Instant::now() >= deadline {
                // 非零退出 + 说清等了多久：调用方写 `|| echo 没出现` 就能接住，
                // 不会把"轮询跑完"误当成"东西出现了"
                JsonOutput::error(format!(
                    "等待文字「{}」超时（{}s）——它没有出现，或者在视口外（先用 steps 的 `滚动查找`）",
                    want, args.timeout
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    match fetch.elements(args.cached, ocr_src.as_ref()).await {
        Ok(elements) => {
            let output = if args.interactive {
                elements.into_iter().filter(|e| e.clickable).collect::<Vec<_>>()
            } else {
                elements
            };
            // 直接输出元素数组
            JsonOutput::print(&output);
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
