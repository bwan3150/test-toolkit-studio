// 对外抽象类型：上层只见这些，**不接触任何 genai 类型**

/// 一个暴露给模型的工具定义
#[derive(Debug, Clone)]
pub struct LlmTool {
    /// 工具名（即 control 动作名 / finish / ask_user / request_screenshot 等）
    pub name: String,
    /// 给模型看的说明：这个工具做什么、何时用
    pub description: String,
    /// 入参的 JSON Schema（object，含 properties / required）
    pub schema: serde_json::Value,
}

impl LlmTool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            schema,
        }
    }
}

/// 模型发起的一次工具调用
#[derive(Debug, Clone)]
pub struct LlmToolCall {
    /// 本次调用的唯一 id（回填工具结果时需原样带回）
    pub call_id: String,
    /// 被调用的工具名
    pub name: String,
    /// 调用参数（已解析为 JSON）
    pub arguments: serde_json::Value,
}

/// 模型一轮回复：工具调用（一次可能多个）或纯文本
#[derive(Debug, Clone)]
pub enum LlmReply {
    /// 模型决定调用工具（探索循环的主路径）
    /// text 为模型在调用工具时**同时**给出的思考/说明（多数模型可有，可空），
    /// 用于在 CLI 实时展示"AI 在想什么"。
    ToolCalls { text: Option<String>, calls: Vec<LlmToolCall> },
    /// 模型给出纯文本（如澄清说明、未走工具时的兜底）
    Text(String),
}
