// 测试流程控制器

use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, debug, warn, error};

use crate::core::error::{Result, AiTesterError};
use crate::models::*;
use crate::agents::{receptionist::Receptionist, librarian::Librarian, worker::Worker, supervisor::Supervisor};
use crate::tke_integration::TkeExecutor;
use crate::ocr::{OcrService, OpenAiVisionOcr, XmlProcessor};

/// AI测试控制器
pub struct AiTestController {
    pub project_path: PathBuf,
    pub device_id: String,
    openai_api_key: String,
    tke_executor: TkeExecutor,
    ocr_service: Arc<dyn OcrService>,
    xml_processor: XmlProcessor,
}

impl AiTestController {
    pub fn new(
        project_path: impl AsRef<Path>,
        device_id: String,
        openai_api_key: String,
    ) -> Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();

        let tke_executor = TkeExecutor::new(&project_path, device_id.clone())?;
        let ocr_service: Arc<dyn OcrService> = Arc::new(OpenAiVisionOcr::new(openai_api_key.clone()));
        let xml_processor = XmlProcessor::new();

        Ok(Self {
            project_path,
            device_id,
            openai_api_key,
            tke_executor,
            ocr_service,
            xml_processor,
        })
    }

    /// 执行AI自动化测试
    pub async fn run_ai_test(&self, test_case: TestCase) -> Result<TestResult> {
        info!("开始AI自动化测试: {}", test_case.name);

        // 1. 创建AI Agents
        let receptionist = Receptionist::new(&self.openai_api_key)?;
        let librarian = Librarian::new(&self.project_path)?;
        let supervisor = Supervisor::new(&self.openai_api_key)?;

        // 2. Receptionist 分析测试用例
        info!("Step 1: Receptionist 分析测试用例");
        let analysis = receptionist.analyze_test_case(&test_case).await?;
        debug!("测试分析结果: {}", analysis);

        // 3. Librarian 查找相关知识
        info!("Step 2: Librarian 查找相关知识");
        let knowledge = librarian.search_knowledge(&test_case).await?;
        debug!("找到 {} 条相关知识", knowledge.len());

        // 4. 获取初始屏幕状态
        info!("Step 3: 获取初始屏幕状态");
        let (screenshot_path, xml_path) = self.tke_executor.capture_screen()?;
        let ui_elements = self.xml_processor.parse_ui_xml(&xml_path)?;
        let filtered_elements = self.xml_processor.filter_useful_elements(ui_elements);
        let ocr_results = self.ocr_service.extract_text(&screenshot_path).await?;

        // 5. 创建专用的Worker
        info!("Step 4: 创建专用 Worker");
        let mut worker = Worker::new(
            &self.openai_api_key,
            test_case.clone(),
            analysis,
            knowledge,
        )?;

        // 6. 执行测试循环
        info!("Step 5: 开始测试执行循环");
        let mut action_history = Vec::new();
        let max_actions = 50; // 防止无限循环

        for step in 1..=max_actions {
            info!("执行第 {} 步", step);

            // Worker 决策
            let (action, reasoning) = worker.decide_action(
                filtered_elements.clone(),
                ocr_results.clone(),
            ).await?;

            debug!("Step {}: {:?} - {}", step, action, reasoning);

            // 执行操作
            self.tke_executor.execute_action(&action)?;
            action_history.push(action.clone());

            // 等待一下让界面稳定
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            // 检查是否完成
            if worker.is_test_complete().await? {
                info!("Worker 判断测试已完成");
                break;
            }

            // 获取新的屏幕状态
            let (new_screenshot, new_xml) = self.tke_executor.capture_screen()?;
            let new_ui_elements = self.xml_processor.parse_ui_xml(&new_xml)?;
            let new_filtered_elements = self.xml_processor.filter_useful_elements(new_ui_elements);
            let new_ocr_results = self.ocr_service.extract_text(&new_screenshot).await?;

            // 更新状态供下一轮使用
            // TODO: 这里需要更新 Worker 的状态
        }

        // 7. Supervisor 审核结果
        info!("Step 6: Supervisor 审核测试结果");
        let final_state = self.build_final_state_description().await?;
        let review = supervisor.review_test_execution(
            &test_case,
            &action_history,
            &final_state,
        ).await?;

        // 8. 保存测试脚本
        info!("Step 7: 保存测试脚本");
        let script_path = self.tke_executor.save_script(&test_case.name, &action_history)?;
        info!("测试脚本已保存: {:?}", script_path);

        // 9. 根据审核结果确定测试结果
        let test_result = if review.completed {
            if review.passed {
                TestResult::Completed
            } else {
                TestResult::Failed(review.feedback)
            }
        } else {
            TestResult::Failed(format!("测试未完成: {}", review.feedback))
        };

        info!("AI自动化测试完成: {:?}", test_result);
        Ok(test_result)
    }

    /// 构建最终状态描述
    async fn build_final_state_description(&self) -> Result<String> {
        let (screenshot_path, xml_path) = self.tke_executor.capture_screen()?;
        let ui_elements = self.xml_processor.parse_ui_xml(&xml_path)?;
        let ocr_results = self.ocr_service.extract_text(&screenshot_path).await?;

        let mut description = String::new();
        description.push_str("最终屏幕状态:\n");

        // UI元素摘要
        description.push_str(&format!("UI元素数量: {}\n", ui_elements.len()));
        for elem in ui_elements.iter().take(5) {
            if let Some(text) = &elem.text {
                description.push_str(&format!("- {}: '{}'\n", elem.class, text));
            }
        }

        // OCR结果摘要
        description.push_str(&format!("识别文字数量: {}\n", ocr_results.len()));
        for result in ocr_results.iter().take(5) {
            description.push_str(&format!("- '{}'\n", result.text));
        }

        Ok(description)
    }

    /// 获取设备信息
    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        // TODO: 从TKE获取设备信息
        Ok(DeviceInfo {
            device_id: self.device_id.clone(),
            platform: "Android".to_string(),
            screen_width: 1080,
            screen_height: 1920,
        })
    }

    /// 检查TKE是否可用
    pub fn check_tke_availability(&self) -> Result<bool> {
        // 尝试获取设备列表来验证TKE是否正常工作
        match self.tke_executor.get_devices() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}