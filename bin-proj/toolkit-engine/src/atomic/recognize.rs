// recognize - 在当前页面上定位指定元素
// 默认先 fetch 一次最新页面状态（截图+XML），再按策略定位；
// --cached 可跳过采集，直接用设备缓存工作区中已有的页面状态

use crate::{Result, Controller, Recognizer, LocatorStrategy, Platform, Point, Bounds};
use crate::utils::Workarea;
use std::path::Path;

/// recognize 原子方法
pub struct Recognize {
    controller: Controller,
    recognizer: Recognizer,
    workarea: Workarea,
}

impl Recognize {
    /// element_path: 元素库路径（None 按默认路径查找）
    pub fn new(device_id: String, element_path: Option<&Path>) -> Result<Self> {
        let workarea = Workarea::for_device(Some(&device_id))?;
        let platform = Platform::from_device(Some(&device_id));
        let controller = Controller::new(Some(device_id))?;
        let recognizer = Recognizer::new(element_path, workarea.clone(), platform)?;
        Ok(Self { controller, recognizer, workarea })
    }

    /// 设置图像匹配置信度阈值
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.recognizer.set_confidence_threshold(threshold);
    }

    /// 按元素名定位，返回 (中心点, 实时边界框)
    ///
    /// - cached=false 时先采集最新页面状态再定位
    pub async fn find(
        &self,
        element_name: &str,
        strategy: LocatorStrategy,
        cached: bool,
    ) -> Result<(Point, Bounds)> {
        if !cached {
            self.controller.capture_ui_state(&self.workarea).await?;
        }
        self.recognizer.find_element_detailed(element_name, strategy).await
    }

    /// 直接按文本在 UI 树中查找（非元素库定位）
    pub async fn find_text(&self, text: &str, cached: bool) -> Result<(Point, Bounds)> {
        if !cached {
            self.controller.capture_ui_state(&self.workarea).await?;
        }
        self.recognizer.find_element_by_text(text)
    }
}
