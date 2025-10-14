// 主流程控制器 - 协调各个 Agent 和 TKE 执行测试

use crate::agents::{LibrarianAgent, ReceptionistAgent, SupervisorAgent, WorkerAgent};
use crate::models::{RoundLog, TestResult, TestStatus, TesterInput, TesterOutput};
use crate::parser::{ActionTranslator, WorkerParser};
use crate::tke::TkeExecutor;
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

/// AI 测试员主控制器
pub struct TesterOrchestrator {
    /// 输入参数
    input: TesterInput,

    /// TKE 执行器
    tke: TkeExecutor,

    /// Worker Parser
    parser: WorkerParser,

    /// 测试日志
    logs: Vec<RoundLog>,

    /// 已执行的操作历史
    action_history: Vec<String>,

    /// .tks 脚本内容
    tks_script: Vec<String>,
}

impl TesterOrchestrator {
    /// 创建新的控制器
    pub fn new(input: TesterInput) -> Self {
        let tke = TkeExecutor::new(
            &input.tke_path,
            &input.project_path,
            input.device_id.clone(),
        );

        let parser = WorkerParser::new();

        Self {
            input,
            tke,
            parser,
            logs: Vec::new(),
            action_history: Vec::new(),
            tks_script: Vec::new(),
        }
    }

