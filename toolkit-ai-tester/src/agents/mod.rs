// AI Agent 模块

pub mod receptionist;
pub mod librarian;
pub mod worker;
pub mod supervisor;

use async_trait::async_trait;
use crate::core::error::Result;
use crate::models::*;

/// Agent 基础 trait
#[async_trait]
pub trait Agent {
    /// Agent 名称
    fn name(&self) -> &str;

    /// Agent 描述
    fn description(&self) -> &str;

    /// 处理消息
    async fn process(&self, message: AgentMessage) -> Result<AgentMessage>;
}