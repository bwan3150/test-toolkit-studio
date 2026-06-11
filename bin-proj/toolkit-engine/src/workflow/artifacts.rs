// 运行产物管理 - 每次运行留下完整记录用于定位问题
// runs/<时间戳>_<脚本名>/
//   ├── run.json            完整执行日志（脚本path、时间戳、每步成败/报错/耗时）
//   └── steps/
//       ├── step_001.png    每步标注截图（元素框 + 点击坐标点）
//       └── step_001.xml    每步 UI 结构文件

use crate::{Result, TkeError, ActionTrace};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_hollow_circle_mut, draw_hollow_rect_mut, draw_line_segment_mut};
use imageproc::rect::Rect;
use std::path::{Path, PathBuf};

/// 单次运行的产物目录
pub struct RunArtifacts {
    /// 运行根目录 runs/<时间戳>_<名称>/
    pub run_dir: PathBuf,
    /// 每步产物目录 steps/
    steps_dir: PathBuf,
}

impl RunArtifacts {
    /// 创建运行产物目录
    /// runs_root 缺省为 <project>/runs
    pub fn create(project_path: &Path, runs_root: Option<&Path>, name: &str) -> Result<Self> {
        let root = runs_root
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| project_path.join("runs"));

        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let run_dir = root.join(format!("{}_{}", timestamp, sanitize(name)));
        let steps_dir = run_dir.join("steps");

        // steps/ 目录按需创建（flow 根目录等不保存步骤产物的运行不需要）
        std::fs::create_dir_all(&run_dir).map_err(TkeError::IoError)?;

        Ok(Self { run_dir, steps_dir })
    }

    /// 在已有目录下创建（flow 中每个脚本的子目录）
    pub fn create_in(parent: &Path, name: &str) -> Result<Self> {
        let run_dir = parent.join(sanitize(name));
        std::fs::create_dir_all(&run_dir).map_err(TkeError::IoError)?;
        let steps_dir = run_dir.join("steps");
        Ok(Self { run_dir, steps_dir })
    }

    /// 保存单步产物：从 workarea 复制截图（标注后）和 XML
    /// 返回 (截图相对路径, XML相对路径)
    pub fn save_step(
        &self,
        project_path: &Path,
        step_index: usize,
        trace: &ActionTrace,
    ) -> (Option<String>, Option<String>) {
        // 按需创建 steps/ 目录
        if std::fs::create_dir_all(&self.steps_dir).is_err() {
            return (None, None);
        }
        let workarea = project_path.join("workarea");
        let src_png = workarea.join("current_screenshot.png");
        let src_xml = workarea.join("current_ui_tree.xml");

        let png_name = format!("step_{:03}.png", step_index + 1);
        let xml_name = format!("step_{:03}.xml", step_index + 1);

        // 截图：标注元素框 + 点击点后保存（标注失败则原样复制）
        let screenshot = if src_png.exists() {
            let dst = self.steps_dir.join(&png_name);
            let ok = annotate_screenshot(&src_png, &dst, trace)
                .or_else(|_| std::fs::copy(&src_png, &dst).map(|_| ()))
                .is_ok();
            ok.then(|| format!("steps/{}", png_name))
        } else {
            None
        };

        // XML：原样复制
        let xml = if src_xml.exists() {
            let dst = self.steps_dir.join(&xml_name);
            std::fs::copy(&src_xml, &dst)
                .ok()
                .map(|_| format!("steps/{}", xml_name))
        } else {
            None
        };

        (screenshot, xml)
    }

    /// 写入运行日志 run.json
    pub fn write_log<T: serde::Serialize>(&self, log: &T) -> Result<PathBuf> {
        let log_path = self.run_dir.join("run.json");
        let json = serde_json::to_string_pretty(log).map_err(TkeError::JsonError)?;
        std::fs::write(&log_path, json).map_err(TkeError::IoError)?;
        Ok(log_path)
    }
}

/// 标注截图：画元素框（红）+ 点击坐标点（蓝圈白心）+ 滑动轨迹线
fn annotate_screenshot(src: &Path, dst: &Path, trace: &ActionTrace) -> Result<()> {
    let mut img: RgbaImage = image::open(src)
        .map_err(|e| TkeError::ImageError(format!("打开截图失败: {}", e)))?
        .to_rgba8();

    let red = Rgba([255u8, 59, 48, 255]);
    let blue = Rgba([0u8, 122, 255, 255]);
    let white = Rgba([255u8, 255, 255, 255]);

    // 1. 元素框（3px 红框）
    if let Some(b) = &trace.bounds {
        if b.is_visible() {
            for i in 0..3i32 {
                let rect = Rect::at(b.x1 - i, b.y1 - i)
                    .of_size((b.width() + 2 * i).max(1) as u32, (b.height() + 2 * i).max(1) as u32);
                draw_hollow_rect_mut(&mut img, rect, red);
            }
        }
    }

    // 2. 滑动轨迹：起点到终点画线
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

    // 3. 操作坐标点（蓝圈白心）
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
