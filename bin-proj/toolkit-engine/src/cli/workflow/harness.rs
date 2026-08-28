// Harness 命令处理器（③ 工作流）
// tke harness --script <导出.tks目录> [-d 设备 | --platform web] [--testcase 开场白] [-c 配置]
// tke 是一个**对话式、测试领域专精的 AI agent**（CLI/TUI/JSON spawn 三前端），能操作
// 安卓/iOS/web，探索测试并生成 .tks 脚本。默认就进对话——用例靠对话给（编排官会问），
// `--testcase` 只是可选开场白；不再是"接到指令就走过场"的单次触发。
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
    /// 可选开场白：.md/.txt 文件路径，或一段用例文字。不传则进对话——
    /// 终端下向导只选设备/平台、用例由编排官在对话里问；app(--json) 经 NDJSON 发首条消息。
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
    /// 覆盖 [ai].reasoning_effort（供应商无关推理强度）：
    /// none(关) / low / medium / high / xhigh / max / budget:N。缺省 medium。
    #[arg(long)]
    pub reasoning_effort: Option<String>,
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

    // ===== 源码沙盒（ADR-0025 P1）=====
    /// 被测项目的源码工作树（已 checkout 到待测分支）。给了它，AI 多一个
    /// `changed_surfaces` 工具：这次改动碰了哪些界面。**只回界面名/文件/行数，不回代码**
    #[arg(long)]
    pub source_tree: Option<PathBuf>,
    /// 变更面的对照基线（通常是主干分支名，如 main）。不给就算不出变更面
    #[arg(long)]
    pub source_base: Option<String>,
}

/// 处理 Harness 命令
pub async fn handle(
    args: HarnessArgs,
    params: Arc<tke::Params>,
) -> Result<()> {
    use std::io::IsTerminal;

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

    // 合并 AI 配置（提前到 setup 前：准备阶段向导要显示当前模型/供应商/推理）
    let merged_ai = AiConfig {
        provider: args.ai_provider.or(params.ai.provider.clone()),
        model: args.ai_model.or(params.ai.model.clone()),
        api_key: args.ai_key.or(params.ai.api_key.clone()),
        base_url: args.ai_base_url.or(params.ai.base_url.clone()),
        max_rounds: args.max_rounds.or(params.ai.max_rounds),
        prompts_dir: params.ai.prompts_dir.clone(),
        reasoning_effort: args.reasoning_effort.or(params.ai.reasoning_effort.clone()),
    };

    // —— 用例 / 设备 / 平台 ——
    // tke 默认就是个对话式 agent：任务靠对话给（编排官会问），`--testcase` 只是**可选开场白**。
    //   有 --testcase：用它当开场白；设备走 -d、平台走 --platform。
    //   无 --testcase + 终端：进交互向导**只选设备/平台 + 显示配置**，用例留空进对话。
    //   无 --testcase + --json / 非终端：用例留空，设备/平台走参数（app 经 NDJSON 发首条消息）。
    // 产物落点：.tks 脚本落**工作区当前目录**（AI 命名、像 coding agent）；运行中间文件落 cache。
    // 都不在这里问、也不再需要 --scripts。
    let (case_text, dev_override, platform): (String, Option<String>, Option<Platform>) =
        if let Some(tc) = args.testcase.clone() {
            let platform = args.platform.map(|p| p.to_platform());
            // `--platform web` 要连带把设备定成 "web"：web 是唯一「设备 id 就是平台名」的端，
            // 不补的话 device 留 None → 下游按 Android 推断 → 报「adb 缺失」。
            // （交互式向导选 Web 时本就会设成 "web"，这里是把两条路径拉齐。）
            let dev = match platform {
                Some(Platform::Web) => Some("web".to_string()),
                // iOS 的 UDID / Android 的序列号推不出来,保持 None（走 -d 或默认设备）
                _ => None,
            };
            (load_case(&tc), dev, platform)
        } else if !params.json && std::io::stdin().is_terminal() {
            match interactive_setup(frontend.as_ref(), &merged_ai).await {
                Some(s) => (String::new(), s.device, s.platform),
                None => {
                    // setup 阶段用户 Ctrl+C / 放弃：恢复终端后安静退出
                    frontend.shutdown().await;
                    std::process::exit(130);
                }
            }
        } else {
            (String::new(), None, args.platform.map(|p| p.to_platform()))
        };
    // script_dir 已不再决定产物落点（.tks 落工作区）；保留字段兼容，给个工作区兜底值。
    let script_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // 没有默认设备**不再报错**（ADR-0011）：编排官会先 list_devices、按任务语义选，
    // 拿不准就 ask_user 问用户。设备类工具真的没设备可用时会拿到明确报错并纠正。

    // setup 选好的参数（探索开始前发一次）：设备/平台展示一行；模型/供应商/推理由 TUI
    // 存起来供 /model 查询，不再灌进消息流（此前的"配置 · 模型 …"整行已移除）
    frontend.emit(tke::UiEvent::SessionInfo {
        device: dev_override
            .clone()
            .or_else(|| params.device())
            .unwrap_or_else(|| "未定（由 AI 按任务选）".to_string()),
        platform: platform.map(|p| p.name().to_string()).unwrap_or_else(|| "(推断)".to_string()),
        model: merged_ai.model.clone().unwrap_or_else(|| "默认（按供应商）".to_string()),
        provider: merged_ai.provider.clone().unwrap_or_else(|| "anthropic（默认）".to_string()),
        // **说实际会发生什么,不是说配置里写了什么**:非 reasoning 世代的模型
        // 我们不发思考参数(否则上游 400),这一行要跟着变 ——
        // 否则界面上写着"推理 medium"而实际压根没开,那种不一致谁也看不出来
        reasoning: effective_reasoning_label(&merged_ai),
    });

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
        tke::engines::ocr::set_ocr_source(src.clone());
    }

    // verify：CLI --verify 出现 或 config.verify=true 即开启
    let verify = args.verify || params.verify;

    // 源码沙盒：给了工作树才有。**拿不到就当没有**，退回盲盒探索 ——
    // 把它做成硬依赖会让"源码那边出了点问题"变成"测试全线停摆"（ADR-0025）
    let source = args.source_tree.as_ref().map(|tree| {
        let base = args.source_base.clone().unwrap_or_default();
        let sha = tke::sandbox::head_sha(tree);
        tke::workflow::agent::runner::options::SourceContext { tree: tree.clone(), base, sha }
    });

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
            source,
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

