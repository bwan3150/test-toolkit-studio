// 【对话日志】AI 对话原始日志（conversation.jsonl）
//
// 把一次 tke harness 探索的完整可复盘过程逐行写成 JSONL：
//   系统提示词 / 用例 / 配置 / 记忆·知识库命中 / 反问·答复 /
//   每轮: 页面快照 → LLM 请求 → LLM 决策 → 要图·传图 → 落库 → .tks 步骤 → 执行结果 / 结束依据
// 设计：每条事件即写即 flush，中途崩溃也能留下已发生的部分。

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{Result, TkeError};

/// 对话日志写入器
/// 双输出：① conversation.jsonl —— 即写即 flush 的追加式流式日志（抗崩溃）；
///        ② finalize() 结束时另导出 conversation.json —— 缩进美化的数组，给人阅读。
pub struct Transcript {
    path: PathBuf,
    file: File,
    /// 内存累积所有事件，用于结束时导出美化 JSON
    events: Vec<serde_json::Value>,
    /// agent 作用域栈：栈顶 = 当前正在执行的 agent。每条事件按栈顶归属（多 agent 框架的
    /// 核心——归属来自"现在是哪个 agent 在跑"这一显式上下文，不靠按事件猜测）。
    /// 进入某 agent 的执行用 `scoped()` 压栈、Drop 自动弹栈，嵌套(doctor→reexplore→explorer)天然由栈处理。
    /// 栈空 = 还没进入任何 agent，归属为编排层 "harness"。
    agent_stack: Vec<String>,
}

/// agent 执行作用域守卫：创建即压栈、Drop 即弹栈。
/// 保证无论函数怎么退出（正常返回 / 早返回 / `?` / panic）都正确弹出当前 agent，作用域不泄漏。
/// 解引用为 `Transcript`，所以作用域内可直接当 `Transcript` 用（含传给下层 `&mut Transcript`）。
pub struct AgentScope<'a> {
    tx: &'a mut Transcript,
}

impl Drop for AgentScope<'_> {
    fn drop(&mut self) {
        self.tx.agent_stack.pop();
    }
}

impl std::ops::Deref for AgentScope<'_> {
    type Target = Transcript;
    fn deref(&self) -> &Transcript {
        self.tx
    }
}

impl std::ops::DerefMut for AgentScope<'_> {
    fn deref_mut(&mut self) -> &mut Transcript {
        self.tx
    }
}

impl Transcript {
    /// 在指定路径创建日志文件（自动建父目录）
    pub fn create(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
        }
        let file = File::create(&path).map_err(TkeError::IoError)?;
        Ok(Self { path, file, events: Vec::new(), agent_stack: Vec::new() })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 进入某 agent 的执行作用域：压栈，返回守卫；守卫 Drop 时自动弹栈。
    /// 用法：`let mut tx = tx.scoped("explorer"); ...`（守卫解引用为 Transcript，原有 `tx.log(...)` 不变）。
    pub fn scoped(&mut self, agent: &str) -> AgentScope<'_> {
        self.agent_stack.push(agent.to_string());
        AgentScope { tx: self }
    }

    /// 当前正在执行的 agent（栈顶）；栈空则为编排层 "harness"
    fn current_agent(&self) -> &str {
        self.agent_stack.last().map(String::as_str).unwrap_or("harness")
    }

    /// 写入一条事件：kind 为事件类型，data 为对象（会被注入 type / agent / 时间戳）
    pub fn log(&mut self, kind: &str, mut data: serde_json::Value) {
        if !data.is_object() {
            data = serde_json::json!({ "value": data });
        }
        data["type"] = serde_json::json!(kind);
        // agent 归属：按当前作用域栈顶（权威来源，覆盖 data 里可能残留的 agent 字段）
        data["agent"] = serde_json::json!(self.current_agent());
        data["ts"] = serde_json::json!(now_rfc3339());
        if let Ok(line) = serde_json::to_string(&data) {
            let _ = writeln!(self.file, "{}", line);
            let _ = self.file.flush();
        }
        self.events.push(data);
    }

    /// 结束时导出缩进美化的对话 JSON（数组），便于人工阅读
    pub fn finalize(&self, pretty_path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.events).map_err(TkeError::JsonError)?;
        std::fs::write(pretty_path, json).map_err(TkeError::IoError)?;
        Ok(())
    }
}

/// 当前时间（RFC3339）；chrono 已是项目依赖
fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}
