// 有状态会话：内部持有 genai 客户端 + 累积的对话请求
// 把 genai 的 chat/tool/image 调用全部封在这里，对外只暴露 LlmReply/LlmTool/LlmToolCall。

use std::path::Path;

use genai::chat::{ChatMessage, ChatRequest, ContentPart, Tool, ToolResponse};
use genai::Client;

use crate::utils::AiConfig;
use crate::{Result, TkeError};

use super::client::build_client;
use super::types::{LlmReply, LlmTool, LlmToolCall};

/// 一次 AI 探索会话：内部持有 genai 客户端 + 累积的对话请求
///
/// 典型用法（探索循环）：
/// ```ignore
/// let mut sess = LlmSession::new(&cfg.ai, system_prompt, tools)?;
/// sess.user(test_case);
/// loop {
///     match sess.next().await? {
///         LlmReply::ToolCalls(calls) => {
///             for c in &calls {
///                 let result = execute(c);
///                 sess.tool_result(&c.call_id, result);
///             }
///         }
///         LlmReply::Text(_) => { /* 结束或澄清 */ }
///     }
/// }
/// ```
pub struct LlmSession {
    client: Client,
    model: String,
    /// 累积的对话请求（genai 以"整段 messages"为请求单位，逐轮 append）
    req: ChatRequest,
}

impl LlmSession {
    /// 从 [ai] 配置构建会话：注入 system 提示词与工具集
    pub fn new(cfg: &AiConfig, system: impl Into<String>, tools: Vec<LlmTool>) -> Result<Self> {
        let (client, model) = build_client(cfg)?;

        // 工具抽象 → genai Tool
        let genai_tools: Vec<Tool> = tools
            .into_iter()
            .map(|t| {
                Tool::new(t.name)
                    .with_description(t.description)
                    .with_schema(t.schema)
            })
            .collect();

        let system: String = system.into();
        let mut req = ChatRequest::new(vec![ChatMessage::system(system)]);
        if !genai_tools.is_empty() {
            req = req.with_tools(genai_tools);
        }

        Ok(Self { client, model, req })
    }

    /// 当前使用的模型名（日志用）
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 追加一条用户消息（纯文本）：投喂用例、页面元素列表、用户答复等
    pub fn user(&mut self, text: impl Into<String>) {
        let text: String = text.into();
        self.req = self.req.clone().append_message(ChatMessage::user(text));
    }

    /// 追加一条用户消息 + 图片：用于"AI 主动要图 → 系统真实传图"
    pub fn user_with_image(&mut self, text: &str, image_path: &Path) -> Result<()> {
        let img = ContentPart::from_binary_file(image_path).map_err(|e| {
            TkeError::LlmError(format!("加载截图失败 {}: {}", image_path.display(), e))
        })?;
        let msg = ChatMessage::user(vec![ContentPart::from_text(text.to_string()), img]);
        self.req = self.req.clone().append_message(msg);
        Ok(())
    }

    /// 回填一次工具调用的执行结果（与某个 call_id 配对），供模型下一轮参考
    pub fn tool_result(&mut self, call_id: impl Into<String>, content: impl Into<String>) {
        let call_id: String = call_id.into();
        let content: String = content.into();
        let resp = ToolResponse::new(call_id, content);
        self.req = self.req.clone().append_message(resp);
    }

    /// 向模型请求下一步决策
    pub async fn next(&mut self) -> Result<LlmReply> {
        let res = self
            .client
            .exec_chat(&self.model, self.req.clone(), None)
            .await
            .map_err(|e| TkeError::LlmError(e.to_string()))?;

        // first_text 借用 res，必须在 into_tool_calls 消费前取出
        let text = res.first_text().map(|s| s.to_string());
        let tool_calls = res.into_tool_calls();

        if tool_calls.is_empty() {
            return Ok(LlmReply::Text(text.unwrap_or_default()));
        }

        // 把 assistant 的工具调用作为一条消息回填进会话历史
        self.req = self.req.clone().append_message(tool_calls.clone());

        let mapped = tool_calls
            .into_iter()
            .map(|tc| LlmToolCall {
                call_id: tc.call_id,
                name: tc.fn_name,
                arguments: tc.fn_arguments,
            })
            .collect();
        Ok(LlmReply::ToolCalls(mapped))
    }
}