/// 交互式 setup：在已建好的前端（通常是全屏 TUI）里**只挑设备 / 平台**。
/// 用例不在这里问——开场后由编排官 ask_user 在对话里问（tke 是对话式 agent）。
/// 全程走前端的 await_choice/await_answer（TUI 下是列表/输入框，Ctrl+C 即时退出）。
/// 准备阶段向导的解析结果：只挑设备/平台（.tks 落工作区、中间文件落 cache，无需在这里问目录）。
struct SetupResult {
    device: Option<String>,
    platform: Option<Platform>,
}

/// 交互式准备阶段：选设备/平台 + **显示当前配置**（模型/供应商/推理，来自 CLI/config）。
/// 用户中途 Ctrl+C / 放弃返回 None。
async fn interactive_setup(ui: &dyn tke::Frontend, _ai: &AiConfig) -> Option<SetupResult> {
    // —— 设备/平台 ——
    // **一律走 discover()**：它是"这台机器能测什么"的单一来源，`tke device list`
    // 用的也是它。早先这里是自己 adb 列一遍、iOS 让人手打 UDID、模拟器压根不出现——
    // 于是同一个问题在两个地方有两套答案，而手打 UDID 这道门槛本身就没必要。
    let found = tke::tools::discover::discover();

    enum Kind {
        /// 一台具体设备（id 已经是 `-d` 能直接用的值）
        Target(String),
        /// 不在这里定：设备由编排官按任务语义选（ADR-0011），跨设备任务必须走这条
        Defer,
        /// 兜底：discover 没列到（没装 adb、设备刚插上还没认出来……）
        Manual,
    }
    let mut opts: Vec<String> = Vec::new();
    let mut kinds: Vec<Kind> = Vec::new();
    for t in &found.targets {
        // 选项串用「组\t标签」编码，render_choice 按组渲染小标题
        let group = match t.kind {
            "android" => "Android",
            "ios" => "iOS 真机",
            "ios-sim" => "iOS 模拟器",
            _ => "Web",
        };
        opts.push(format!("{}\t{} {}　{}", group, t.model, t.os, t.id));
        kinds.push(Kind::Target(t.id.clone()));
    }
    // 某一类**没查成**也要摆出来（INV-9）：不然人只会以为"设备没连"，
    // 而真相可能是"没装 adb / 没装 idb"
    for sk in &found.skipped {
        opts.push(format!("未检测\t{}：{}", sk.kind, sk.why.lines().next().unwrap_or("")));
        kinds.push(Kind::Manual);
    }
    // 跨设备/跨平台任务在这里选哪一台都是错的——交给编排官按任务语义决定
    opts.push("由 AI 决定\t按任务描述选设备（跨设备任务选这个；不确定时它会问你）".to_string());
    kinds.push(Kind::Defer);
    opts.push("手动输入\t上面都不是（自己填设备 ID）".to_string());
    kinds.push(Kind::Manual);

    let idx = ui.await_choice("选择设备".to_string(), opts).await?;
    let (device, platform): (Option<String>, Option<Platform>) = match &kinds[idx] {
        Kind::Target(id) => (Some(id.clone()), Some(Platform::from_device(Some(id)))),
        Kind::Defer => (None, None),
        Kind::Manual => {
            let id = ui
                .await_answer(
                    0,
                    "设备 ID（安卓序列号 / iOS UDID / sim:模拟器UDID / web）".to_string(),
                )
                .await?;
            let id = id.trim().to_string();
            if id.is_empty() {
                (Some("web".to_string()), Some(Platform::Web))
            } else {
                (Some(id.clone()), Some(Platform::from_device(Some(&id))))
            }
        }
    };

    // 模型/供应商/推理不再打进消息流——随 SessionInfo 一次性下发，TUI 用 /model 查询
    Some(SetupResult { device, platform })
}

/// 这次打算用的推理强度（给 SessionInfo 显示）。
///
/// **是"打算"不是"一定"**：上游若拒绝这个参数，会在第一次调用时自动摘掉重试
/// （探测 + 回退，见 provider::session）。这里不预判 —— 预判就又变成一张追不上的名单。
fn effective_reasoning_label(ai: &tke::AiConfig) -> String {
    ai.reasoning_effort.clone().unwrap_or_else(|| "medium（默认）".to_string())
}
