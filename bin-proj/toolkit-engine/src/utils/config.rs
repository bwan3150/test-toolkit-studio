// 配置文件加载 - --config <tke.toml>
// config 文件等同于自动输入这些 CLI 参数，显式 CLI 参数优先于 config

use crate::{Result, TkeError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// tke.toml 配置
/// 示例:
///   device = "f64b3b4d"
///   element = "locator/element.json"   # 相对路径基于 config 文件所在目录
///   log = "logs"
#[derive(Debug, Default, Deserialize)]
pub struct TkeConfig {
    /// 目标设备 ID
    pub device: Option<String>,
    /// 元素库路径
    pub element: Option<PathBuf>,
    /// 产物输出目录
    pub log: Option<PathBuf>,
    /// 脚本输出目录（harness 生成的 .tks 落点；CLI/显式参数优先）
    pub scripts: Option<PathBuf>,
    /// 在线 OCR 服务地址（缺省用内置默认；私有部署/换服务时配置）
    pub ocr_url: Option<String>,
    /// AI 配置（tke harness 探索测试用）：[ai] 段
    #[serde(default)]
    pub ai: AiConfig,
    /// 记忆/知识库配置：[knowledge] 段（本期留口子，未配置则跳过真实调用）
    #[serde(default)]
    pub knowledge: KnowledgeConfig,
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
    /// 自定义服务端点（OpenAI 兼容端点专用：doubao 火山方舟 / qwen 百炼）
    pub base_url: Option<String>,
    /// 探索循环最大轮数上限（防失控烧 token），缺省见 AgentRunner 默认值
    pub max_rounds: Option<u32>,
    /// 自定义提示词目录（约定 agents/*.md、tools/*.md；CLI 同名参数优先）
    pub prompts_dir: Option<String>,
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
        config.element = config.element.map(|p| resolve(base, p));
        config.log = config.log.map(|p| resolve(base, p));
        config.scripts = config.scripts.map(|p| resolve(base, p));

        Ok(config)
    }
}

fn resolve(base: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() { p } else { base.join(p) }
}
