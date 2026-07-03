// 有状态会话：内部持有 genai 客户端 + 累积的对话请求
// 把 genai 的 chat/tool/image 调用全部封在这里，对外只暴露 LlmReply/LlmTool/LlmToolCall。

use std::path::Path;

use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart, ReasoningEffort, Tool, ToolResponse};
use genai::Client;

use crate::utils::AiConfig;
use crate::{Result, TkeError};

use super::client::build_client;
use super::types::{LlmReply, LlmTool, LlmToolCall};

/// 把 `[ai].reasoning_effort` 字符串解析成 genai 的 ReasoningEffort（供应商无关）。
/// 缺省（None）→ Medium（默认开启思考）；显式 "none"/"off" → None（关闭）。
/// 取值：none/off · low · medium · high · xhigh · max · budget:N。无法识别按 Medium。
fn parse_reasoning_effort(spec: Option<&str>) -> Option<ReasoningEffort> {
    let s = match spec {
        None => return Some(ReasoningEffort::Medium), // 缺省即开
        Some(s) => s.trim().to_lowercase(),
    };
    match s.as_str() {
        "none" | "off" | "" => None,
        "low" => Some(ReasoningEffort::Low),
        "medium" => Some(ReasoningEffort::Medium),
        "high" => Some(ReasoningEffort::High),
        "xhigh" => Some(ReasoningEffort::XHigh),
        "max" => Some(ReasoningEffort::Max),
        other => {
            // budget:N → 固定 token 预算
            if let Some(n) = other.strip_prefix("budget:").and_then(|n| n.trim().parse::<u32>().ok()) {
                Some(ReasoningEffort::Budget(n))
            } else {
                Some(ReasoningEffort::Medium)
            }
        }
    }
}

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
    /// 每轮请求选项（含供应商无关的 reasoning effort）。一次会话固定，每次 exec_chat 复用。
    options: ChatOptions,
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
    /// 上一条"诊断 trace"消息在 messages 中的下标（脚本医生用：进入新一轮诊断时压缩旧 trace，
    /// 只保留最新一份完整 trace，历史只剩"改了什么/为什么"的工具结果，防上下文随轮数暴涨）
    last_trace_idx: Option<usize>,
    /// 所有"页面消息"(user_page)在 messages 中的下标（升序）——compact_history 的安全切点：
    /// 页面消息之前必然是上一轮收尾的 tool_result，切在这里不会拆散 tool_call/tool_result 配对
    page_indices: Vec<usize>,
    /// 最近一条「大体积工具结果」(下标, call_id, 占位文本)：追加新的大结果时把上一条原地
    /// 替换成占位摘要，上下文只保留最近一份大 payload（编排官 explore/read_file 返回用）
    last_bulky_tool: Option<(usize, String, String)>,
}

/// 历史页面元素被压缩后的占位文本（塞进 LLM 上下文，故只写对 AI 有用的话；
/// AI 访问不了本地文件，不提 conversation.json）
const PAGE_ELIDED: &str = "【上一轮页面元素已省略，请依据你已执行的步骤与当前页面判断】";

/// 历史截图被压缩后的占位文本（截图只在 AI 当次要图时保留一张，用完即省略）
const IMAGE_ELIDED: &str = "【上一张截图已省略，以当前页面为准】";

