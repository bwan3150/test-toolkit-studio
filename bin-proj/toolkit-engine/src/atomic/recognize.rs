// recognize - 在当前页面上定位指定元素
// 默认先 fetch 一次最新页面状态（截图+XML），再按策略定位；
// --cached 可跳过采集，直接用 workarea 中已有的页面状态

use crate::{Result, Controller, Recognizer, LocatorStrategy, Point, Bounds};
use std::path::PathBuf;

/// recognize 原子方法
pub struct Recognize {
    controller: Controller,
    recognizer: Recognizer,
    project_path: PathBuf,
}

impl Recognize {
    pub fn new(device_id: String, project_path: PathBuf) -> Result<Self> {
        let controller = Controller::new(Some(device_id))?;
        let recognizer = Recognizer::new(project_path.clone())?;
        Ok(Self { controller, recognizer, project_path })
    }

    /// 设置图像匹配置信度阈值
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.recognizer.set_confidence_threshold(threshold);
    }

    /// 按元素名定位，返回坐标
    ///
    /// - cached=false 时先采集最新页面状态再定位
    pub async fn find(
        &self,
        element_name: &str,
        strategy: LocatorStrategy,
        cached: bool,
    ) -> Result<Point> {
        if !cached {
            self.controller.capture_ui_state(&self.project_path).await?;
        }
        self.recognizer.find_element(element_name, strategy).await
    }

    /// 直接按文本在 UI 树中查找（非元素库定位）
    pub async fn find_text(&self, text: &str, cached: bool) -> Result<Point> {
        if !cached {
            self.controller.capture_ui_state(&self.project_path).await?;
        }
        self.recognizer.find_element_by_text(text)
    }

    /// 获取元素库中保存的元素框（用于结果展示/截图标注）
    pub fn locator_bounds(&self, element_name: &str) -> Option<Bounds> {
        self.recognizer
            .get_locator(element_name)
            .and_then(|l| l.bounds.clone())
    }
}
