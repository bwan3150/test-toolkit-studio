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
    /// 脚本输出目录（生成的 .tks 落点；文件名由 AI 在 finish 时起，落库去重不覆盖旧脚本）
    pub script_dir: PathBuf,
    /// AI 配置（已合并 CLI --ai-* 覆盖）
    pub ai: AiConfig,
    /// 提示词来源（可自定义：注入文本 / .md 文件 / 目录）
    pub prompt: PromptSpec,
    /// OCR 增强来源（None=不跑 OCR，行为同此前；--ocr 显式开启）
    pub ocr: Option<OcrSource>,
    /// 生成脚本后自检+自修复：重启净化→整脚本 tke run 回放→失败让 AI 从失败步续接修复，
    /// 直到连续通过 2 次。`--verify` 显式开启（默认关）。
    pub verify: bool,
    /// 统一参数表（device/element/log/knowledge 查表取参）
    pub params: Arc<Params>,
}

/// AgentResult 里附带的自检结果（仅 --verify 时有意义）
#[derive(Default)]
pub struct VerifyReport {
    /// 是否跑了自检
    pub ran: bool,
    /// 最终是否稳定通过（连续 2 次干净回放）
    pub passed: bool,
    /// 触发的修复次数
    pub repairs: usize,
    /// 修复阶段新建的元素名（与探索阶段合并后展示）
    pub created: Vec<String>,
    /// 修复阶段更新描述的元素差异行
    pub updated: Vec<String>,
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
