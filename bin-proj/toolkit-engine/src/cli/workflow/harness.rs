// Harness 命令处理器（③ 工作流）
// tke harness <用例.md|"用例文字"> --script <导出.tks路径> [-d 设备] [-c 配置]
// AI 在真机上探索测试并生成 .tks 脚本（tke 内置 AI 闭环，已替代废弃的 tester-ai）
//
// 参数来源优先级：CLI 显式 --ai-* / --system-prompt* > 配置文件 [ai] 段。
// 敏感 key 建议放 -c 配置文件，避免出现在进程命令行。

use std::path::PathBuf;
use std::sync::Arc;

use tke::{AgentRunner, AgentRunOptions, AiConfig, JsonOutput, Platform, PromptSpec, Result};

/// clap 平台枚举（--platform android/ios/web）
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum PlatformArg {
    Android,
    Ios,
    Web,
}

impl PlatformArg {
    pub fn to_platform(self) -> Platform {
        match self {
            PlatformArg::Android => Platform::Android,
            PlatformArg::Ios => Platform::Ios,
            PlatformArg::Web => Platform::Web,
        }
    }
}

/// Harness 命令参数
#[derive(clap::Args)]
pub struct HarnessArgs {
    /// 测试用例：.md/.txt 文件路径，或一段文字描述（不传则进入交互式向导）
    #[arg(long)]
    pub testcase: Option<String>,

    /// 目标平台（不传则由设备推断 / 向导里选）
    #[arg(long, value_enum)]
    pub platform: Option<PlatformArg>,

    /// 覆盖 [ai].provider（anthropic/openai/gemini/deepseek/doubao/qwen）
    #[arg(long)]
    pub ai_provider: Option<String>,
    /// 覆盖 [ai].model
    #[arg(long)]
    pub ai_model: Option<String>,
    /// 覆盖 [ai].api_key（敏感，建议改用 -c 配置文件）
    #[arg(long)]
    pub ai_key: Option<String>,
    /// 覆盖 [ai].base_url（OpenAI 兼容端点：doubao/qwen）
    #[arg(long)]
    pub ai_base_url: Option<String>,
    /// 覆盖探索最大轮数
    #[arg(long)]
    pub max_rounds: Option<u32>,

    /// 用 OCR 文字增强每轮元素表（给无 text/content-desc 的图标补可读文字）：
    /// offline=本地 tesseract；online=配置的在线服务；http(s)://...=指定在线服务 URL。
    /// 不传则不跑 OCR（行为同此前）
    #[arg(long)]
    pub ocr: Option<String>,

    /// 生成脚本后自检+自修复：重启净化→从头 tke run 回放生成的 .tks→失败则让 AI 从
    /// 失败步重新探索续接，直到连续通过 2 次。
    #[arg(long)]
    pub verify: bool,

    // ===== 提示词自定义 =====
    /// 直接注入【编排官(primary)】系统提示词文本（最高优先级；只覆盖编排官，不影响 explorer 等 worker）
    #[arg(long)]
    pub system_prompt: Option<String>,
    /// 【编排官(primary)】系统提示词 .md 文件路径（explorer/doctor 等 worker 角色请用 --prompts-dir）
    #[arg(long)]
    pub system_prompt_file: Option<PathBuf>,
    /// 提示词目录（约定 agents/<role>.md、tools/...、messages/...；可覆盖任意角色，含编排官与各 worker）
    #[arg(long)]
    pub prompts_dir: Option<PathBuf>,
}

