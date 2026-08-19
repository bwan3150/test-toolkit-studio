// 运行产物管理 - 每次运行留下完整记录用于定位问题
// 仅在指定 --log <dir> 时启用。两种布局，按命令语义选（见 Layout）：
//
// ① Layout::Task（`tke steps`）——**一个任务一份证据**，反复调用续写同一目录：
// <log>/
//   ├── report.html           一份全程报告（步骤连续编号，跨设备也在同一条时间线上）
//   ├── log.json              { batches: [每批一个 ExecutionResult] }
//   ├── screenshots/
//   │   └── step_001.png      跨批次**连续编号**
//   └── pages/
//       └── step_001.xml
//
// ② Layout::Timestamped（`tke run` / flow / harness）——每次运行独立一份：
// <log>/<脚本名>_<时间戳>/
//   ├── log.json / report.html / screenshots/ / pages/
//   （第二遍回放另起一个目录，方便和上一遍对比——这正是回放场景要的）

use crate::{Result, TkeError, ActionTrace};
use crate::utils::Workarea;
use ab_glyph::{FontVec, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_hollow_circle_mut, draw_hollow_rect_mut,
    draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use std::path::{Path, PathBuf};

/// 产物布局：一次性检查(steps)与可回放脚本运行(run/flow/harness)的语义不同，落点也不同
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    /// 每次运行独立一份 `<log>/<名>_<时间戳>/`（run / flow / harness）
    Timestamped,
    /// 一个任务一份，反复调用**续写同一目录**、步骤连续编号（steps）
    Task,
}

/// 单次运行的产物目录
pub struct RunArtifacts {
    /// 运行根目录（Timestamped 为 `<log>/<名>_<时间戳>/`；Task 为 `<log>/` 本身）
    pub run_dir: PathBuf,
    /// 截图序列目录
    screenshots_dir: PathBuf,
    /// 页面结构序列目录（tke 归一化后的元素表）
    page_dir: PathBuf,
    /// 原始页面序列目录（驱动直给，没被 tke 动过）
    raw_page_dir: PathBuf,
    /// 本批步骤编号的起点：Task 布局下续写已有序列，从已存在的最大编号往后接
    step_offset: usize,
    /// 文字标注字体（系统字体，加载失败则不画文字）
    font: Option<FontVec>,
}

