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

    // ===== 提示词自定义（三选一，优先级从上到下）=====
    /// 直接注入主系统提示词文本（最高优先级）
    #[arg(long)]
    pub system_prompt: Option<String>,
    /// 主系统提示词 .md 文件路径
    #[arg(long)]
    pub system_prompt_file: Option<PathBuf>,
    /// 提示词目录（约定 agents/*.md、tools/*.md；覆盖内置默认）
    #[arg(long)]
    pub prompts_dir: Option<PathBuf>,
}

/// 处理 Harness 命令
pub async fn handle(
    args: HarnessArgs,
    params: Arc<tke::Params>,
) -> Result<()> {
    use std::io::IsTerminal;

    // 用例 / 设备 / 平台来源三选一：
    //   --testcase 直跑模式：testcase 是文件则读取、否则当文字；platform 来自 --platform；设备走全局 -d
    //   无 --testcase 且在终端：进入交互式 setup 向导（向导自选设备/平台/用例）
    //   无 --testcase 且非终端（管道/CI）：报错（无法交互）
    let (case_text, wizard_device, platform): (String, Option<String>, Option<Platform>) =
        if let Some(tc) = args.testcase.clone() {
            let p = std::path::Path::new(&tc);
            let text = if p.is_file() {
                std::fs::read_to_string(p).unwrap_or_else(|e| {
                    JsonOutput::error(format!("读取用例文件失败 {}: {}", p.display(), e))
                })
            } else {
                tc
            };
            if params.device().is_none() {
                JsonOutput::error(
                    "harness 需指定设备：-d/--device <设备ID>（或不带 --testcase 启动进入交互向导）",
                );
            }
            (text, None, args.platform.map(|p| p.to_platform()))
        } else if std::io::stdin().is_terminal() {
            run_setup_wizard(&params).await
        } else {
            JsonOutput::error(
                "harness 需要 --testcase <用例>（或在终端中无参启动以进入交互向导）",
            )
        };

    // 脚本输出目录：来自 --scripts 或 config 的 scripts；文件名由 AI 起、自动去重不覆盖
    let script_dir = params.scripts.clone().unwrap_or_else(|| {
        JsonOutput::error("harness 必须指定脚本输出目录: --scripts <目录>（也可写入 config 的 scripts）")
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

    // 前端：--json 时用 JsonFrontend（被 Electron app spawn，NDJSON 双向）；
    //   真 TTY（stdin+stderr 都是终端）用 TuiFrontend（ratatui 全屏交互，失败回落 Plain）；
    //   否则（管道 / CI / 重定向）用 PlainFrontend（行式回归锚点）。
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
    let result = AgentRunner::run(
        AgentRunOptions {
            case: case_text,
            script_dir,
            ai: merged_ai,
            prompt,
            ocr,
            verify,
            platform,
            device: wizard_device,
            params: params.clone(),
        },
        frontend.as_ref(),
    )
    .await
    .unwrap_or_else(|e| JsonOutput::error(e.to_string()));
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

/// 读取一行 stdin（向导专用，朴素 stdin/stdout，发生在前端选择之前，不涉及 TUI/JSON）
async fn read_line(prompt: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    let _ = std::io::stdout().flush();
    tokio::task::spawn_blocking(|| {
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
        s.trim().to_string()
    })
    .await
    .unwrap_or_default()
}

/// 交互式 setup 向导：列设备 → 选设备/平台 → 输入用例。
/// 返回 (用例文字, 设备覆盖(web→None), 平台覆盖)，元组顺序与 handle 解构 (case_text, wizard_device, platform) 一致。
async fn run_setup_wizard(_params: &tke::Params) -> (String, Option<String>, Option<Platform>) {
    // —— 1. 欢迎 ——
    println!("\n=== tke harness 交互式向导 ===");
    println!("AI 将在所选设备上探索测试并生成 .tks 脚本。\n");

    // —— 2. 列 Android 设备（仅 Android 有列举能力）——
    let devices: Vec<String> = match tke::drivers::AdbDriver::new(None) {
        Ok(adb) => adb.get_devices().unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    if devices.is_empty() {
        println!("未检测到 Android 设备（adb 未发现已连接设备，或仅做 Web/iOS 测试）。");
    } else {
        println!("已检测到 Android 设备：");
        for (i, d) in devices.iter().enumerate() {
            println!("  [{}] {}", i + 1, d);
        }
    }

    // —— 3. 选设备 ——
    println!("\n选择目标设备：");
    println!("  · 输入上方序号选 Android 设备");
    println!("  · 或直接粘贴设备 ID（Android 序列号 / iOS UDID / wda:... 等）");
    println!("  · 或输入 web 表示 Web 测试（无设备）");
    let raw = read_line("> ").await;

    let mut device: Option<String> = None;
    let mut platform: Option<Platform> = None;
    if raw.eq_ignore_ascii_case("web") {
        device = None;
        platform = Some(Platform::Web);
    } else if let Ok(n) = raw.parse::<usize>() {
        // 选序号
        if n >= 1 && n <= devices.len() {
            device = Some(devices[n - 1].clone());
            platform = Some(Platform::Android);
        } else {
            println!("序号超出范围，按文字处理：{}", raw);
            device = Some(raw.clone());
            platform = Some(Platform::from_device(Some(&raw)));
        }
    } else if !raw.is_empty() {
        // 直接粘贴的设备 ID
        device = Some(raw.clone());
        platform = Some(Platform::from_device(Some(&raw)));
    }

    // —— 4. 选平台（设备已能推断则跳过；否则提示 ios/web 或回车用推断）——
    if platform.is_none() {
        println!("\n选择平台：ios / web（回车=由设备推断）");
        let p = read_line("> ").await;
        platform = match p.to_lowercase().as_str() {
            "ios" => Some(Platform::Ios),
            "web" => Some(Platform::Web),
            "android" => Some(Platform::Android),
            "" => None,
            other => {
                println!("未识别平台「{}」，将由设备推断。", other);
                None
            }
        };
    }

    // —— 5. 输入用例（文件路径则读取，否则当文字；空则重试一次）——
    println!("\n请输入测试用例（可填一段描述，或 .md/.txt 文件路径）：");
    let mut case_input = read_line("> ").await;
    if case_input.is_empty() {
        println!("用例不能为空，请重新输入：");
        case_input = read_line("> ").await;
        if case_input.is_empty() {
            JsonOutput::error("未提供测试用例，向导退出");
        }
    }
    let case_text = {
        let p = std::path::Path::new(&case_input);
        if p.is_file() {
            std::fs::read_to_string(p).unwrap_or_else(|e| {
                JsonOutput::error(format!("读取用例文件失败 {}: {}", p.display(), e))
            })
        } else {
            case_input
        }
    };

    // —— 6. 确认摘要 ——
    let dev_show = device.clone().unwrap_or_else(|| "(无设备 / Web)".to_string());
    let plat_show = platform.map(|p| p.name()).unwrap_or("(由设备推断)");
    let case_first = case_text.lines().next().unwrap_or("").trim();
    println!("\n--- 确认 ---");
    println!("  设备   {}", dev_show);
    println!("  平台   {}", plat_show);
    println!("  用例   {}", case_first);
    println!();

    (case_text, device, platform)
}
