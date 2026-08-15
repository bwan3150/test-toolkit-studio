// Script 工作流 - 执行单个 .tks 脚本
// 逐行执行：实时回调 step_start/step_end 事件（NDJSON 输出由 handler 负责）
// 指定 --log 时每步保存标注截图 + 页面结构文件，结束后写入完整 log.json
// 工作区使用系统缓存临时目录，运行结束即删除；web 会话由脚本的 关闭 指令控制

use crate::{Result, TkeError, ExecutionResult, StepResult, ScriptParser, ScriptInterpreter, Params, TksCommand};
use crate::utils::Workarea;
use super::{RunEvent, RunArtifacts, Layout};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 定位自愈钩子工厂（AI 辅助驾驶的装配形式）：构造 healer 需要元素库 json 路径与脚本原文
/// （分诊上下文），而 .tklib 解包/脚本读取发生在 run() 内部——装配层（CLI）注入工厂，
/// run() 拿到后再造。返回 None = 造不了（如未指定设备），本次回放不带自愈。
pub type HealerFactory = Arc<
    dyn Fn(std::path::PathBuf, &str) -> Option<Arc<dyn super::tks::ElementHealer>> + Send + Sync,
>;

/// 脚本运行器（持有参数表，device/element 查表取得）
pub struct ScriptRunner {
    params: Arc<Params>,
    /// 定位自愈钩子（None=关闭；replay/repair 装配时注入）
    healer: Option<Arc<dyn super::tks::ElementHealer>>,
    /// 自愈钩子工厂（AI 辅助驾驶）：run/flow 装配时注入，解包 .tklib 后延迟构造 healer。
    /// 自愈只救活当次执行 + 在报告里标注，**不回写 .tks / .tklib**（修正落在解包出的临时副本上）
    healer_factory: Option<HealerFactory>,
}

impl ScriptRunner {
    pub fn new(params: Arc<Params>) -> Self {
        Self { params, healer: None, healer_factory: None }
    }

    /// 注入定位自愈钩子（builder 式）
    pub fn with_healer(mut self, healer: Arc<dyn super::tks::ElementHealer>) -> Self {
        self.healer = Some(healer);
        self
    }

    /// 注入自愈钩子工厂（builder 式；AI 辅助驾驶，见 HealerFactory）
    pub fn with_healer_factory(mut self, factory: HealerFactory) -> Self {
        self.healer_factory = Some(factory);
        self
    }

    /// 执行脚本文件
    ///
    /// - log_root: 产物根目录；None 时不保存任何产物（纯执行 + 事件流）
    /// - on_event: 实时事件回调（逐行输出）
    ///
    /// 元素库解析：内部注入（harness 诊断/回放经 with_element_lib）优先；否则要求脚本旁有
    /// **同名 `.tklib` 元素包**（`foo.tks + foo.tklib` 两件套），解包到 cache 使用；
    /// 找不到同名 .tklib 直接报错——没有共享元素库，也没有静默降级。
    pub async fn run(
        &self,
        script_path: &Path,
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        // 解析脚本文件
        let parser = ScriptParser::new();
        let script = parser.parse_file(&script_path.to_path_buf())?;

        let script_stem = script_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("script")
            .to_string();
        let display_path = script_path.to_string_lossy().to_string();

        let runner = if self.params.element_lib().is_some() {
            None // 已内部注入（harness 临时库/解包库），沿用
        } else {
            let tklib = crate::utils::tklib::tklib_path(script_path);
            if !tklib.is_file() {
                return Err(TkeError::InvalidArgument(format!(
                    "缺少元素包 {}：脚本与元素包必须同名同目录（{}.tks + {}.tklib 两件套，一起拷贝即可在任何机器回放）",
                    tklib.display(),
                    script_stem,
                    script_stem
                )));
            }
            let dest = self
                .params
                .cache_root()
                .join("tklib-unpack")
                .join(format!("{}-{}", script_stem, std::process::id()));
            let lib_json = crate::utils::tklib::unpack(&tklib, &dest)?;
            let mut nested = Self::new(Arc::new(self.params.with_element_lib(lib_json.clone())));
            // AI 辅助驾驶：装配层注入了工厂时，以解包出的**临时副本** + 脚本原文（分诊上下文）
            // 构造自愈钩子——AI 的修正只写副本救活当次执行，原 .tks / .tklib 一个字节都不动
            if let Some(factory) = &self.healer_factory {
                let script_text = std::fs::read_to_string(script_path).unwrap_or_default();
                nested.healer = factory(lib_json, &script_text);
            }
            Some(nested)
        };
        let effective = runner.as_ref().unwrap_or(self);

        effective.run_script(script, &display_path, &script_stem, log_root, on_event).await
    }

