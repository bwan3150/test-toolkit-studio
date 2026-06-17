// fetch - 提取当前页面的元素列表
// 默认先刷新一次页面状态（截图+XML 到设备缓存工作区），再解析输出元素；
// --cached 直接用工作区已有状态（先 tke refresh 后多次 fetch 的场景）

use crate::{Result, Controller, Fetcher, UIElement};
use crate::engines::ocr::{enrich_with_ocr, OcrSource};
use crate::utils::Workarea;

/// fetch 原子方法
pub struct Fetch {
    controller: Controller,
    workarea: Workarea,
}

impl Fetch {
    pub fn new(device_id: String) -> Result<Self> {
        let workarea = Workarea::for_device(Some(&device_id))?;
        let controller = Controller::new(Some(device_id))?;
        Ok(Self { controller, workarea })
    }

    /// 提取元素列表
    /// cached=false 时先采集最新页面状态（截图+XML）
    /// ocr 为 Some 时，用 OCR 文字增强元素表（给无标签元素补可读文字）；失败则降级用原始表
    pub async fn elements(&self, cached: bool, ocr: Option<&OcrSource>) -> Result<Vec<UIElement>> {
        if !cached {
            self.controller.capture_ui_state(&self.workarea).await?;
        }

        let fetcher = Fetcher::new();
        let mut elements = fetcher.fetch_elements_from_file(&self.workarea.ui_tree_path())?;

        if let Some(src) = ocr {
            if let Ok(bytes) = std::fs::read(self.workarea.screenshot_path()) {
                // OCR 失败不致命：保留原始元素表
                let _ = enrich_with_ocr(&mut elements, &bytes, src).await;
            }
        }

        Ok(elements)
    }

    /// 列出标签页（仅 web；其它平台为空）
    pub fn list_tabs(&self) -> Vec<crate::TabInfo> {
        self.controller.list_tabs()
    }
}