    /// 执行完整测试流程
    pub async fn run(&mut self) -> Result<TesterOutput> {
        let start_time = Utc::now();

        info!("开始 AI 自动化测试: {}", self.input.test_case_name);

        // 1. Receptionist 分析测试用例
        info!("步骤 1: Receptionist 分析测试用例");
        let receptionist = ReceptionistAgent::new();
        let analysis = receptionist
            .analyze_test_case(
                &self.input.test_case_name,
                &self.input.test_case_description,
                &self.input.app_package,
            )
            .await?;

        info!("测试目标: {}", analysis.test_objective);

        // 2. Librarian 检索知识
        info!("步骤 2: Librarian 检索知识库");
        let librarian = LibrarianAgent::new(self.input.knowledge_base_path.clone());
        let knowledge = librarian
            .retrieve_knowledge(&self.input.test_case_description)
            .await?;

        info!("检索到 {} 条相关知识", knowledge.relevant_items.len());

        // 3. 初始化 Worker
        let test_instruction = format!(
            "测试目标: {}\n\n测试步骤建议:\n{}\n\n关注要点:\n{}\n\n预期结果: {}",
            analysis.test_objective,
            analysis
                .suggested_approach
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n"),
            analysis.key_points.join("\n"),
            analysis.expected_outcome
        );

        let worker = WorkerAgent::new(test_instruction.clone(), knowledge.summary.clone());

        // 4. 初始化 Supervisor
        let supervisor = SupervisorAgent::new(analysis.test_objective.clone());

        // 5. 启动应用
        info!("步骤 3: 启动被测应用");
        self.tke
            .launch(&self.input.app_package, &self.input.app_activity)
            .await
            .context("启动应用失败")?;

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 6. 开始测试循环
        info!("步骤 4: 开始测试循环");
        let mut round = 1;
        let mut test_result = TestResult::Incomplete;
        let mut error_message: Option<String> = None;

        while round <= self.input.max_rounds {
            info!("========== 第 {} 轮测试 ==========", round);

            match self.execute_round(&worker, round).await {
                Ok(decision) => {
                    // 检查 Worker 是否认为测试完成
                    if decision.test_completed {
                        info!("Worker 认为测试已完成，提交给 Supervisor 审核");

                        // 获取最近 5 轮的日志
                        let recent_rounds: Vec<String> = self
                            .logs
                            .iter()
                            .rev()
                            .take(5)
                            .rev()
                            .map(|log| {
                                format!(
                                    "轮次 {}: 观察={}, 决策={}, 操作={}",
                                    log.round, log.observation, log.decision, log.action
                                )
                            })
                            .collect();

                        let review = supervisor
                            .review_test(&recent_rounds, &decision.reasoning)
                            .await?;

                        match review {
                            crate::agents::supervisor::SupervisorReview::Incomplete {
                                feedback,
                            } => {
                                warn!("Supervisor 判定测试未完成: {}", feedback);
                                // 继续测试
                            }
                            crate::agents::supervisor::SupervisorReview::PassedNormal {
                                summary,
                            } => {
                                info!("Supervisor 判定测试通过: {}", summary);
                                test_result = TestResult::Passed;
                                break;
                            }
                            crate::agents::supervisor::SupervisorReview::FailedWithBug {
                                bug_description,
                                summary,
                            } => {
                                warn!("Supervisor 判定测试失败 (发现 Bug): {}", bug_description);
                                test_result = TestResult::FailedWithBug;
                                error_message = Some(bug_description);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("第 {} 轮执行失败: {}", round, e);
                    error_message = Some(e.to_string());
                    test_result = TestResult::Error;
                    break;
                }
            }

            round += 1;
        }

        if round > self.input.max_rounds {
            warn!("达到最大轮数限制 ({}), 测试未完成", self.input.max_rounds);
            test_result = TestResult::Incomplete;
        }

        // 7. 保存 .tks 脚本
        info!("步骤 5: 保存 .tks 脚本");
        self.save_tks_script().await?;

        let end_time = Utc::now();

        // 8. 生成输出
        let output = TesterOutput {
            success: matches!(test_result, TestResult::Passed),
            test_case_id: self.input.test_case_id.clone(),
            status: if matches!(test_result, TestResult::Error) {
                TestStatus::Failed
            } else {
                TestStatus::Completed
            },
            result: test_result,
            script_path: self.input.output_script_path.clone(),
            total_rounds: round - 1,
            start_time,
            end_time,
            error: error_message,
            logs: self.logs.clone(),
        };

        info!("AI 自动化测试完成");

        Ok(output)
    }

    /// 执行单轮测试
    async fn execute_round(
        &mut self,
        worker: &WorkerAgent,
        round: u32,
    ) -> Result<crate::models::ActionDecision> {
        let round_start = Utc::now();

        // 1. 截图和获取 UI 树
        let capture_result = self.tke.capture().await?;

        // 2. OCR 识别
        let ocr_result = self
            .tke
            .ocr(&capture_result.screenshot, false, None)
            .await?;

        // 3. 提取 UI 元素
        let ui_elements = self.tke.extract_ui_elements(&capture_result.xml).await?;

        // 4. Worker Parser 解析屏幕
        let screen_state = self.parser.parse_screen(
            ocr_result,
            ui_elements,
            capture_result.screenshot,
            capture_result.xml,
        )?;

        let screen_description = WorkerParser::generate_screen_description(&screen_state);

        // 5. Worker 做出决策
        let decision = worker
            .make_decision(&screen_description, round, &self.action_history)
            .await?;

        info!("Worker 决策: {:?}", decision.action_type);
        info!("决策理由: {}", decision.reasoning);

        // 6. 转换并执行操作
        let tks_line = ActionTranslator::translate_to_tks_script(&decision, &self.parser)?;

        // 执行 TKE 命令 (简化处理)
        // TODO: 根据不同的 ActionType 调用对应的 TKE 方法

        // 7. 记录日志
        let log = RoundLog {
            round,
            timestamp: round_start,
            observation: screen_description,
            decision: decision.reasoning.clone(),
            action: tks_line.clone(),
            action_success: true,
            error: None,
        };

        self.logs.push(log);
        self.action_history.push(format!(
            "[第{}轮] {}",
            round,
            decision.reasoning
        ));
        self.tks_script.push(tks_line);

        Ok(decision)
    }

    /// 保存 .tks 脚本
    async fn save_tks_script(&self) -> Result<()> {
        let script_path = PathBuf::from(&self.input.output_script_path);

        // 创建父目录
        if let Some(parent) = script_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&script_path)
            .await?;

        // 写入脚本头
        file.write_all(
            format!(
                "# AI 自动生成的测试脚本\n# 测试用例: {}\n# 生成时间: {}\n\n",
                self.input.test_case_name,
                Utc::now().format("%Y-%m-%d %H:%M:%S")
            )
            .as_bytes(),
        )
        .await?;

        // 写入脚本内容
        for line in &self.tks_script {
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;

        info!("已保存 .tks 脚本: {}", script_path.display());

        Ok(())
    }
}
