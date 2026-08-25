// 配置文件加载 - --config <tke.toml>
// config 文件等同于自动输入这些 CLI 参数，显式 CLI 参数优先于 config

use crate::{Result, TkeError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// tke.toml 配置
/// 示例:
///   device = "f64b3b4d"
///   log = "logs"
#[derive(Debug, Default, Deserialize)]
pub struct TkeConfig {
    /// 目标设备 ID
    pub device: Option<String>,
    /// 元素库路径
    /// 产物输出目录
    pub log: Option<PathBuf>,
    /// 脚本输出目录（harness 生成的 .tks 落点；CLI/显式参数优先）
    pub scripts: Option<PathBuf>,
    /// 缓存目录：运行中间文件（截图/页面/会话日志/临时元素库）落点；不设用系统临时目录
    pub cache: Option<PathBuf>,
    /// 工作区目录：AI 文件操作的范围根（.tks/交付文件落点）；不设用进程当前目录。app spawn 用
    pub current_dir: Option<PathBuf>,
    /// 在线 OCR 服务地址（缺省用内置默认；私有部署/换服务时配置）
    pub ocr_url: Option<String>,
    /// OCR 来源模式（harness/run 用）：online / offline / http(s)://... ；CLI --ocr 优先
    pub ocr: Option<String>,
    /// harness 生成脚本后是否自检+自修复（等价 CLI --verify）；CLI --verify 出现则也为 true
    pub verify: Option<bool>,
    /// AI 辅助驾驶（run/flow 回放的定位自愈）：某步元素定位失败时，让 AI 按当前实时页面
    /// 判断"哪个其实就是它"（App 小改版/文案微调/位置变化），当场救活本步并把修正持久化
    /// 回元素包——后续回放直接命中，无须 AI 再介入。默认开启；需配置 [ai] 才真正生效。
    /// CLI --copilot true/false 优先于此。
    pub copilot: Option<bool>,
    /// web 无头模式：auto（默认，按有没有桌面自动判断）/ on（强制无头）/ off（强制有头）。
    /// 无头服务器、docker、CI 里不必配——auto 就会走无头。CLI --headless 优先于此。
    pub headless: Option<String>,
    /// AI 配置（tke harness 探索测试用）：[ai] 段
    #[serde(default)]
    pub ai: AiConfig,
    /// harness 验证/修复各环节的次数上限：[harness] 段
    #[serde(default)]
    pub harness: HarnessConfig,
    /// 记忆/知识库配置：[knowledge] 段（本期留口子，未配置则跳过真实调用）
    #[serde(default)]
    pub knowledge: KnowledgeConfig,
}

/// [harness] 段：探索/验证/修复各环节的次数上限（都可选，缺省见 HarnessLimits 默认）
#[derive(Debug, Deserialize, Default, Clone)]
pub struct HarnessConfig {
    /// 探索失败后「反思+从头重探」的次数上限（默认 1）
    pub reexplore: Option<u32>,
    /// 验证/修复阶段「活体重探(修复)」的次数上限（默认 6）
    pub repairs: Option<u32>,
    /// 稳定性测试需连续通过几次才算稳定（默认 2）
    pub stability: Option<u32>,
    /// 脚本医生单次诊断的轮数上限（默认 10）
    pub doctor_iters: Option<u32>,
}

/// [ai] 段：统一多家大模型的接入参数
/// provider 决定走哪家适配器；doubao/qwen 等国产模型走 OpenAI 兼容端点，用 base_url 指定
#[derive(Debug, Deserialize, Default, Clone)]
pub struct AiConfig {
    /// 服务商：anthropic / openai / gemini / deepseek / doubao / qwen
    pub provider: Option<String>,
    /// 模型名（如 claude-sonnet-4-5、gpt-4o、deepseek-chat）
    pub model: Option<String>,
    /// API Key（也可由各家标准环境变量提供，如 ANTHROPIC_API_KEY）
    pub api_key: Option<String>,
    /// 自定义服务端点。两种用法：① OpenAI 兼容端点（doubao 火山方舟 / qwen 百炼）
    /// ② **AI 网关**——provider 照实写，只换地址，各家原生协议保留
    /// （平台把用户的 key 留在自己那儿，节点只拿短期任务令牌，见 ADR-0023）
    pub base_url: Option<String>,
    /// 探索循环最大轮数上限（防失控烧 token），缺省见 AgentRunner 默认值
    pub max_rounds: Option<u32>,
    /// 自定义提示词目录（约定 agents/*.md、tools/*.md；CLI 同名参数优先）
    pub prompts_dir: Option<String>,
    /// 推理强度（供应商无关，经 genai 映射到各家原生 reasoning：anthropic 思考预算 /
    /// openai o-系列 / gemini thinking / deepseek reasoner）。取值：
    /// none(关) / low / medium / high / xhigh / max / budget:N。缺省 = medium。
    pub reasoning_effort: Option<String>,
}

impl AiConfig {
    /// 用环境变量覆盖 `[ai]`（**优先级在配置文件之上、CLI 显式参数之下**）。
    ///
    /// 为什么要有这条路：远程任务层要**每次任务换一把调用方的 key**（平台把 App 自己的
    /// API Key 交下来，token 计到那个 App 账上，见 ADR-0023 D3 修订）。另外两条路都不合适——
    /// **`--ai-key` 会出现在 `ps aux` 里**（同一台节点上任何人都看得见），
    /// 写临时配置文件则是把密钥落到磁盘上。环境变量是这三者里唯一不留痕的。
    ///
    /// 非密的那几项（provider/model/base_url/reasoning）也一并支持，纯粹是为了成套——
    /// 只让 key 走 env、其余走别的路，调用方要记两套规矩。
    pub fn apply_env(mut self) -> Self {
        fn get(k: &str) -> Option<String> {
            std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
        }
        if let Some(v) = get("TKE_AI_PROVIDER") {
            self.provider = Some(v);
        }
        if let Some(v) = get("TKE_AI_MODEL") {
            self.model = Some(v);
        }
        if let Some(v) = get("TKE_AI_KEY") {
            self.api_key = Some(v);
        }
        if let Some(v) = get("TKE_AI_BASE_URL") {
            self.base_url = Some(v);
        }
        if let Some(v) = get("TKE_AI_REASONING") {
            self.reasoning_effort = Some(v);
        }
        self
    }
}

/// [knowledge] 段：mem0 记忆 + RAG 知识库的远端服务地址
/// 本期两者均为可选；未配置 endpoint 时 AgentRunner 跳过真实调用并在 raw log 中记 skipped
#[derive(Debug, Deserialize, Default, Clone)]
pub struct KnowledgeConfig {
    /// mem0 记忆服务 endpoint（空 = 跳过）
    pub mem0_endpoint: Option<String>,
    /// RAG 知识库检索 endpoint（空 = 跳过）
    pub rag_endpoint: Option<String>,
}

impl TkeConfig {
    /// 加载配置文件；其中的相对路径基于 config 文件所在目录解析
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            TkeError::InvalidArgument(format!("读取配置文件失败 {}: {}", path.display(), e))
        })?;

        let mut config: TkeConfig = toml::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("配置文件解析失败: {}", e)))?;

        // 相对路径基于 config 文件所在目录
        let base = path.parent().unwrap_or(Path::new("."));
        config.log = config.log.map(|p| resolve(base, p));
        config.scripts = config.scripts.map(|p| resolve(base, p));

        Ok(config)
    }
}

