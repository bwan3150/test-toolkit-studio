// Workarea - 页面采集工作区（系统缓存目录，与项目目录解耦）
//
// 两种模式：
//   1. 设备缓存区: <cache 根>/workarea/<device_id>/
//      原子命令 fetch/recognize 使用，跨进程共享（fetch 后 recognize --cached 才能工作），不删除
//   2. 运行临时区: <cache 根>/run-<时间戳>-<pid>/
//      run 工作流使用，运行结束后整目录删除
//
// **cache 根跟着 `--cache` 走**。此前这里写死 `$TMPDIR/tke`，
// 于是 `--cache` 的文档("截图/页面结构…的落点")对 refresh/fetch 是不作数的。
// 两个后果：
//   1. `tke serve` 给每个会话分了独立 cache 目录，采集产物却全落在设备级共享目录里 ——
//      租约结束也不清，**下一个租户能读到上一个租户的屏幕**（INV-17 说的"设备脏状态"）
//   2. 平台按会话取产物时什么也取不到（云设备的"点一下截一张"就是卡在这里）
//
// 做成进程级而不是逐层透传：一次进程就一个 `--cache`，这是事实本身的形状；
// 而透传要动十几处签名（含 agent 执行引擎深处）。与 set_ocr_url / set_web_headless 同一套路。

use crate::{Result, TkeError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CACHE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 设定本进程的 cache 根（main 里解析完 params 就调一次）。
/// 只认第一次 —— 中途换根会让"先 fetch 后 recognize --cached"这类跨命令约定失效。
pub fn set_cache_root(root: PathBuf) {
    let _ = CACHE_ROOT.set(root);
}

/// 本进程的 cache 根：显式设过就用它，否则退回 `$TMPDIR/tke`（保持老行为）
fn cache_root() -> PathBuf {
    CACHE_ROOT
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("tke"))
}

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
        let dir = cache_root()
            .join("workarea")
            .join(sanitize(device_id.unwrap_or("default")));
        std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;
        Ok(Self { dir, is_temp: false })
    }

    /// 运行临时区（run 工作流专用，结束后调用 cleanup 删除）。
    /// 目录名带**进程内自增序号**：此前只有 秒级时间戳+pid，同一进程同一秒启动的多个回放
    /// （并行测试/并行回放）会共享同一目录，截图/XML 互相覆盖——flaky 的经典来源。
    pub fn temp_for_run() -> Result<Self> {
        static RUN_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = RUN_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = cache_root().join(format!(
            "run-{}-{}-{}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            std::process::id(),
            seq
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

    /// **驱动直接给的原始页面**（web=DOM 原文 .html，安卓/iOS=驱动原生 XML）。
    ///
    /// 与 `ui_tree_path()` 的区别是这份**没被 tke 动过**：
    ///   - 给 AI 一个"页面本来长什么样"的参照，判断某个元素是不是被我们漏采了
    ///   - 也是脚本持久化的底料——将来页面改版，对着两份原文才看得出改了什么
    pub fn raw_page_path(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("current_raw_page.{}", ext))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 设备缓存区必须落在 `--cache` 根下。
    ///
    /// 回归的是这个真问题：这里原本写死 `$TMPDIR/tke`，于是 `tke serve` 明明给每个会话
    /// 分了独立 cache 目录，refresh/fetch 的截图和页面结构却全落在设备级共享目录里 ——
    /// 租约结束也不清，下一个租户读得到上一个租户的屏幕。
    ///
    /// **只断言"在根之下"，不断言完整路径**：目录分层将来可能调整，
    /// 而"不许跑到根外面去"才是这条不变量本身。
    #[test]
    fn 设备缓存区落在cache根之下() {
        let root = std::env::temp_dir().join("tke-test-cache-root");
        set_cache_root(root.clone());
        // OnceLock 只认第一次：同进程内别的测试可能已经设过，那就以实际生效的为准
        let effective = cache_root();

        let wa = Workarea::for_device(Some("emulator-5554")).unwrap();
        assert!(
            wa.dir().starts_with(&effective),
            "设备缓存区 {:?} 跑到了 cache 根 {:?} 外面",
            wa.dir(),
            effective
        );

        let run = Workarea::temp_for_run().unwrap();
        assert!(
            run.dir().starts_with(&effective),
            "运行临时区 {:?} 跑到了 cache 根 {:?} 外面",
            run.dir(),
            effective
        );
        run.cleanup();
    }

    #[test]
    fn 设备id里的特殊字符不会变成目录分隔() {
        assert_eq!(sanitize("192.168.1.5:5555"), "192_168_1_5_5555");
        assert!(!sanitize("../../etc").contains('/'));
    }
}
