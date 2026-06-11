// Workarea - 页面采集工作区（系统缓存目录，与项目目录解耦）
//
// 两种模式：
//   1. 设备缓存区: $TMPDIR/tke/workarea/<device_id>/
//      原子命令 fetch/recognize 使用，跨进程共享（fetch 后 recognize --cached 才能工作），不删除
//   2. 运行临时区: $TMPDIR/tke/run-<时间戳>-<pid>/
//      run 工作流使用，运行结束后整目录删除

use crate::{Result, TkeError};
use std::path::{Path, PathBuf};

/// 页面采集工作区
#[derive(Debug, Clone)]
pub struct Workarea {
    dir: PathBuf,
    /// 临时区标记：true 时 cleanup() 删除整个目录
    is_temp: bool,
}

impl Workarea {
    /// 设备缓存区（跨进程共享，不删除）
    pub fn for_device(device_id: Option<&str>) -> Result<Self> {
        let dir = std::env::temp_dir()
            .join("tke")
            .join("workarea")
            .join(sanitize(device_id.unwrap_or("default")));
        std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;
        Ok(Self { dir, is_temp: false })
    }

    /// 运行临时区（run 工作流专用，结束后调用 cleanup 删除）
    pub fn temp_for_run() -> Result<Self> {
        let dir = std::env::temp_dir().join("tke").join(format!(
            "run-{}-{}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;
        Ok(Self { dir, is_temp: true })
    }

    /// 工作区目录
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 当前截图路径
    pub fn screenshot_path(&self) -> PathBuf {
        self.dir.join("current_screenshot.png")
    }

    /// 当前 UI 结构文件路径（xml/wda/dom）
    pub fn ui_tree_path(&self) -> PathBuf {
        self.dir.join("current_ui_tree.xml")
    }

    /// 清理：临时区删除整个目录，设备缓存区不动
    pub fn cleanup(&self) {
        if self.is_temp {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// 清理设备 ID 中不适合做目录名的字符（如 192.168.1.5:5555）
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