    /// 不落文件执行一串 .tks 指令（tke steps）
    pub async fn run_lines(
        &self,
        lines: &[String],
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        // 拼成最小脚本交给解析器
        let content = format!("步骤:\n{}", lines.join("\n"));
        let parser = ScriptParser::new();
        let script = parser.parse(&content)?;

        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("没有可执行的有效指令".to_string()));
        }

        // steps = 一次性检查：`--log` 指的就是任务目录，反复调用续写同一份证据
        self.run_script_with(script, "<steps>", "steps", log_root, Layout::Task, on_event).await
    }

    /// 内部统一执行逻辑
    async fn run_script(
        &self,
        script: crate::TksScript,
        display_path: &str,
        script_stem: &str,
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {
        self.run_script_with(script, display_path, script_stem, log_root, Layout::Timestamped, on_event)
            .await
    }

    /// 按指定产物布局执行（run/flow/harness 走 Timestamped，steps 走 Task）
    async fn run_script_with(
        &self,
        script: crate::TksScript,
        display_path: &str,
        script_stem: &str,
        log_root: Option<&Path>,
        layout: Layout,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<ExecutionResult> {

        // 产物目录（仅 --log 时创建）
        let artifacts = match log_root {
            Some(root) => Some(RunArtifacts::create_with(root, script_stem, layout)?),
            None => None,
        };
        let run_dir_str = artifacts
            .as_ref()
            .map(|a| a.run_dir.to_string_lossy().to_string())
            .unwrap_or_default();

        // 3. 临时工作区 + 解释器
        let workarea = Workarea::temp_for_run()?;
        let element = self.params.element_lib();
        let mut interpreter = ScriptInterpreter::new(
            self.params.device(),
            element.as_deref(),
            workarea.clone(),
        )?;
        if let Some(h) = self.healer.clone() {
            interpreter.set_healer(h);
        }

        let start_time = chrono::Local::now().to_rfc3339();

        let mut result = ExecutionResult {
            success: true,
            case_id: script.case_id.clone(),
            script_name: if script.script_name.is_empty() {
                script_stem.to_string()
            } else {
                script.script_name.clone()
            },
            start_time: start_time.clone(),
            end_time: String::new(),
            steps: Vec::new(),
            error: None,
            script_path: Some(display_path.to_string()),
            run_dir: artifacts.as_ref().map(|_| run_dir_str.clone()),
            launched_packages: Vec::new(),
            device: self.params.device(),
        };

        on_event(&RunEvent::RunStart {
            script: display_path.to_string(),
            script_name: result.script_name.clone(),
            total_steps: script.steps.len(),
            run_dir: run_dir_str.clone(),
            start_time,
        });

        // 4. 逐步执行
        // AI 辅助驾驶记账：healer 按发生顺序记自愈成功的元素名，每步执行后 diff 出
        // "这步新增的自愈"，标进 StepResult / StepEnd（报告用，不改脚本资产）
        let mut healed_seen = 0usize;
        for (index, step) in script.steps.iter().enumerate() {
            // 中断检查点：Ctrl+C 后不再继续后续步骤，及时停下（否则一整段回放要跑完才轮到上层检查）
            // 软停（Esc）：用户请求停下当前回放、转入暂停等指导——同样在每步执行前提前停。
            if crate::utils::interrupt::aborted() || crate::utils::interrupt::pause_requested() {
                result.success = false;
                result.error = Some("已中断（用户 Ctrl+C）".to_string());
                break;
            }
            on_event(&RunEvent::StepStart {
                index,
                line: step.line_number,
                command: step.raw.clone(),
            });

            let step_start = Instant::now();
            // 每步硬超时：元素反复重试/页面无响应时不再干等（视觉元素图像匹配尤其慢）。
            // 超时即判该步失败——回放上层据此让 AI 早点介入修复，而不是傻等。
            // 命令感知：滚动查找是「最多 30 次 capture+OCR+滑动」的循环，20s 必然误杀，
            //   会把"目标没找到（页面/文字不对）"伪装成"页面无响应"，诱导医生删掉这条导航。
            //   给它足够预算，让它要么真找到、要么干净返回「滚 N 次仍未出现 X」的明确错。
            //   等待命令自带超时参数，也放宽到不被外层夹掉。
            // 自带时长参数的命令（重试断言 `断言 [x, 存在, 30s]`、`等待 [30s]`）：
            // 外层预算必须 ≥ 它自己的时长 + 余量，否则会被掐死在半路（P-08 的同一类坑）。
            let self_budget_secs = |params: &[crate::TksParam]| -> u64 {
                params
                    .iter()
                    .find_map(|p| match p {
                        crate::TksParam::Duration(ms) => Some((*ms as u64).div_ceil(1000)),
                        _ => None,
                    })
                    .unwrap_or(0)
            };
            let step_timeout_secs: u64 = match step.command {
                TksCommand::ScrollFind => 75,
                TksCommand::Assert | TksCommand::Wait => self_budget_secs(&step.params) + 20,
                // AI 辅助驾驶在场：元素定位失败第 3 次会触发一次 LLM 挑选（oneshot 最多两问），
                // 20s 会把自愈掐死在半路——放宽到 60s。代价是真死页面要多等 40s 才判失败。
                _ if self.healer.is_some() => 60,
                _ => 20,
            };
            let exec_result = match tokio::time::timeout(
                std::time::Duration::from_secs(step_timeout_secs),
                interpreter.interpret_step(step),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => Err(TkeError::DeviceError(format!(
                    "执行超时（>{}s）：元素反复重试或页面无响应",
                    step_timeout_secs
                ))),
            };
            let duration_ms = step_start.elapsed().as_millis() as u64;

            // settle：执行后给页面切换/渲染留时间，避免下一步在动画中/旧页面上执行
            // （元素类步骤的解析还有隐式等待兜底；这里主要稳住纯坐标/滑动后的渲染）
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let (success, error) = match exec_result {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            };
            // 分诊诊断（AI 辅助驾驶）：本步救不活时 healer 给出失败真因分析
            // （前面步骤走偏/路径重构/App 元素消失…）→ 拼进报错，报告/终端可见
            let error = match (error, self.healer.as_ref().and_then(|h| h.take_diagnosis())) {
                (Some(e), Some(d)) => Some(format!("{}\n🩺 AI 分诊：{}", e, d)),
                (e, _) => e,
            };

            // 本步是否发生了定位自愈（AI 辅助驾驶）：diff healer 的顺序记账
            let healed = match &self.healer {
                Some(h) => {
                    let names = h.healed();
                    let new = names.get(healed_seen..).unwrap_or(&[]).join("、");
                    healed_seen = names.len();
                    (!new.is_empty()).then_some(new)
                }
                None => None,
            };

            // 保存本步产物（仅 --log 时）
            let (screenshot, xml) = if let Some(artifacts) = &artifacts {
                // 本步执行中若未采集过页面状态（如纯坐标点击/等待/返回），补采一次，
                // 保证每一步都留有截图和结构文件
                if !interpreter.last_trace.captured {
                    let _ = interpreter.capture_state().await;
                }
                artifacts.save_step(
                    &workarea,
                    index,
                    &interpreter.last_trace,
                    &step.raw,
                    success,
                )
            } else {
                (None, None)
            };

            let step_result = StepResult {
                index,
                command: step.raw.clone(),
                success,
                error: error.clone(),
                duration_ms,
                line: Some(step.line_number),
                screenshot: screenshot.clone(),
                xml: xml.clone(),
                healed: healed.clone(),
                note: step.note.clone(),
            };

            on_event(&RunEvent::StepEnd {
                index,
                line: step.line_number,
                command: step.raw.clone(),
                success,
                error: error.clone(),
                duration_ms,
                screenshot,
                xml,
                healed,
            });

            result.steps.push(step_result);

            // 步骤失败即停止
            if !success {
                result.success = false;
                result.error = error;
                break;
            }

            // 步骤间短暂延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        result.end_time = chrono::Local::now().to_rfc3339();
        result.launched_packages = interpreter.launched_packages.clone();

        // 5. 写入完整运行日志 + 人看的 HTML 报告（仅 --log 时）
        let log_path = match &artifacts {
            Some(a) => match layout {
                // Task：把本批**追加**进任务的 log.json，再整份重建报告。
                // 一个任务始终只有一份 report.html —— 人不必在十几份碎报告之间跳。
                Layout::Task => {
                    let p = a.append_task_log(&result)?.to_string_lossy().to_string();
                    if let Err(e) = crate::workflow::report::write_task_report(&a.run_dir) {
                        tracing::warn!("生成 report.html 失败: {}", e);
                    }
                    p
                }
                Layout::Timestamped => {
                    let p = a.write_log(&result)?.to_string_lossy().to_string();
                    // 报告生成失败不该让整次运行报错——证据本体(log.json/截图)已经落好了
                    if let Err(e) = crate::workflow::report::write_report(&a.run_dir, &result) {
                        tracing::warn!("生成 report.html 失败: {}", e);
                    }
                    // 顺手把父目录的**全流程报告**重建一次（flow/harness 会分子目录跑多份）
                    crate::workflow::report::refresh_session_report(&a.run_dir);
                    p
                }
            },
            None => String::new(),
        };

        // 6. 清理临时工作区
        // 注: web 会话不自动销毁——生命周期由脚本的 `关闭` 指令控制,
        //     不写则保留会话供后续脚本/指令复用; 新建会话时会收割历史孤儿
        workarea.cleanup();

        on_event(&RunEvent::RunEnd {
            success: result.success,
            total_steps: result.steps.len(),
            successful_steps: result.steps.iter().filter(|s| s.success).count(),
            error: result.error.clone(),
            run_dir: run_dir_str,
            log: log_path,
            // AI 辅助驾驶汇总：哪些步是 AI 找回定位才通过的（提示脚本定位在新版 App 上已失效）
            healed: result
                .steps
                .iter()
                .filter_map(|s| s.healed.as_ref().map(|h| format!("第{}步「{}」", s.index + 1, h)))
                .collect(),
        });

        Ok(result)
    }
}

/// 校验脚本文件存在且为 .tks
pub fn validate_script_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(TkeError::InvalidArgument(format!(
            "脚本文件不存在: {}",
            path.display()
        )));
    }
    if path.extension().and_then(|s| s.to_str()) != Some("tks") {
        return Err(TkeError::InvalidArgument(format!(
            "不是 .tks 脚本文件: {}",
            path.display()
        )));
    }
    Ok(())
}
