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
    /// 目标平台覆盖（向导/--platform）；None 则由设备推断
    pub platform: Option<crate::models::Platform>,
    /// 设备覆盖（向导选的）；None 则用 params.device()
    pub device: Option<String>,
    /// 统一参数表（device/element/log/knowledge 查表取参）
    pub params: Arc<Params>,
}

impl AgentRunOptions {
    /// 工具级设备覆盖（ADR-0011）：返回一个把 device/platform 换成指定设备的副本。
    ///
    /// 设备不再是"启动时选定、整场不变"的会话级属性——编排官每次调设备类工具时按任务语义
    /// 指定跑在哪台上（"在 A 手机改设置，再去 B 手机看"）。平台按该设备重新推断，
    /// 否则会拿着上一台的平台去操作新设备。
    pub fn with_device(&self, device: &str) -> Self {
        Self {
            case: self.case.clone(),
            script_dir: self.script_dir.clone(),
            ai: self.ai.clone(),
            prompt: self.prompt.clone(),
            ocr: self.ocr.clone(),
            verify: self.verify,
            platform: Some(crate::models::Platform::from_device(Some(device))),
            device: Some(device.to_string()),
            params: self.params.clone(),
        }
    }

    /// 当前生效的设备（工具未指定时的默认）：显式覆盖 > `-d` > 空
    pub fn effective_device(&self) -> Option<String> {
        self.device.clone().or_else(|| self.params.device())
    }
}

/// AgentResult 里附带的自检结果（仅 --verify 时有意义）
#[derive(Default)]
pub struct VerifyReport {
    /// 是否跑了自检
    pub ran: bool,
    /// 最终是否稳定通过（稳定性测试连续 NEED_PASS 次干净回放都到达目标）
    pub passed: bool,
    /// 触发的活体重探次数
    pub repairs: usize,
    /// 【诊断】脚本医生是否把脚本修到「至少能跑通并到达目标」一次（false=修复失败）
    pub reached: bool,
    /// 【诊断】是否因达到医生诊断轮数上限而停（true=优化达上限：仍可跑、未必最短）
    pub hit_iter_limit: bool,
    /// 【诊断】最终脚本步数（落盘版本，含结尾关闭步）
    pub final_steps: usize,
    /// 【验证】稳定性测试实际连续通过的次数
    pub stability_passes: usize,
    /// 修复阶段新建的元素名（与探索阶段合并后展示）
    pub created: Vec<String>,
    /// 修复阶段更新描述的元素差异行
    pub updated: Vec<String>,
    /// 脚本医生（独立会话）消耗的输入 token（探索会话外，需并入总量统计）
    pub extra_prompt: i64,
    /// 脚本医生（独立会话）消耗的输出 token
    pub extra_completion: i64,
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
