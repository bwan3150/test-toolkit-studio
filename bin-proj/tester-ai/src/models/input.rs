// 输入模型 - 定义接收的 JSON 结构

use serde::{Deserialize, Serialize};

/// AI 测试员主输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesterInput {
    /// 测试用例 ID
    pub test_case_id: String,

    /// 测试用例名称
    pub test_case_name: String,

    /// 测试用例描述
    pub test_case_description: String,

    /// 被测试的 App 包名
    pub app_package: String,

    /// 被测试的 App Activity
    pub app_activity: String,

    /// 项目路径
    pub project_path: String,

    /// workarea 路径
    pub workarea_path: String,

    /// 知识库路径
    pub knowledge_base_path: String,

    /// elements.json 路径
    pub elements_path: String,

    /// 输出的 .tks 脚本路径
    pub output_script_path: String,

    /// AI 配置
    pub ai_config: AiConfig,

    /// TKE 可执行文件路径
    pub tke_path: String,

    /// 设备 ID
    pub device_id: Option<String>,

    /// 最大测试轮数
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
}

fn default_max_rounds() -> u32 {
    50
}

/// AI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// LLM 提供商 (openai, anthropic, etc.)
    pub provider: String,

    /// API Key
    pub api_key: String,

    /// 模型名称
    pub model: String,

    /// API 基础 URL (可选)
    pub base_url: Option<String>,

    /// 温度参数
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_temperature() -> f32 {
    0.7
}