impl RunArtifacts {
    /// 在 log 根目录下创建本次运行的产物目录
    /// 命名统一为 `<名>_<时间戳>`（无名则纯 `<时间戳>`），run/flow/harness 一致
    pub fn create(log_root: &Path, name: &str) -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let dir_name = if name.trim().is_empty() {
            timestamp
        } else {
            format!("{}_{}", sanitize(name), timestamp)
        };
        let run_dir = log_root.join(dir_name);
        Self::create_at(run_dir)
    }

    /// 按布局创建（steps 走 Task：`--log` 指的就是任务目录本身，反复调用续写）
    pub fn create_with(log_root: &Path, name: &str, layout: Layout) -> Result<Self> {
        match layout {
            Layout::Timestamped => Self::create(log_root, name),
            Layout::Task => {
                let mut a = Self::create_at(log_root.to_path_buf())?;
                a.step_offset = a.next_step_index();
                Ok(a)
            }
        }
    }

    /// 在已有目录下创建（flow 中每个脚本的子目录）
    pub fn create_in(parent: &Path, name: &str) -> Result<Self> {
        Self::create_at(parent.join(sanitize(name)))
    }

    fn create_at(run_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&run_dir).map_err(TkeError::IoError)?;
        let screenshots_dir = run_dir.join("screenshots");
        let page_dir = run_dir.join("pages");
        let raw_page_dir = run_dir.join("raw_pages");
        Ok(Self {
            run_dir,
            screenshots_dir,
            page_dir,
            raw_page_dir,
            step_offset: 0,
            font: load_system_font(),
        })
    }

    /// 已有序列的下一个编号：扫 `screenshots/step_NNN.*` 与 `pages/step_NNN.*` 取最大值。
    /// 按**文件**而不是按 log.json 数步数——中途 Ctrl+C 的批次可能没写完 log，
    /// 但截图已经落了；漏算就会覆盖掉上一批的证据。
    fn next_step_index(&self) -> usize {
        let mut max = 0usize;
        for dir in [&self.screenshots_dir, &self.page_dir] {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let n = name
                    .strip_prefix("step_")
                    .and_then(|s| s.split('.').next())
                    .and_then(|s| s.parse::<usize>().ok());
                if let Some(n) = n {
                    max = max.max(n);
                }
            }
        }
        max
    }

    /// 保存单步产物：从工作区复制截图（标注后）和页面结构文件
    /// label 为本步操作描述（如 "返回" / "点击 [{Devices入口}]"），画在截图顶部横幅
    /// 返回 (截图相对路径, 结构文件相对路径)
    /// 工作区里那份「驱动直给的原文」——按 `current_raw_page.*` 前缀找，
    /// **不认扩展名**：驱动想用什么后缀是它的事，收集方不该有一份需要维护的清单
    fn find_raw_page_impl(dir: &std::path::Path) -> Option<PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("current_raw_page."))
            })
    }

    pub fn save_step(
        &self,
        workarea: &Workarea,
        step_index: usize,
        trace: &ActionTrace,
        label: &str,
        success: bool,
    ) -> (Option<String>, Option<String>) {
        let src_png = workarea.screenshot_path();
        let src_xml = workarea.ui_tree_path();

        // Task 布局下接着上一批的编号往下排（step_offset），全程一条连续序列
        let seq = self.step_offset + step_index + 1;
        let png_name = format!("step_{:03}.png", seq);
        let xml_name = format!("step_{:03}.xml", seq);

        // 截图：标注横幅 + 元素框 + 点击点后保存（标注失败则原样复制）
        let screenshot = if src_png.exists() && std::fs::create_dir_all(&self.screenshots_dir).is_ok() {
            let dst = self.screenshots_dir.join(&png_name);
            let banner = format!(
                "Step {} {} | {}",
                seq,
                if success { "OK" } else { "FAIL" },
                label
            );
            let ok = annotate_screenshot(&src_png, &dst, trace, &banner, success, self.font.as_ref())
                .or_else(|_| std::fs::copy(&src_png, &dst).map(|_| ()))
                .is_ok();
            ok.then(|| format!("screenshots/{}", png_name))
        } else {
            None
        };

        // 页面结构：存**解析后的元素表 JSON**，而不是内部那份 XML。
        // 这份等于"当前页面的元素库"——AI 直接读就知道页面上有什么、能点什么、
        // 各自在哪（也是将来把一次性检查固化成脚本的底料）。
        // 落盘用紧凑 JSON：一个元素一行，几十个元素也就几 KB。
        let json_name = format!("step_{:03}.json", seq);
        let xml = if src_xml.exists() && std::fs::create_dir_all(&self.page_dir).is_ok() {
            match crate::Fetcher::new().fetch_elements_from_file(&src_xml) {
                Ok(els) => serde_json::to_string_pretty(&els)
                    .ok()
                    .and_then(|s| std::fs::write(self.page_dir.join(&json_name), s).ok())
                    .map(|_| format!("pages/{}", json_name)),
                // 解析不了就退回原样复制，总比什么都不留强
                Err(_) => {
                    let dst = self.page_dir.join(&xml_name);
                    std::fs::copy(&src_xml, &dst).ok().map(|_| format!("pages/{}", xml_name))
                }
            }
        } else {
            None
        };

        // **驱动直给的原文**另存一份（web=.html / 安卓与 iOS 真机=.xml / iOS 模拟器=.json）。
        // `pages/` 里是 tke 筛选归一化后的元素表——好读、能直接拿来定位；
        // `raw_pages/` 是没动过的原文——用来回答"这个元素是被我们筛掉了，还是压根没采到"，
        // 也是将来页面改版时做脚本持久化的底料。拿不到就跳过：它是参照物，缺了不影响这一步。
        //
        // ⚠️ **按前缀扫，不要写扩展名白名单**。早先是 `for ext in ["html","xml"]`，
        // 于是 iOS 模拟器的 `.json` 原文**静静地没了**——不报错、不提示，
        // 只有人对着报告数目录才发现。新驱动用什么后缀都不该由这里来记。
        if let Some(src_raw) = Self::find_raw_page_impl(workarea.dir()) {
            if std::fs::create_dir_all(&self.raw_page_dir).is_ok() {
                let ext = src_raw.extension().and_then(|e| e.to_str()).unwrap_or("txt");
                let _ = std::fs::copy(&src_raw, self.raw_page_dir.join(format!("step_{:03}.{}", seq, ext)));
            }
        }

        (screenshot, xml)
    }

    /// 把本批结果**追加**进任务的 log.json（Task 布局）。
    /// 读-改-写而不是覆盖：同一个任务目录会被反复调用，覆盖等于把前面几批的证据抹掉。
    pub fn append_task_log(&self, result: &crate::ExecutionResult) -> Result<PathBuf> {
        let log_path = self.run_dir.join("log.json");
        let mut task = crate::TaskLog::load(&log_path);
        task.batches.push(serde_json::from_value(
            serde_json::to_value(result).map_err(TkeError::JsonError)?,
        ).map_err(TkeError::JsonError)?);
        let json = serde_json::to_string_pretty(&task).map_err(TkeError::JsonError)?;
        std::fs::write(&log_path, json).map_err(TkeError::IoError)?;
        Ok(log_path)
    }

    /// 写入运行日志 log.json
    pub fn write_log<T: serde::Serialize>(&self, log: &T) -> Result<PathBuf> {
        let log_path = self.run_dir.join("log.json");
        let json = serde_json::to_string_pretty(log).map_err(TkeError::JsonError)?;
        std::fs::write(&log_path, json).map_err(TkeError::IoError)?;
        Ok(log_path)
    }
}

