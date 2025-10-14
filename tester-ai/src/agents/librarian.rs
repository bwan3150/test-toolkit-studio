// Librarian Agent - 从知识库中检索相关信息

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

/// Librarian 检索的知识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarianKnowledge {
    /// 相关知识条目
    pub relevant_items: Vec<KnowledgeItem>,

    /// 知识摘要
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    /// 知识标题
    pub title: String,

    /// 知识内容
    pub content: String,

    /// 相关度评分 (0-1)
    pub relevance_score: f32,
}

/// Librarian Agent
pub struct LibrarianAgent {
    /// 知识库路径
    knowledge_base_path: String,
}

impl LibrarianAgent {
    /// 创建新的 Librarian Agent
    pub fn new(knowledge_base_path: String) -> Self {
        Self {
            knowledge_base_path,
        }
    }

    /// 检索相关知识
    pub async fn retrieve_knowledge(
        &self,
        test_case_description: &str,
    ) -> Result<LibrarianKnowledge> {
        info!("Librarian 检索知识: {}", self.knowledge_base_path);

        // TODO: 实现实际的知识库检索
        // 1. 扫描知识库文件夹
        // 2. 读取 markdown/txt 文件
        // 3. 使用向量搜索或关键词匹配找到相关内容
        // 4. 使用 LLM 总结相关知识

        // 暂时返回空知识
        Ok(LibrarianKnowledge {
            relevant_items: vec![],
            summary: "暂无相关知识".to_string(),
        })
    }

    /// 扫描知识库文件
    async fn scan_knowledge_base(&self) -> Result<Vec<String>> {
        let path = Path::new(&self.knowledge_base_path);

        if !path.exists() {
            return Ok(vec![]);
        }

        let mut files = Vec::new();

        if path.is_dir() {
            let entries = std::fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                let file_path = entry.path();
                if file_path.is_file() {
                    if let Some(ext) = file_path.extension() {
                        if ext == "md" || ext == "txt" {
                            if let Some(path_str) = file_path.to_str() {
                                files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(files)
    }
}
