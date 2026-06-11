// refresh - 刷新页面状态到工作区
// 通过驱动（当前 adb）采集整张截图 + UI XML 到设备缓存工作区，
// 可选：OCR 提取截图文字、按坐标剪裁单个元素的元素图

use crate::{Result, TkeError, Controller};
use crate::utils::Workarea;
use serde::Serialize;
use std::path::PathBuf;

/// refresh 选项
#[derive(Debug, Default)]
pub struct RefreshOptions {
    /// 仅采集 XML（不截图）
    pub xml_only: bool,
    /// 同时对截图做 OCR（语言代码，如 "eng"/"chi_sim"）
    pub ocr_lang: Option<String>,
    /// 按坐标剪裁元素图 (x1, y1, x2, y2)
    pub crop: Option<(u32, u32, u32, u32)>,
    /// 剪裁输出路径（默认工作区 element_crop.png）
    pub crop_out: Option<PathBuf>,
}

/// refresh 结果
#[derive(Debug, Serialize)]
pub struct RefreshResult {
    pub success: bool,
    /// 截图路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// UI XML 路径
    pub xml: String,
    /// OCR 识别出的文字
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<serde_json::Value>,
    /// 剪裁出的元素图路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_image: Option<String>,
}

/// refresh 原子方法
pub struct Refresh {
    controller: Controller,
    workarea: Workarea,
}

impl Refresh {
    pub fn new(device_id: String) -> Result<Self> {
        let workarea = Workarea::for_device(Some(&device_id))?;
        let controller = Controller::new(Some(device_id))?;
        Ok(Self { controller, workarea })
    }

    /// 执行采集
    pub async fn run(&self, opts: RefreshOptions) -> Result<RefreshResult> {
        let screenshot_path = self.workarea.screenshot_path();
        let xml_path = self.workarea.ui_tree_path();

        // 1. 采集截图 + XML（写入设备缓存工作区）
        if opts.xml_only {
            self.controller.capture_xml_only(&self.workarea).await?;
        } else {
            self.controller.capture_ui_state(&self.workarea).await?;
        }

        let mut result = RefreshResult {
            success: true,
            screenshot: if opts.xml_only {
                None
            } else {
                Some(screenshot_path.to_string_lossy().to_string())
            },
            xml: xml_path.to_string_lossy().to_string(),
            ocr: None,
            element_image: None,
        };

        // 2. 可选：OCR 提取截图文字
        if let Some(lang) = &opts.ocr_lang {
            if opts.xml_only {
                return Err(TkeError::InvalidArgument(
                    "--ocr 需要截图，不能与 --xml-only 同时使用".to_string(),
                ));
            }
            let image_data = std::fs::read(&screenshot_path).map_err(TkeError::IoError)?;
            let ocr_result = crate::run_ocr(&image_data, false, lang)
                .await
                .map_err(|e| TkeError::OcrError(e.to_string()))?;
            result.ocr = Some(serde_json::to_value(&ocr_result).map_err(TkeError::JsonError)?);
        }

        // 3. 可选：按坐标剪裁元素图
        if let Some((x1, y1, x2, y2)) = opts.crop {
            if opts.xml_only {
                return Err(TkeError::InvalidArgument(
                    "--crop 需要截图，不能与 --xml-only 同时使用".to_string(),
                ));
            }
            if x2 <= x1 || y2 <= y1 {
                return Err(TkeError::InvalidArgument(
                    "剪裁坐标无效: 要求 x2 > x1 且 y2 > y1".to_string(),
                ));
            }
            let out_path = opts
                .crop_out
                .unwrap_or_else(|| self.workarea.dir().join("element_crop.png"));

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