/// 处理 Harness 命令
pub async fn handle(
    args: HarnessArgs,
    params: Arc<tke::Params>,
) -> Result<()> {
    use std::io::IsTerminal;

    // 脚本输出目录：尽早校验（setup 之前），避免交互完才报错
    let script_dir = params.scripts.clone().unwrap_or_else(|| {
        JsonOutput::error("harness 必须指定脚本输出目录: --scripts <目录>（也可写入 config 的 scripts）")
    });

    // —— 前端：在 setup 之前就建好 ——
    //   --json → JsonFrontend（被 Electron app spawn，NDJSON 双向）
    //   真 TTY（stdin+stderr 都是终端）→ TuiFrontend（ratatui 全屏，失败回落 Plain）
    //   否则（管道/CI/重定向）→ PlainFrontend（行式）
    // 无 --testcase 时，setup（选设备/输入用例）也在这个前端里完成——一进去就是全屏 TUI，
    // 且 setup 阶段 Ctrl+C 是 TUI 按键事件、可即时退出（不再卡在阻塞 stdin 上）。
    let frontend: Box<dyn tke::Frontend> = if params.json {
        Box::new(tke::JsonFrontend::spawn())
    } else if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        match tke::TuiFrontend::spawn() {
            Ok(f) => Box::new(f),
            Err(_) => Box::new(tke::PlainFrontend::new()),
        }
    } else {
        Box::new(tke::PlainFrontend::new())
    };

    // —— 用例 / 设备 / 平台 ——
    //   --testcase 直跑：文件则读取、否则当文字；platform 来自 --platform；设备走全局 -d
    //   无 --testcase + 终端：在前端里交互 setup（选设备/输入用例）
    //   无 --testcase + --json / 非终端：报错（app/管道场景必须显式给 --testcase）
    let (case_text, dev_override, platform): (String, Option<String>, Option<Platform>) =
        if let Some(tc) = args.testcase.clone() {
            let text = load_case(&tc);
            if params.device().is_none() {
                JsonOutput::error(
                    "harness 需指定设备：-d/--device <设备ID>（或不带 --testcase 启动进入交互向导）",
                );
            }
            (text, None, args.platform.map(|p| p.to_platform()))
        } else if params.json {
            JsonOutput::error("--json 模式必须用 --testcase 提供用例（app 集成不进交互向导）");
        } else if std::io::stdin().is_terminal() {
            match interactive_setup(frontend.as_ref()).await {
                Some(s) => s,
                None => {
                    // setup 阶段用户 Ctrl+C / 放弃：恢复终端后安静退出
                    frontend.shutdown().await;
                    std::process::exit(130);
                }
            }
        } else {
            JsonOutput::error(
                "harness 需要 --testcase <用例>（或在终端中无参启动以进入交互向导）",
            )
        };

    // setup 选好的参数显示在 TUI 顶部（探索开始前的第一条）
    frontend.emit(tke::UiEvent::SessionInfo {
        device: dev_override
            .clone()
            .or_else(|| params.device())
            .unwrap_or_else(|| "(web)".to_string()),
        platform: platform.map(|p| p.name().to_string()).unwrap_or_else(|| "(推断)".to_string()),
        case: case_text.lines().next().unwrap_or("").trim().to_string(),
    });

    // 合并 AI 配置：CLI --ai-* 覆盖 config [ai] 段（查 params.ai）
    let merged_ai = AiConfig {
        provider: args.ai_provider.or(params.ai.provider.clone()),
        model: args.ai_model.or(params.ai.model.clone()),
        api_key: args.ai_key.or(params.ai.api_key.clone()),
        base_url: args.ai_base_url.or(params.ai.base_url.clone()),
        max_rounds: args.max_rounds.or(params.ai.max_rounds),
        prompts_dir: params.ai.prompts_dir.clone(),
    };

    // 提示词来源：CLI 文本/文件优先；目录 CLI > 配置 [ai].prompts_dir
    let prompt = PromptSpec {
        system_text: args.system_prompt,
        system_file: args.system_prompt_file,
        prompts_dir: args
            .prompts_dir
            .or_else(|| merged_ai.prompts_dir.clone().map(PathBuf::from)),
    };

    // --ocr：CLI > config.ocr；"online" 用配置的 ocr_url 兜底；都没有则不跑 OCR
    let ocr_spec = args.ocr.clone().or_else(|| params.ocr.clone());
    let ocr = ocr_spec
        .as_deref()
        .and_then(|spec| tke::engines::ocr::resolve_ocr(spec, &tke::utils::params::ocr_url()));
    // 同时设进程级 OCR 来源：让验证/医生阶段的回放（recognizer 解析 ocr 元素/断言）与探索用同一模式
    if let Some(src) = &ocr {
        tke::utils::params::set_ocr_source(src.clone());
    }

    // verify：CLI --verify 出现 或 config.verify=true 即开启
    let verify = args.verify || params.verify;

    // —— 运行 ——（无论成败都先 emit Done + shutdown 恢复终端，再决定退出码/报错）
    let run_result = AgentRunner::run(
        AgentRunOptions {
            case: case_text,
            script_dir,
            ai: merged_ai,
            prompt,
            ocr,
            verify,
            platform,
            device: dev_override,
            params: params.clone(),
        },
        frontend.as_ref(),
    )
    .await;
    let result = match run_result {
        Ok(r) => r,
        Err(e) => {
            frontend.emit(tke::UiEvent::Done { success: false, script: None, conversation: String::new() });
            frontend.shutdown().await;
            JsonOutput::error(e.to_string());
        }
    };
    frontend.emit(tke::UiEvent::Done {
        success: result.success,
        script: if result.success { Some(result.script.display().to_string()) } else { None },
        conversation: result.conversation.display().to_string(),
    });
    frontend.shutdown().await;

    // 状态/依据/模型/token/元素变更已在「结果」框中统一展示，这里只补产物路径。
    // 未稳定通过的脚本已被删除（不留半成品），只显示日志路径供复盘。
    if result.success {
        eprintln!("  脚本   {}", result.script.display());
    }
    eprintln!("  日志   {}", result.conversation.display());

    std::process::exit(if result.success { 0 } else { 1 });
}