/// 历史诊断 trace 被压缩后的占位文本（脚本医生：你已据它做过编辑，见随后的工具结果，以最新 trace 为准）
const TRACE_ELIDED: &str = "【上一轮诊断 trace 的逐步页面详情已省略——你已据它做过编辑（见随后的工具调用与结果），请以下方最新一轮 trace 为准】";

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

        // 推理强度（供应商无关）：缺省 medium；显式 none 则不开。
        // normalize_reasoning_content：把 deepseek/qwen 等 <think>…</think> 抽成 reasoning_content。
        let mut options = ChatOptions::default().with_normalize_reasoning_content(true);
        if let Some(effort) = parse_reasoning_effort(cfg.reasoning_effort.as_deref()) {
            options = options.with_reasoning_effort(effort);
        }

        Ok(Self {
            client,
            model,
            options,
            req,
            last_usage: (0, 0),
            total_usage: (0, 0),
            last_page_idx: None,
            last_image_idx: None,
            last_trace_idx: None,
            page_indices: Vec::new(),
            last_bulky_tool: None,
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
        self.page_indices.push(self.req.messages.len() - 1);
    }

    /// 追加一条"诊断 trace"消息，并把上一轮的同类消息压缩为占位（脚本医生专用）。
    /// 这样 LLM 上下文只保留**当前轮**的完整逐步 trace（含每步页面），历史 trace 的页面详情
    /// 被省略，只剩 AI 的编辑工具调用与结果——既让医生看清当前全貌，又防上下文随诊断轮数暴涨。
    pub fn user_trace(&mut self, text: impl Into<String>) {
        if let Some(i) = self.last_trace_idx {
            if let Some(msg) = self.req.messages.get_mut(i) {
                *msg = ChatMessage::user(TRACE_ELIDED);
            }
        }
        self.req = self.req.clone().append_message(ChatMessage::user(text.into()));
        self.last_trace_idx = Some(self.req.messages.len() - 1);
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

    /// 回填一次**大体积**工具结果（explore 的末页全文、read_file 的文件内容等）：
    /// 追加新的一条时，把上一条大结果原地替换成占位摘要（call_id 不变、配对不破）——
    /// 上下文只保留最近一份大 payload，长 REPL 会话不再随大结果线性膨胀。
    pub fn tool_result_bulky(&mut self, call_id: impl Into<String>, content: impl Into<String>, placeholder: impl Into<String>) {
        if let Some((i, cid, ph)) = self.last_bulky_tool.take() {
            if let Some(msg) = self.req.messages.get_mut(i) {
                *msg = ToolResponse::new(cid, ph).into();
            }
        }
        let call_id: String = call_id.into();
        self.req = self.req.clone().append_message(ToolResponse::new(call_id.clone(), content.into()));
        self.last_bulky_tool = Some((self.req.messages.len() - 1, call_id, placeholder.into()));
    }

    /// 压缩久远历史（探索会话用）：只保留最近 `keep_pages` 轮页面消息以来的完整对话，更早的
    /// （已被 elide 的页面占位、AI 思考、tool_result 等）整体替换成一条确定性 `summary` 消息。
    /// 前导消息（system + 用例开场 + 约束）永远保留。切点永远取某条"页面消息"的位置——它之前
    /// 必然是上一轮收尾的 tool_result，不会拆散 assistant tool_call 与 tool_result 的配对。
    /// 返回是否发生了压缩。防 prompt 随轮数平方级膨胀（每次 next 全量重发历史）。
    pub fn compact_history(&mut self, keep_pages: usize, summary: impl Into<String>) -> bool {
        if self.page_indices.len() <= keep_pages {
            return false;
        }
        let cut = self.page_indices[self.page_indices.len() - keep_pages];
        let preamble_end = self.page_indices[0]; // 第一条页面消息之前 = 前导（system/用例/约束）
        if cut <= preamble_end {
            return false;
        }
        let mut msgs = std::mem::take(&mut self.req.messages);
        let tail = msgs.split_off(cut);
        msgs.truncate(preamble_end);
        msgs.push(ChatMessage::user(summary.into()));
        let new_cut = msgs.len(); // tail 从这里重新开始
        msgs.extend(tail);
        self.req.messages = msgs;
        let shift = cut - new_cut;
        self.page_indices = self.page_indices.iter().filter(|&&i| i >= cut).map(|&i| i - shift).collect();
        let adjust = |v: &mut Option<usize>| {
            *v = match *v {
                Some(i) if i >= cut => Some(i - shift),
                _ => None, // 已被压掉的下标作废
            };
        };
        adjust(&mut self.last_page_idx);
        adjust(&mut self.last_image_idx);
        adjust(&mut self.last_trace_idx);
        match &mut self.last_bulky_tool {
            Some((i, _, _)) if *i >= cut => *i -= shift,
            other => *other = None,
        }
        true
    }

    /// 向模型请求下一步决策
    pub async fn next(&mut self) -> Result<LlmReply> {
        let res = self
            .client
            .exec_chat(&self.model, self.req.clone(), Some(&self.options))
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
