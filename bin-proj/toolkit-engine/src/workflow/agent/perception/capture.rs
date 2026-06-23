// 感知：采集当前页面（完整截图 + UI XML）并解析元素列表
// 截图/页面的持久化产物由 RunArtifacts 按 step 统一保存（见 runner），此处只采集到工作区。

use std::path::PathBuf;

use crate::engines::ocr::{enrich_with_ocr, OcrSource};
use crate::{Control, Fetcher, Platform, Refresh, RefreshOptions, Result, TabInfo, UIElement, Workarea};

/// 一次采集的结果
pub struct Perceived {
    /// 解析出的元素列表
    pub elements: Vec<UIElement>,
    /// 工作区当前截图路径（要图时直接喂给 AI）
    pub shot_path: PathBuf,
    /// 工作区当前 UI XML 路径
    pub xml_path: PathBuf,
    /// OCR 给无标签元素回填文字的个数
    pub ocr_filled: usize,
    /// OCR 新增伪元素的个数（XML 漏掉的文字）
    pub ocr_added: usize,
    /// OCR 识别到的文字总数（如实展示用：>0 即说明 OCR 在工作，哪怕 filled/added 都为 0）
    pub ocr_recognized: usize,
    /// OCR 是否报错（接口失败/识别崩溃等）——Some=出错了，别让"0"掩盖问题
    pub ocr_error: Option<String>,
    /// 浏览器标签页（仅 web；其它平台为空）
    pub tabs: Vec<TabInfo>,
}

/// 采集一轮：刷新（截图 + XML）→ 解析元素 →（可选）OCR 增强
/// ocr 为 Some 时，用 OCR 文字给无标签元素补可读文字；失败降级用原始表。
pub async fn capture(
    device: &str,
    workarea: &Workarea,
    fetcher: &Fetcher,
    ocr: Option<&OcrSource>,
) -> Result<Perceived> {
    let refresh = Refresh::new(device.to_string())?;
    refresh.run(RefreshOptions::default()).await?;

    let xml_path = workarea.ui_tree_path();
    let shot_path = workarea.screenshot_path();
    let mut elements = fetcher.fetch_elements_from_file(&xml_path)?;

    // OCR：如实记录结果——别把"接口报错"和"识别到 0 个"混成一个静默的 0。
    let (mut ocr_filled, mut ocr_added, mut ocr_recognized) = (0, 0, 0);
    let mut ocr_error: Option<String> = None;
    if let Some(src) = ocr {
        match std::fs::read(&shot_path) {
            Ok(bytes) => match enrich_with_ocr(&mut elements, &bytes, src).await {
                Ok((f, a, n)) => {
                    ocr_filled = f;
                    ocr_added = a;
                    ocr_recognized = n;
                }
                Err(e) => ocr_error = Some(e.to_string()),
            },
            Err(e) => ocr_error = Some(format!("读截图失败：{}", e)),
        }
    }

    // 标签页（仅 web；list_tabs 会逐个切换读标题再切回，故只在 web 调用）
    let tabs = if Platform::from_device(Some(device)) == Platform::Web {
        Control::new(device.to_string()).map(|c| c.list_tabs()).unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(Perceived { elements, shot_path, xml_path, ocr_filled, ocr_added, ocr_recognized, ocr_error, tabs })
}
