// AgentRunner 的入参 / 结果

use std::path::PathBuf;
use std::sync::Arc;

use crate::engines::ocr::OcrSource;
use crate::{AiConfig, Params};

use super::super::prompt::PromptSpec;

/// AgentRunner 入参
/// device/element/log/knowledge 经 params 查表取得；ai 为 harness 合并 --ai-* 覆盖后的结果
pub struct AgentRunOptions {
    /// 测试用例文字（已从 .md 文件或命令行读出）
    pub case: String,
    /// 生成的 .tks 导出路径
    pub script_out: PathBuf,
    /// AI 配置（已合并 CLI --ai-* 覆盖）
    pub ai: AiConfig,
    /// 提示词来源（可自定义：注入文本 / .md 文件 / 目录）
    pub prompt: PromptSpec,
    /// OCR 增强来源（None=不跑 OCR，行为同此前；--ocr 显式开启）
    pub ocr: Option<OcrSource>,
    /// 统一参数表（device/element/log/knowledge 查表取参）
    pub params: Arc<Params>,
}

/// AgentRunner 结果
pub struct AgentResult {
    pub success: bool,
    pub rounds: usize,
    pub script: PathBuf,
    pub conversation: PathBuf,
    pub finish_reason: String,
    /// 是否被用户中断（Ctrl+C）
    pub aborted: bool,
}
