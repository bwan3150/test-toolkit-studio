// 【记忆/知识库】mem0 记忆 + RAG 知识库远端检索 —— 本期留口子
//
// 未配置 endpoint（[knowledge] 段为空）则跳过真实调用，返回 Skipped(原因)，
// 由上层记进 conversation 原始日志。远端就绪后在 query_* 内补 HTTP 调用即可，接口不变。

use crate::utils::KnowledgeConfig;

/// 一次检索的结果
pub enum KnowledgeOutcome {
    /// 命中：检索到的上下文文本
    Hit(String),
    /// 跳过：原因（未配置 / 暂未实现 / 调用失败）
    Skipped(String),
}

/// 记忆 + 知识库客户端
pub struct Knowledge {
    mem0_endpoint: Option<String>,
    rag_endpoint: Option<String>,
}

impl Knowledge {
    pub fn new(cfg: &KnowledgeConfig) -> Self {
        let norm = |s: &Option<String>| {
            s.as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Self {
            mem0_endpoint: norm(&cfg.mem0_endpoint),
            rag_endpoint: norm(&cfg.rag_endpoint),
        }
    }

    /// 查询 mem0 记忆
    pub fn query_memory(&self, _query: &str) -> KnowledgeOutcome {
        match &self.mem0_endpoint {
            None => KnowledgeOutcome::Skipped("mem0 未配置（[knowledge].mem0_endpoint 为空）".into()),
            // TODO: mem0 远端就绪后在此发起 HTTP 检索
            Some(ep) => KnowledgeOutcome::Skipped(format!(
                "mem0 真实调用尚未实现（本期留口子），endpoint={}",
                ep
            )),
        }
    }

    /// 查询 RAG 知识库
    pub fn query_rag(&self, _query: &str) -> KnowledgeOutcome {
        match &self.rag_endpoint {
            None => KnowledgeOutcome::Skipped("RAG 未配置（[knowledge].rag_endpoint 为空）".into()),
            // TODO: RAG 远端就绪后在此发起 HTTP 检索
            Some(ep) => KnowledgeOutcome::Skipped(format!(
                "RAG 真实调用尚未实现（本期留口子），endpoint={}",
                ep
            )),
        }
    }
}
