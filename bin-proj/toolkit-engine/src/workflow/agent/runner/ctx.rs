// 【驱动上下文/结果】DriveCtx（驱动循环所需的只读环境）与 DriveOutcome（一轮驱动的产出）。
// 从 flow.rs 拆出——它们被 testrun/tksops/reflect/doctor 等广泛借用，是跨模块的数据契约。

use std::path::Path;

use crate::engines::ocr::OcrSource;
use crate::{Fetcher, RunArtifacts, StepResult, Workarea};

use super::super::prompt::PromptSet;
use super::super::ui::Frontend;

/// 循环所需的只读上下文
pub struct DriveCtx<'a> {
    pub device: &'a str,
    pub element_path: &'a Path,
    pub workarea: &'a Workarea,
    pub fetcher: &'a Fetcher,
    pub artifacts: &'a RunArtifacts,
    /// OCR 增强来源（None=不跑 OCR，行为同此前）
    pub ocr: Option<&'a OcrSource>,
    pub max_rounds: usize,
    /// 提示词集合：运行时消息模板（每轮页面/各类提示/截图等）从这里取，可外部覆盖
    pub prompts: &'a PromptSet,
    /// 用户原始测试用例（finish 终点校验：对照原话需求判断是否真到对的地方）
    pub case: &'a str,
    /// AI 配置：供 finish 时新建独立的「监督官」会话审查（与探索会话分开）
    pub ai: &'a crate::AiConfig,
    /// UI 前端：引擎的所有渲染事件经 emit 发出，安全点 drain_commands 取命令
    pub ui: &'a dyn Frontend,
    /// 通用任务模式（operate）：朝任意目标驱动设备、不产可回放脚本——跳过测试专属的
    /// 踩实官(自动断言)与监督官(finish 把关)。false=测试探索（行为不变）。
    pub task_mode: bool,
}

/// 循环结果
#[derive(Default)]
pub struct DriveOutcome {
    pub success: bool,
    pub reason: String,
    pub lines: Vec<String>,
    pub steps: Vec<StepResult>,
    pub rounds: usize,
    /// 本轮新增的元素名（人工审核用）
    pub created: Vec<String>,
    /// 本轮更新了描述的元素名（人工审核用）
    pub updated: Vec<String>,
    /// 是否被用户中断（Ctrl+C）
    pub aborted: bool,
    /// AI 在 finish 时给生成脚本起的简短文件名（不含扩展名）；None 则由上层兜底
    pub script_name: Option<String>,
    /// 本次发生的元素改名 (旧名, 新名)，供上层把前缀脚本里的引用同步改掉
    pub renames: Vec<(String, String)>,
    /// 每个落库步骤当时 AI 给的 comment（理由），与 lines 一一对应；供反思 agent 复盘"怎么走成这样"
    pub step_comments: Vec<String>,
    /// 子 agent(踩实官 + finish 监督官)独立会话消耗的 token 合计，供上层并入总量
    pub subagent_pt: i64,
    pub subagent_ct: i64,
}
