// 感知：采集当前页面（完整截图 + UI XML）并解析元素列表
// 截图/页面的持久化产物由 RunArtifacts 按 step 统一保存（见 runner），此处只采集到工作区。

use std::path::PathBuf;

use crate::{Fetcher, Refresh, RefreshOptions, Result, UIElement, Workarea};

/// 一次采集的结果
pub struct Perceived {
    /// 解析出的元素列表
    pub elements: Vec<UIElement>,
    /// 工作区当前截图路径（要图时直接喂给 AI）
    pub shot_path: PathBuf,
    /// 工作区当前 UI XML 路径
    pub xml_path: PathBuf,
}

/// 采集一轮：刷新（截图 + XML）→ 解析元素
pub async fn capture(device: &str, workarea: &Workarea, fetcher: &Fetcher) -> Result<Perceived> {
    let refresh = Refresh::new(device.to_string())?;
    refresh.run(RefreshOptions::default()).await?;

    let xml_path = workarea.ui_tree_path();
    let shot_path = workarea.screenshot_path();
    let elements = fetcher.fetch_elements_from_file(&xml_path)?;

    Ok(Perceived { elements, shot_path, xml_path })
}