/// 加载系统中文字体（按平台尝试常见路径，全部失败返回 None）
fn load_system_font() -> Option<FontVec> {
    let mut candidates: Vec<&str> = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.extend([
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ]);
    } else if cfg!(target_os = "windows") {
        candidates.extend([
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ]);
    } else {
        candidates.extend([
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]);
    }

    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            // ttc 字体集合取第 0 个字体
            if let Ok(font) = FontVec::try_from_vec_and_index(data, 0) {
                return Some(font);
            }
        }
    }

    None
}

/// 标注截图：顶部横幅（操作描述+成败）+ 元素框（红）+ 点击坐标点（蓝圈白心）+ 滑动轨迹线
fn annotate_screenshot(
    src: &Path,
    dst: &Path,
    trace: &ActionTrace,
    banner_text: &str,
    success: bool,
    font: Option<&FontVec>,
) -> Result<()> {
    let mut img: RgbaImage = image::open(src)
        .map_err(|e| TkeError::ImageError(format!("打开截图失败: {}", e)))?
        .to_rgba8();

    let red = Rgba([255u8, 59, 48, 255]);
    let green = Rgba([52u8, 199, 89, 255]);
    let blue = Rgba([0u8, 122, 255, 255]);
    let white = Rgba([255u8, 255, 255, 255]);

    // 1. 顶部横幅：半透明黑底 + 左侧成败色条 + 操作描述文字
    let banner_h = (img.height() / 24).clamp(60, 110);
    let (w, _) = img.dimensions();
    for y in 0..banner_h {
        for x in 0..w {
            let p = img.get_pixel_mut(x, y);
            // 70% 黑色叠加
            p.0[0] = (p.0[0] as f32 * 0.3) as u8;
            p.0[1] = (p.0[1] as f32 * 0.3) as u8;
            p.0[2] = (p.0[2] as f32 * 0.3) as u8;
        }
    }
    // 左侧成败色条
    let bar_color = if success { green } else { red };
    for y in 0..banner_h {
        for x in 0..(w.min(14)) {
            img.put_pixel(x, y, bar_color);
        }
    }
    // 操作描述文字（无系统字体则跳过）
    if let Some(font) = font {
        let scale = PxScale::from(banner_h as f32 * 0.45);
        let ty = (banner_h as f32 * 0.25) as i32;
        draw_text_mut(&mut img, white, 30, ty, scale, font, banner_text);
    }

    // 2. 元素框（3px 红框，来自运行时实际匹配到的元素）
    if let Some(b) = &trace.bounds {
        if b.is_visible() {
            for i in 0..3i32 {
                let rect = Rect::at(b.x1 - i, b.y1 - i)
                    .of_size((b.width() + 2 * i).max(1) as u32, (b.height() + 2 * i).max(1) as u32);
                draw_hollow_rect_mut(&mut img, rect, red);
            }
        }
    }

    // 3. 滑动轨迹：起点到终点画线
    if trace.points.len() >= 2 {
        let from = trace.points[0];
        let to = trace.points[1];
        for offset in -1..=1i32 {
            draw_line_segment_mut(
                &mut img,
                (from.x as f32 + offset as f32, from.y as f32),
                (to.x as f32 + offset as f32, to.y as f32),
                blue,
            );
        }
    }

    // 4. 操作坐标点（蓝圈白心）
    for p in &trace.points {
        draw_filled_circle_mut(&mut img, (p.x, p.y), 14, blue);
        draw_filled_circle_mut(&mut img, (p.x, p.y), 7, white);
        for r in 18..=20i32 {
            draw_hollow_circle_mut(&mut img, (p.x, p.y), r, blue);
        }
    }

    img.save(dst)
        .map_err(|e| TkeError::ImageError(format!("保存标注截图失败: {}", e)))?;

    Ok(())
}

