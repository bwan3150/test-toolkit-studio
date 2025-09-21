// Librarian Agent - 负责查找和管理知识库

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;
use tracing::{info, debug, warn};

use crate::core::error::{Result, AiTesterError};
use crate::models::*;
use super::Agent;

pub struct Librarian {
    knowledge_base_path: PathBuf,
}

impl Librarian {
    pub fn new(project_path: &Path) -> Result<Self> {
        let knowledge_base_path = project_path.join("knowledge_base");

        // 如果知识库目录不存在，创建它
        if !knowledge_base_path.exists() {
            std::fs::create_dir_all(&knowledge_base_path)
                .map_err(|e| AiTesterError::FileSystemError(
                    format!("创建知识库目录失败: {}", e)
                ))?;
            info!("创建知识库目录: {:?}", knowledge_base_path);
        }

        Ok(Self {
            knowledge_base_path,
        })
    }

    /// 搜索相关知识
    pub async fn search_knowledge(&self, test_case: &TestCase) -> Result<Vec<String>> {
        info!("Librarian 开始搜索相关知识: {}", test_case.name);

        let mut knowledge = Vec::new();

        // 搜索关键词
        let keywords = Self::extract_keywords(&test_case.name, &test_case.description);

        // 遍历知识库文件
        for entry in WalkDir::new(&self.knowledge_base_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // 只处理文本文件
            if let Some(ext) = path.extension() {
                if ext != "txt" && ext != "md" && ext != "json" {
                    continue;
                }
            }

            // 读取文件内容
            if let Ok(content) = std::fs::read_to_string(path) {
                // 检查是否包含关键词
                if Self::content_matches(&content, &keywords) {
                    debug!("找到相关知识文件: {:?}", path);
                    knowledge.push(content);

                    // 限制知识条目数量
                    if knowledge.len() >= 5 {
                        break;
                    }
                }
            }
        }

        if knowledge.is_empty() {
            warn!("未找到相关知识");
        } else {
            info!("找到 {} 条相关知识", knowledge.len());
        }

        Ok(knowledge)
    }

    /// 保存新知识
    pub async fn save_knowledge(&self, test_case_id: &str, content: &str) -> Result<()> {
        let file_name = format!("{}.txt", test_case_id);
        let file_path = self.knowledge_base_path.join(file_name);

        std::fs::write(&file_path, content)
            .map_err(|e| AiTesterError::FileSystemError(
                format!("保存知识失败: {}", e)
            ))?;

        info!("知识已保存: {:?}", file_path);
        Ok(())
    }

    /// 提取关键词
    fn extract_keywords(name: &str, description: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // 简单的关键词提取（可以改进为更智能的方式）
        let text = format!("{} {}", name, description).to_lowercase();

        // 分词（简单处理，可以使用更好的分词库）
        for word in text.split_whitespace() {
            // 过滤太短的词
            if word.len() > 2 {
                // 去除标点符号
                let cleaned = word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();

                if !cleaned.is_empty() && !keywords.contains(&cleaned) {
                    keywords.push(cleaned);
                }
            }
        }

        keywords
    }

    /// 检查内容是否匹配关键词
    fn content_matches(content: &str, keywords: &[String]) -> bool {
        let content_lower = content.to_lowercase();

        // 至少匹配一半的关键词
        let required_matches = (keywords.len() / 2).max(1);
        let mut matches = 0;

        for keyword in keywords {
            if content_lower.contains(keyword) {
                matches += 1;
                if matches >= required_matches {
                    return true;
                }
            }
        }

        false
    }
}

#[async_trait]
impl Agent for Librarian {
    fn name(&self) -> &str {
        "Librarian"
    }

    fn description(&self) -> &str {
        "负责查找和管理测试知识库"
    }

    async fn process(&self, message: AgentMessage) -> Result<AgentMessage> {
        match message {
            AgentMessage::TestCaseAnalysis { test_case, analysis } => {
                let knowledge = self.search_knowledge(&test_case).await?;
                Ok(AgentMessage::KnowledgeFound {
                    test_case_id: test_case.id,
                    knowledge,
                })
            }
            _ => Err(AiTesterError::AgentError(
                format!("{} 不处理此类消息", self.name())
            )),
        }
    }
}