/// 用例：文件路径则读取内容，否则当作文字描述
fn load_case(s: &str) -> String {
    let p = std::path::Path::new(s);
    if p.is_file() {
        std::fs::read_to_string(p)
            .unwrap_or_else(|e| JsonOutput::error(format!("读取用例文件失败 {}: {}", p.display(), e)))
    } else {
        s.to_string()
    }
}

/// 交互式 setup：在已建好的前端（通常是全屏 TUI）里逐项问设备 / 用例。
/// 全程走前端的 await_answer（TUI 下是输入框，Ctrl+C 即时退出）。
/// 返回 (用例文字, 设备覆盖(web→Some("web")), 平台覆盖)；用户中途 Ctrl+C / 放弃返回 None。
async fn interactive_setup(
    ui: &dyn tke::Frontend,
) -> Option<(String, Option<String>, Option<Platform>)> {
    // 列 Android 设备（仅 Android 有列举能力；iOS/Web 让用户手填 / 选）
    let devices: Vec<String> = match tke::drivers::AdbDriver::new(None) {
        Ok(adb) => adb.get_devices().unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    // 候选 = 已连 Android 设备 + Web + 手动输入（iOS/wda 暂无列举能力，走手动）
    let mut options: Vec<String> = devices.clone();
    options.push("web — 网页测试（无设备）".to_string());
    options.push("手动输入设备 ID（iOS UDID / wda:.. 等）".to_string());
    let idx = ui.await_choice("选择目标设备".to_string(), options).await?;
    let (device, platform): (Option<String>, Option<Platform>) = if idx < devices.len() {
        (Some(devices[idx].clone()), Some(Platform::Android))
    } else if idx == devices.len() {
        (Some("web".to_string()), Some(Platform::Web))
    } else {
        // 手动输入设备 ID
        let id = ui
            .await_answer(0, "请输入设备 ID（iOS UDID / wda:.. 等）".to_string())
            .await?;
        let id = id.trim().to_string();
        if id.is_empty() {
            (Some("web".to_string()), Some(Platform::Web))
        } else {
            (Some(id.clone()), Some(Platform::from_device(Some(&id))))
        }
    };

    let case_in = ui
        .await_answer(0, "请输入测试用例（一段描述，或 .md/.txt 文件路径）".to_string())
        .await?;
    let case_text = load_case(case_in.trim());

    Some((case_text, device, platform))
}
