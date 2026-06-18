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
}

impl Transcript {
    /// 在指定路径创建日志文件（自动建父目录）
    pub fn create(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
        }
        let file = File::create(&path).map_err(TkeError::IoError)?;
        Ok(Self { path, file, events: Vec::new() })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 写入一条事件：kind 为事件类型，data 为对象（会被注入 type 与时间戳）
    pub fn log(&mut self, kind: &str, mut data: serde_json::Value) {
        if !data.is_object() {
            data = serde_json::json!({ "value": data });
        }
        data["type"] = serde_json::json!(kind);
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
