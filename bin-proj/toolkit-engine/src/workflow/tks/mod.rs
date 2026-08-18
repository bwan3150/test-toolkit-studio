// tks 模块 - .tks 脚本引擎：解析器 + 解释器 + 单步执行器（编辑器调试用）
// 完整脚本/flow 的执行在 workflow 模块（带实时事件和产物）。
// 命名注意：本模块与 workflow/agent/runner(AI 编排层)是两回事——曾同名 "runner" 极易混淆，故改名 tks。

// 子模块
mod parser;
mod interpreter;

// 导出
pub use parser::{ScriptParser, script_to_source, step_to_source};
pub use interpreter::{ScriptInterpreter, ActionTrace};

/// 【定位自愈钩子】元素解析失败若干次后回调（Healenium 式 self-healing）：
/// 实现方（agent 层）基于**当前工作区里最新的页面采集**（解析器每次重试都刚采过）
/// 判断"页面上哪个元素其实就是它"（改版/文字微调/位置变化），返回该元素的实时坐标与
/// 边界框——本步当场救活；实现方同时负责把修正**持久化到元素库文件**（供以后的回放）。
/// None = 救不了（页面上确无对应物），解析器继续按原路径失败。
#[async_trait::async_trait]
pub trait ElementHealer: Send + Sync {
    async fn heal(&self, element_name: &str, workarea: &crate::utils::Workarea) -> Option<(crate::Point, crate::Bounds)>;

    /// 本次运行自愈成功的元素名（按发生顺序）。上层用它做两件事：
    /// ① 逐步提示"这步是 AI 救活的"② 运行结束后把修正过的元素库回包 .tklib。
    /// 默认空 = 实现方不做自愈记账。
    fn healed(&self) -> Vec<String> {
        Vec::new()
    }

    /// 分诊诊断：heal 救不活时，对失败真正原因的分析（前面步骤走偏/路径整体重构/
    /// App 元素消失或不可交互…）。上层在该步最终失败时取走、拼进报错让人看到。
    /// **取走即清空**——诊断只归属当前失败的这一步，不得串到后续步骤。默认 None。
    fn take_diagnosis(&self) -> Option<String> {
        None
    }
}

use crate::{Result, TkeError, StepResult};
use crate::utils::Workarea;
use std::path::Path;
use std::time::Instant;

/// 单步执行器（编辑器调试用）
pub struct TksRunner {
    device_id: Option<String>,
    pub parser: ScriptParser,
}

impl TksRunner {
    pub fn new(device_id: Option<String>) -> Self {
        Self {
            device_id,
            parser: ScriptParser::new(),
        }
    }

    /// 运行单行脚本指令
    /// element_path: 元素库路径（None 按默认路径查找）
    pub async fn run_single_step(
        &mut self,
        line: &str,
        element_path: Option<&Path>,
    ) -> Result<StepResult> {
        // 构造一个最小的脚本来解析单行指令
        let minimal_script = format!("用例: 单步执行\n脚本名: 单步\n\n步骤:\n{}", line);

        // 解析脚本
        let script = self.parser.parse(&minimal_script)?;

        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("无效的脚本指令".to_string()));
        }

        // 单步执行使用设备缓存工作区（与 fetch/recognize 共享）
        let workarea = Workarea::for_device(self.device_id.as_deref())?;

        // 初始化解释器
        let mut interpreter = ScriptInterpreter::new(
            self.device_id.clone(),
            element_path,
            workarea,
        )?;

        let step = &script.steps[0];
        let start_time = Instant::now();

        // 执行单个步骤
        let outcome = interpreter.interpret_step(step).await;
        // 对话框探测：AI 逐步探索时同样会撞上,而它不在 DOM 里、下一步必然失败（P-37）
        let dialog = interpreter.dialog_text();
        match outcome {
            Ok(()) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: true,
                error: None,
                duration_ms: start_time.elapsed().as_millis() as u64,
                line: None,
                screenshot: None,
                xml: None,
                healed: None,
                note: None,
                dialog: dialog.clone(),
            }),
            Err(e) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: false,
                error: Some(e.to_string()),
                duration_ms: start_time.elapsed().as_millis() as u64,
                line: None,
                screenshot: None,
                xml: None,
                healed: None,
                note: None,
                dialog,
            })
        }
    }
}