fn resolve(base: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() { p } else { base.join(p) }
}

#[cfg(test)]
mod ai_env_tests {
    use super::*;

    /// 环境变量覆盖 `[ai]` —— 远程任务层靠它每次换一把调用方的 key。
    /// **串行跑**（环境变量是进程级的），所以四条断言挤在一个测试里
    #[test]
    fn 环境变量覆盖配置文件里的ai段() {
        let base = AiConfig {
            provider: Some("anthropic".into()),
            model: Some("老模型".into()),
            api_key: Some("节点自己的key".into()),
            ..Default::default()
        };

        // 没设环境变量 → 原样不动
        for k in ["TKE_AI_PROVIDER", "TKE_AI_MODEL", "TKE_AI_KEY", "TKE_AI_BASE_URL", "TKE_AI_REASONING"] {
            std::env::remove_var(k);
        }
        let untouched = base.clone().apply_env();
        assert_eq!(untouched.model.as_deref(), Some("老模型"));
        assert_eq!(untouched.api_key.as_deref(), Some("节点自己的key"));

        // 设了 → 覆盖掉；这正是"平台把 App 的 key 交下来"那一下
        std::env::set_var("TKE_AI_KEY", "调用方的key");
        std::env::set_var("TKE_AI_MODEL", "新模型");
        std::env::set_var("TKE_AI_BASE_URL", "https://ark.example/v1");
        let overridden = base.clone().apply_env();
        assert_eq!(overridden.api_key.as_deref(), Some("调用方的key"));
        assert_eq!(overridden.model.as_deref(), Some("新模型"));
        assert_eq!(overridden.base_url.as_deref(), Some("https://ark.example/v1"));
        assert_eq!(overridden.provider.as_deref(), Some("anthropic"), "没设的那项不该被清掉");

        // 空串等于没设——否则 `TKE_AI_KEY=` 会把节点自己的 key 抹成空的，
        // 报错还是"没有 key"，查起来完全摸不着头脑
        std::env::set_var("TKE_AI_KEY", "   ");
        assert_eq!(base.clone().apply_env().api_key.as_deref(), Some("节点自己的key"));

        for k in ["TKE_AI_PROVIDER", "TKE_AI_MODEL", "TKE_AI_KEY", "TKE_AI_BASE_URL", "TKE_AI_REASONING"] {
            std::env::remove_var(k);
        }
    }
}
