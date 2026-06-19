// Harness 命令处理器（③ 工作流）
// tke harness <用例.md|"用例文字"> --script <导出.tks路径> [-d 设备] [-c 配置]
// AI 在真机上探索测试并生成 .tks 脚本（tke 内置 AI 闭环，已替代废弃的 tester-ai）
//
// 参数来源优先级：CLI 显式 --ai-* / --system-prompt* > 配置文件 [ai] 段。
// 敏感 key 建议放 -c 配置文件，避免出现在进程命令行。

use std::path::PathBuf;
use std::sync::Arc;

use tke::{AgentRunner, AgentRunOptions, AiConfig, JsonOutput, PromptSpec, Result};

/// Harness 命令参数
#[derive(clap::Args)]
pub struct HarnessArgs {
    /// 测试用例: .md/.txt 文件路径，或一段文字描述
    pub case: String,

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
    // 早期校验设备（AgentRunner 内部也经 params 查表）
    if params.device().is_none() {
        JsonOutput::error("harness 必须指定设备: -d/--device <设备ID>");
    }

    // 脚本输出目录：来自 --scripts 或 config 的 scripts；文件名由 AI 起、自动去重不覆盖
    let script_dir = params.scripts.clone().unwrap_or_else(|| {
        JsonOutput::error("harness 必须指定脚本输出目录: --scripts <目录>（也可写入 config 的 scripts）")
    });

    // 用例：文件则读取内容，否则当作文字
    let case_text = {
        let p = std::path::Path::new(&args.case);
        if p.is_file() {
            std::fs::read_to_string(p)
                .unwrap_or_else(|e| JsonOutput::error(format!("读取用例文件失败 {}: {}", p.display(), e)))
        } else {
            args.case.clone()
        }
    };

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

    // --ocr：解析来源（"online" 用配置的 ocr_url 兜底）；不传则不跑 OCR
    let ocr = args
        .ocr
        .as_deref()
        .and_then(|spec| tke::engines::ocr::resolve_ocr(spec, &tke::utils::params::ocr_url()));

    let result = AgentRunner::run(AgentRunOptions {
        case: case_text,
        script_dir,
        ai: merged_ai,
        prompt,
        ocr,
        verify: args.verify,
        params: params.clone(),
    })
    .await
    .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    // 状态/依据/模型/token/元素变更已在「结果」框中统一展示，这里只补产物路径。
    // 未稳定通过的脚本已被删除（不留半成品），只显示日志路径供复盘。
    if result.success {
        eprintln!("  脚本   {}", result.script.display());
    }
    eprintln!("  日志   {}", result.conversation.display());

    std::process::exit(if result.success { 0 } else { 1 });
}
