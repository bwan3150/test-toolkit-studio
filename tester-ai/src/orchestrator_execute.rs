// 操作执行模块 - 将 AI 决策转换为实际的 TKE 命令执行

use crate::models::{ActionDecision, ActionType};
use crate::orchestrator::TesterOrchestrator;
use anyhow::{Context, Result};
use tracing::info;

impl TesterOrchestrator {
    /// 执行 AI 决策的操作
    pub async fn execute_action(&mut self, decision: &ActionDecision) -> Result<()> {
        match decision.action_type {
            ActionType::Click => {
                let element_id = decision
                    .target_element_id
                    .context("Click 操作需要目标元素 ID")?;

                let pos = self
                    .parser
                    .get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                info!("执行点击: ({}, {})", pos.0, pos.1);
                self.tke.tap(pos.0, pos.1).await?;
            }

            ActionType::Swipe => {
                let element_id = decision
                    .target_element_id
                    .context("Swipe 操作需要起点元素 ID")?;

                let from = self
                    .parser
                    .get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                let to = decision
                    .params
                    .to
                    .context("Swipe 操作需要终点坐标")?;

                let duration = decision.params.duration;

                info!("执行滑动: ({}, {}) -> ({}, {})", from.0, from.1, to.0, to.1);
                self.tke.swipe(from.0, from.1, to.0, to.1, duration).await?;
            }

            ActionType::Input => {
                let element_id = decision
                    .target_element_id
                    .context("Input 操作需要目标元素 ID")?;

                let pos = self
                    .parser
                    .get_element_position(element_id)
                    .context(format!("找不到元素 ID {}", element_id))?;

                let text = decision
                    .params
                    .text
                    .as_ref()
                    .context("Input 操作需要文本")?;

                info!("执行点击输入框: ({}, {})", pos.0, pos.1);
                self.tke.tap(pos.0, pos.1).await?;

                // 等待输入框获得焦点
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // 清空输入框
                info!("清空输入框内容");
                self.tke.clear_input().await?;

                // 短暂等待清空完成
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                info!("执行输入: {}", text);
                self.tke.input(text).await?;
            }

            ActionType::Back => {
                info!("执行返回");
                self.tke.back().await?;
            }

            ActionType::Wait => {
                let duration = decision.params.duration.unwrap_or(1000);
                info!("执行等待: {} ms", duration);
                tokio::time::sleep(tokio::time::Duration::from_millis(duration as u64)).await;
            }

            ActionType::Launch => {
                let package = decision
                    .params
                    .package
                    .as_ref()
                    .context("Launch 操作需要包名")?;

                let activity = decision
                    .params
                    .activity
                    .as_ref()
                    .context("Launch 操作需要 Activity")?;

                info!("执行启动应用: {} / {}", package, activity);
                self.tke.launch(package, activity).await?;
            }

            ActionType::Stop => {
                let package = decision
                    .params
                    .package
                    .as_ref()
                    .context("Stop 操作需要包名")?;

                info!("执行停止应用: {}", package);
                self.tke.stop(package).await?;
            }

            // 其他操作类型暂不实际执行
            _ => {
                info!("操作类型 {:?} 暂不支持实际执行", decision.action_type);
            }
        }

        Ok(())
    }
}