/// 清理名称中不适合做目录名的字符
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ') { '_' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tke-artifacts-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Task 布局反复创建 = 续写同一目录，编号接着往下排。
    /// 这条要是坏了，第二批会从 step_001 重新开始、**直接覆盖掉第一批的证据**——
    /// 而且不会报任何错，只是报告里少了几步。
    #[test]
    fn task_layout_continues_numbering() {
        let dir = tmpdir("task-seq");
        let a = RunArtifacts::create_with(&dir, "steps", Layout::Task).unwrap();
        assert_eq!(a.run_dir, dir, "Task 布局下 --log 目录就是任务目录，不再套时间戳子目录");
        assert_eq!(a.step_offset, 0);

        // 模拟第一批落了 2 步
        std::fs::create_dir_all(dir.join("screenshots")).unwrap();
        std::fs::write(dir.join("screenshots/step_001.png"), b"x").unwrap();
        std::fs::write(dir.join("screenshots/step_002.png"), b"x").unwrap();

        let b = RunArtifacts::create_with(&dir, "steps", Layout::Task).unwrap();
        assert_eq!(b.step_offset, 2, "第二批应从 step_003 接着排");

        // 只有 pages/ 有残留（截图写失败）时也要接得上
        let dir2 = tmpdir("task-seq-pages");
        std::fs::create_dir_all(dir2.join("pages")).unwrap();
        std::fs::write(dir2.join("pages/step_007.xml"), b"<x/>").unwrap();
        let c = RunArtifacts::create_with(&dir2, "steps", Layout::Task).unwrap();
        assert_eq!(c.step_offset, 7);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    /// Timestamped 布局(run/flow/harness)不受影响：每次另起一个带时间戳的子目录
    #[test]
    fn timestamped_layout_makes_own_dir() {
        let dir = tmpdir("ts");
        let a = RunArtifacts::create_with(&dir, "foo", Layout::Timestamped).unwrap();
        assert_ne!(a.run_dir, dir);
        assert!(a.run_dir.starts_with(&dir));
        assert!(
            a.run_dir.file_name().unwrap().to_string_lossy().starts_with("foo_"),
            "目录名应是 <名>_<时间戳>"
        );
        assert_eq!(a.step_offset, 0, "回放不续写，永远从 step_001 开始");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// log.json 累积：新格式追加，且能读旧的单批格式（不丢历史证据）
    #[test]
    fn task_log_accumulates_and_reads_legacy() {
        let dir = tmpdir("tasklog");
        let a = RunArtifacts::create_with(&dir, "steps", Layout::Task).unwrap();
        let mk = |name: &str| crate::ExecutionResult {
            success: true,
            case_id: String::new(),
            script_name: name.to_string(),
            start_time: "2026-08-15T10:00:00+10:00".into(),
            end_time: String::new(),
            steps: Vec::new(),
            error: None,
            script_path: None,
            run_dir: None,
            launched_packages: Vec::new(),
            device: Some("web".into()),
            device_label: Some("Chrome（无头）".into()),
        };
        a.append_task_log(&mk("first")).unwrap();
        a.append_task_log(&mk("second")).unwrap();
        let t = crate::TaskLog::load(&dir.join("log.json"));
        assert_eq!(t.batches.len(), 2, "第二批应追加而不是覆盖");
        assert_eq!(t.batches[0].script_name, "first");

        // 旧格式：整个文件就是一个 ExecutionResult
        let legacy = tmpdir("tasklog-legacy");
        let p = legacy.join("log.json");
        std::fs::write(&p, serde_json::to_string(&mk("legacy")).unwrap()).unwrap();
        let t2 = crate::TaskLog::load(&p);
        assert_eq!(t2.batches.len(), 1, "旧的单批 log.json 也要读得出来");
        assert_eq!(t2.batches[0].script_name, "legacy");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&legacy);
    }
}

#[cfg(test)]
mod raw_page_tests {
    use super::*;

    /// 原文收集**不能认扩展名**：认了就得维护一份清单，而漏掉的那次是静默的
    /// （iOS 模拟器的 `.json` 就这么没进过报告，`raw_pages/` 空着也不报错）
    #[test]
    fn picks_up_raw_page_whatever_the_extension_is() {
        for ext in ["html", "xml", "json", "以后某个新驱动的后缀"] {
            let tmp = std::env::temp_dir().join(format!("tke-rawpage-test-{}", ext.len()));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).unwrap();
            std::fs::write(tmp.join(format!("current_raw_page.{}", ext)), "原文").unwrap();

            let found = RunArtifacts::find_raw_page_impl(&tmp);
            assert!(found.is_some(), "扩展名 .{} 的原文没被找到", ext);
            assert_eq!(std::fs::read_to_string(found.unwrap()).unwrap(), "原文");
            let _ = std::fs::remove_dir_all(&tmp);
        }
    }

    /// 没有原文时安静跳过——它是参照物，缺了不该让这一步失败
    #[test]
    fn no_raw_page_is_fine() {
        let tmp = std::env::temp_dir().join("tke-rawpage-test-none");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(RunArtifacts::find_raw_page_impl(&tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
