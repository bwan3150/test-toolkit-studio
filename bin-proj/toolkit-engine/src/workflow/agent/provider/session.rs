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
///         LlmReply::ToolCalls { calls, .. } => {
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
    /// 最近一次 next() 的 token 用量 (上行/输入, 下行/输出)
    last_usage: (i64, i64),
    /// 本会话累计 token 用量 (上行总, 下行总)
    total_usage: (i64, i64),
    /// 上一条"页面元素列表"消息在 messages 中的下标（用于进入新一轮时压缩它）
    last_page_idx: Option<usize>,
    /// 上一条"截图"消息在 messages 中的下标（用于只保留 AI 当前请求的那一张）
    last_image_idx: Option<usize>,
}

/// 历史页面元素被压缩后的占位文本（塞进 LLM 上下文，故只写对 AI 有用的话；
/// AI 访问不了本地文件，不提 conversation.json）
const PAGE_ELIDED: &str = "【上一轮页面元素已省略，请依据你已执行的步骤与当前页面判断】";

/// 历史截图被压缩后的占位文本（截图只在 AI 当次要图时保留一张，用完即省略）
const IMAGE_ELIDED: &str = "【上一张截图已省略，以当前页面为准】";

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

        Ok(Self {
            client,
            model,
            req,
            last_usage: (0, 0),
            total_usage: (0, 0),
            last_page_idx: None,
            last_image_idx: None,
        })
    }

    /// 当前使用的模型名（日志用）
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 最近一次 next() 的 token 用量 (上行/输入, 下行/输出)
    pub fn last_usage(&self) -> (i64, i64) {
        self.last_usage
    }

    /// 本会话累计 token 用量 (上行总, 下行总)
    pub fn total_usage(&self) -> (i64, i64) {
        self.total_usage
    }

    /// 追加一条用户消息（纯文本）：投喂用例、用户答复、提示等
    pub fn user(&mut self, text: impl Into<String>) {
        let text: String = text.into();
        self.req = self.req.clone().append_message(ChatMessage::user(text));
    }

    /// 追加一条"页面元素列表"消息，并把上一轮的同类消息压缩为占位。
    /// 这样 LLM 上下文只保留**当前轮**的完整页面元素，历史只剩 AI 的思考/决策与执行步骤，
    /// 避免上下文随轮数暴涨、也避免历史页面干扰判断。conversation.json 仍完整记录（独立于此）。
    pub fn user_page(&mut self, text: impl Into<String>) {
        if let Some(i) = self.last_page_idx {
            if let Some(msg) = self.req.messages.get_mut(i) {
                *msg = ChatMessage::user(PAGE_ELIDED);
            }
        }
        // 进入新一轮：上一轮 AI 请求的截图也已用完，一并压缩（上下文不留旧图）
        if let Some(i) = self.last_image_idx.take() {
            if let Some(msg) = self.req.messages.get_mut(i) {
                *msg = ChatMessage::user(IMAGE_ELIDED);
            }
        }
        self.req = self.req.clone().append_message(ChatMessage::user(text.into()));
        self.last_page_idx = Some(self.req.messages.len() - 1);
    }

    /// 追加一条用户消息 + 图片：用于"AI 主动要图 → 系统真实传图"。
    /// 同轮多次要图时压缩上一张，LLM 上下文只保留 AI 当前请求的这一张截图。
    pub fn user_with_image(&mut self, text: &str, image_path: &Path) -> Result<()> {
        let img = ContentPart::from_binary_file(image_path).map_err(|e| {
            TkeError::LlmError(format!("加载截图失败 {}: {}", image_path.display(), e))
        })?;
        if let Some(i) = self.last_image_idx {
            if let Some(msg) = self.req.messages.get_mut(i) {
                *msg = ChatMessage::user(IMAGE_ELIDED);
            }
        }
        let msg = ChatMessage::user(vec![ContentPart::from_text(text.to_string()), img]);
        self.req = self.req.clone().append_message(msg);
        self.last_image_idx = Some(self.req.messages.len() - 1);
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

        // token 用量：累计 + 记录本次（部分 provider 可能不返回，按 0 处理）
        let pt = res.usage.prompt_tokens.unwrap_or(0) as i64;
        let ct = res.usage.completion_tokens.unwrap_or(0) as i64;
        self.last_usage = (pt, ct);
        self.total_usage.0 += pt;
        self.total_usage.1 += ct;

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
        // text：模型调工具时同时给的思考文字（可空），供 CLI 展示
        Ok(LlmReply::ToolCalls { text, calls: mapped })
    }
}
