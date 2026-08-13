// Librarian Agent - 从知识库中检索相关信息

use anyhow::Result;
use rig::completion::Prompt;
use rig::providers::openai;
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
    /// OpenAI 客户端
    client: openai::Client,
    /// 模型名称
    model: String,
}

impl LibrarianAgent {
    /// 创建新的 Librarian Agent
    pub fn new(knowledge_base_path: String, api_key: String, model: String) -> Result<Self> {
        let client = openai::Client::new(&api_key);
        Ok(Self {
            knowledge_base_path,
            client,
            model,
        })
    }

    /// 检索相关知识
    pub async fn retrieve_knowledge(
        &self,
        test_case_description: &str,
    ) -> Result<LibrarianKnowledge> {
        info!("Librarian 检索知识: {}", self.knowledge_base_path);

        // 扫描并读取知识库文件
        let knowledge_files = self.scan_knowledge_base().await?;

        if knowledge_files.is_empty() {
            return Ok(LibrarianKnowledge {
                relevant_items: vec![],
                summary: "知识库为空".to_string(),
            });
        }

        // 读取所有知识文件内容
        let mut all_knowledge = String::new();
        for file_path in &knowledge_files {
            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                all_knowledge.push_str(&format!("\n\n=== {} ===\n", file_path));
                all_knowledge.push_str(&content);
            }
        }

        if all_knowledge.is_empty() {
            return Ok(LibrarianKnowledge {
                relevant_items: vec![],
                summary: "知识库无内容".to_string(),
            });
        }

        // 使用 LLM 提取相关知识
        let prompt = format!(
            r#"你是一个知识库管理员。

测试用例描述:
{}

知识库内容:
{}

请从知识库中提取与测试用例相关的信息，并生成一个简洁的摘要。
摘要应该包含对测试有帮助的关键信息，比如账号信息、操作流程、预期行为等。

只返回摘要文本，不要有额外的解释。
"#,
            test_case_description, all_knowledge
        );

        let agent = self.client.agent(&self.model).build();

        let summary = agent
            .prompt(&prompt)
            .await
            .unwrap_or_else(|_| "未能提取知识".to_string());

        Ok(LibrarianKnowledge {
            relevant_items: vec![],
            summary,
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
