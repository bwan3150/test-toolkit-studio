// fetch - 采集页面信息
// 通过驱动（当前 adb）获取整张截图 + UI XML，
// 可选：OCR 提取截图文字、提取元素列表、按坐标裁剪单个元素的元素图

use crate::{Result, TkeError, Controller, Fetcher, UIElement};
use serde::Serialize;
use std::path::PathBuf;

/// fetch 选项
#[derive(Debug, Default)]
pub struct FetchOptions {
    /// 仅采集 XML（不截图）
    pub xml_only: bool,
    /// 同时提取元素列表
    pub elements: bool,
    /// 同时对截图做 OCR（语言代码，如 "eng"/"chi_sim"）
    pub ocr_lang: Option<String>,
    /// 按坐标裁剪元素图 (x1, y1, x2, y2)
    pub crop: Option<(u32, u32, u32, u32)>,
    /// 裁剪输出路径（默认 workarea/element_crop.png）
    pub crop_out: Option<PathBuf>,
}

/// fetch 结果
#[derive(Debug, Serialize)]
pub struct FetchResult {
    pub success: bool,
    /// 截图路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// UI XML 路径
    pub xml: String,
    /// 元素列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<UIElement>>,
    /// OCR 识别出的文字
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<serde_json::Value>,
    /// 裁剪出的元素图路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_image: Option<String>,
}

/// fetch 原子方法
pub struct Fetch {
    controller: Controller,
    project_path: PathBuf,
}

impl Fetch {
    pub fn new(device_id: String, project_path: PathBuf) -> Result<Self> {
        let mut controller = Controller::new(Some(device_id.clone()))?;
        controller.set_device(Some(device_id));
        Ok(Self { controller, project_path })
    }

    /// 执行采集
    pub async fn run(&self, opts: FetchOptions) -> Result<FetchResult> {
        let workarea = self.project_path.join("workarea");
        let screenshot_path = workarea.join("current_screenshot.png");
        let xml_path = workarea.join("current_ui_tree.xml");

        // 1. 采集截图 + XML（写入 workarea）
        if opts.xml_only {
            self.controller.capture_xml_only(&self.project_path).await?;
        } else {
            self.controller.capture_ui_state(&self.project_path).await?;
        }

        let mut result = FetchResult {
            success: true,
            screenshot: if opts.xml_only {
                None
            } else {
                Some(screenshot_path.to_string_lossy().to_string())
            },
            xml: xml_path.to_string_lossy().to_string(),
            elements: None,
            ocr: None,
            element_image: None,
        };

        // 2. 可选：提取元素列表
        if opts.elements {
            let fetcher = Fetcher::new();
            result.elements = Some(fetcher.fetch_elements_from_file(&xml_path)?);
        }

        // 3. 可选：OCR 提取截图文字
        if let Some(lang) = &opts.ocr_lang {
            if opts.xml_only {
                return Err(TkeError::InvalidArgument(
                    "--ocr 需要截图，不能与 --xml-only 同时使用".to_string(),
                ));
            }
            let image_data = std::fs::read(&screenshot_path).map_err(TkeError::IoError)?;
            let ocr_result = crate::ocr(&image_data, false, lang)
                .await
                .map_err(|e| TkeError::OcrError(e.to_string()))?;
            result.ocr = Some(serde_json::to_value(&ocr_result).map_err(TkeError::JsonError)?);
        }

        // 4. 可选：按坐标裁剪元素图
        if let Some((x1, y1, x2, y2)) = opts.crop {
            if opts.xml_only {
                return Err(TkeError::InvalidArgument(
                    "--crop 需要截图，不能与 --xml-only 同时使用".to_string(),
                ));
            }
            if x2 <= x1 || y2 <= y1 {
                return Err(TkeError::InvalidArgument(
                    "裁剪坐标无效: 要求 x2 > x1 且 y2 > y1".to_string(),
                ));
            }
            let out_path = opts
                .crop_out
                .unwrap_or_else(|| workarea.join("element_crop.png"));

            let img = image::open(&screenshot_path)
                .map_err(|e| TkeError::ImageError(format!("打开截图失败: {}", e)))?;
            let cropped = img.crop_imm(x1, y1, x2 - x1, y2 - y1);
            cropped
                .save(&out_path)
                .map_err(|e| TkeError::ImageError(format!("保存元素图失败: {}", e)))?;

            result.element_image = Some(out_path.to_string_lossy().to_string());
        }

        Ok(result)
    }
}
