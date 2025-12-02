// 目标解析模块 - 将参数解析为坐标点

use crate::{Result, TkeError, TksParam, Point, Controller, Recognizer, LocatorStrategy};
use std::path::PathBuf;
use tracing::{debug, info, error};

/// 目标解析器
pub struct TargetResolver<'a> {
    project_path: &'a PathBuf,
    controller: &'a mut Controller,
    recognizer: &'a Recognizer,
}

impl<'a> TargetResolver<'a> {
    pub fn new(
        project_path: &'a PathBuf,
        controller: &'a mut Controller,
        recognizer: &'a Recognizer,
    ) -> Self {
        Self {
            project_path,
            controller,
            recognizer,
        }
    }

    /// 解析目标位置
    pub async fn resolve(&mut self, param: &TksParam) -> Result<Point> {
        match param {
            TksParam::Coordinate(point) => {
                debug!("使用坐标: ({}, {})", point.x, point.y);
                Ok(*point)
            }
            TksParam::Element { name, strategy } => {
                debug!("查找元素: {}, 策略: {:?}", name, strategy);
                self.resolve_element(name, strategy).await
            }
            TksParam::Text(text) => {
                debug!("查找文本元素: {}", text);
                self.resolve_text(text).await
            }
            _ => {
                error!("无效的目标类型: {:?}", param);
                Err(TkeError::InvalidArgument("无效的目标类型".to_string()))
            }
        }
    }

    /// 解析元素定位
    async fn resolve_element(&mut self, name: &str, strategy: &LocatorStrategy) -> Result<Point> {
        // 刷新UI状态
        if let Err(e) = self.controller.capture_ui_state(self.project_path).await {
            error!("刷新UI状态失败: {}", e);
            return Err(e);
        }

        match self.recognizer.find_element(name, strategy.clone()).await {
            Ok(point) => {
                info!("找到元素 '{}' 位置: ({}, {})", name, point.x, point.y);
                Ok(point)
            }
            Err(e) => {
                error!("查找元素 '{}' 失败: {}", name, e);
                Err(e)
            }
        }
    }

    /// 解析文本查找
    async fn resolve_text(&mut self, text: &str) -> Result<Point> {
        // 刷新UI状态
        if let Err(e) = self.controller.capture_ui_state(self.project_path).await {
            error!("刷新UI状态失败: {}", e);
            return Err(e);
        }

        match self.recognizer.find_element_by_text(text) {
            Ok(point) => {
                info!("找到文本元素 '{}' 位置: ({}, {})", text, point.x, point.y);
                Ok(point)
            }
            Err(e) => {
                error!("查找文本元素 '{}' 失败: {}", text, e);
                Err(e)
            }
        }
    }
}
