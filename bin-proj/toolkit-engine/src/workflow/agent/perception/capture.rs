// 感知：采集当前页面（完整截图 + UI XML）并解析元素列表，保存本轮截图快照

use std::path::{Path, PathBuf};

use crate::{Fetcher, Refresh, RefreshOptions, Result, UIElement, Workarea};

/// 一次采集的结果
pub struct Perceived {
    /// 解析出的元素列表
    pub elements: Vec<UIElement>,
    /// 工作区当前截图路径（要图时直接喂给 AI）
    pub shot_path: PathBuf,
    /// 工作区当前 UI XML 路径
    pub xml_path: PathBuf,
    /// 本轮截图快照保存到 screens 目录的路径（供日志/复盘）
    pub saved_shot: Option<PathBuf>,
}

/// 采集一轮：刷新（截图 + XML）→ 解析元素 → 存本轮截图快照
pub async fn capture(
    device: &str,
    workarea: &Workarea,
    fetcher: &Fetcher,
    screens_dir: &Path,
    round: usize,
) -> Result<Perceived> {
    let refresh = Refresh::new(device.to_string())?;
    refresh.run(RefreshOptions::default()).await?;

    let xml_path = workarea.ui_tree_path();
    let shot_path = workarea.screenshot_path();
    let elements = fetcher.fetch_elements_from_file(&xml_path)?;
    let saved_shot = save_round_screenshot(screens_dir, &shot_path, round);

    Ok(Perceived {
        elements,
        shot_path,
        xml_path,
        saved_shot,
    })
}

/// 保存本轮截图到 screens 目录，返回保存路径
fn save_round_screenshot(screens_dir: &Path, shot_path: &Path, round: usize) -> Option<PathBuf> {
    if !shot_path.exists() {
        return None;
    }
    if std::fs::create_dir_all(screens_dir).is_err() {
        return None;
    }
    let dest = screens_dir.join(format!("round_{:03}.png", round));
    std::fs::copy(shot_path, &dest).ok().map(|_| dest)
}
