// 数据模型定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub project_path: String,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub platform: String,
    pub screen_width: u32,
    pub screen_height: u32,
}

/// UI元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub index: usize,
    pub class: String,
    pub text: Option<String>,
    pub resource_id: Option<String>,
    pub content_desc: Option<String>,
    pub bounds: [i32; 4], // [x1, y1, x2, y2]
    pub clickable: bool,
    pub scrollable: bool,
}

/// OCR结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub bounds: [i32; 4],
}

/// 测试操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestAction {
    Tap { x: i32, y: i32 },
    Swipe { x1: i32, y1: i32, x2: i32, y2: i32, duration: u32 },
    Input { text: String },
    Back,
    Home,
    Wait { seconds: u32 },
    Screenshot,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestResult {
    InProgress,
    Completed,
    Failed(String),
}

/// 测试会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSession {
    pub id: Uuid,
    pub test_case: TestCase,
    pub device: DeviceInfo,
    pub actions: Vec<TestAction>,
    pub result: TestResult,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Supervisor 审核结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorReview {
    pub completed: bool,
    pub passed: bool,
    pub feedback: String,
}

/// Agent间的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    TestCaseAnalysis {
        test_case: TestCase,
        analysis: String,
    },
    KnowledgeFound {
        test_case_id: Uuid,
        knowledge: Vec<String>,
    },
    ActionDecision {
        action: TestAction,
        reasoning: String,
    },
    SupervisorReview {
        completed: bool,
        passed: bool,
        feedback: String,
    },
}