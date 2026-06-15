// AgentRunner 的入参 / 结果

use std::path::PathBuf;

use crate::{AiConfig, KnowledgeConfig};

use super::super::prompt::PromptSpec;

/// AgentRunner 入参
pub struct AgentRunOptions {
    /// 测试用例文字（已从 .md 文件或命令行读出）
    pub case: String,
    /// 目标设备
    pub device: String,
    /// 元素库路径（None 则默认放在 script 同级的 element.json）
    pub element: Option<PathBuf>,
    /// 生成的 .tks 导出路径
    pub script_out: PathBuf,
    /// 产物根目录（预留；当前 conversation/screens 落在 script 同级）
    pub log: Option<PathBuf>,
    /// AI 配置
    pub ai: AiConfig,
    /// 记忆/知识库配置
    pub knowledge: KnowledgeConfig,
    /// 提示词来源（可自定义：注入文本 / .md 文件 / 目录）
    pub prompt: PromptSpec,
}

/// AgentRunner 结果
pub struct AgentResult {
    pub success: bool,
    pub rounds: usize,
    pub script: PathBuf,
    pub conversation: PathBuf,
    pub finish_reason: String,
}
