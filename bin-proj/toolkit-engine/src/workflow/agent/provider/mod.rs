// 【对接层】统一多家大模型（基于 genai crate）
//
// 把 genai 的所有具体类型/调用**全部隔离在本子模块内**，对外只暴露干净抽象：
//   types    LlmTool / LlmToolCall / LlmReply（不泄漏任何 genai 类型）
//   client   genai 客户端构建（provider → 适配器/认证/端点选择）
//   session  LlmSession：有状态会话（逐轮 chat / tool_call / image）
//
// provider 取值（来自 [ai].provider，缺省 anthropic）：
//   anthropic / openai / gemini / deepseek  —— 直连官方端点
//   doubao / qwen                           —— OpenAI 兼容端点，需配 [ai].base_url

pub mod client;
pub mod session;
pub mod types;

pub use session::LlmSession;
pub use types::{LlmReply, LlmTool, LlmToolCall};
