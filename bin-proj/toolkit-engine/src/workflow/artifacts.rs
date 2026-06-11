// 运行产物管理 - 每次运行留下完整记录用于定位问题
// 仅在指定 --log <dir> 时启用，目录结构：
// <log>/<时间戳>_<脚本名>/
//   ├── log.json              完整执行日志（脚本path、时间戳、每步成败/报错/耗时）
//   ├── screenshots/
//   │   └── step_001.png      每步标注截图（顶部横幅=操作内容/成败 + 元素框 + 点击坐标点）
//   └── page/
//       └── step_001.xml      每步页面结构文件（xml/wda/dom）

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

/// 单次运行的产物目录
pub struct RunArtifacts {
    /// 运行根目录 <log>/<时间戳>_<名称>/
    pub run_dir: PathBuf,
    /// 截图序列目录
    screenshots_dir: PathBuf,
    /// 页面结构序列目录
    page_dir: PathBuf,
    /// 文字标注字体（系统字体，加载失败则不画文字）
    font: Option<FontVec>,
}

impl RunArtifacts {
    /// 在 log 根目录下创建本次运行的产物目录
    pub fn create(log_root: &Path, name: &str) -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let run_dir = log_root.join(format!("{}_{}", timestamp, sanitize(name)));
        Self::create_at(run_dir)
    }

    /// 在已有目录下创建（flow 中每个脚本的子目录）
    pub fn create_in(parent: &Path, name: &str) -> Result<Self> {
        Self::create_at(parent.join(sanitize(name)))
    }

    fn create_at(run_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&run_dir).map_err(TkeError::IoError)?;
        let screenshots_dir = run_dir.join("screenshots");
        let page_dir = run_dir.join("page");
        Ok(Self {
            run_dir,
            screenshots_dir,
            page_dir,
            font: load_system_font(),
        })
    }

    /// 保存单步产物：从工作区复制截图（标注后）和页面结构文件
    /// label 为本步操作描述（如 "返回" / "点击 [{Devices入口}]"），画在截图顶部横幅
    /// 返回 (截图相对路径, 结构文件相对路径)
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

        let png_name = format!("step_{:03}.png", step_index + 1);
        let xml_name = format!("step_{:03}.xml", step_index + 1);

        // 截图：标注横幅 + 元素框 + 点击点后保存（标注失败则原样复制）
        let screenshot = if src_png.exists() && std::fs::create_dir_all(&self.screenshots_dir).is_ok() {
            let dst = self.screenshots_dir.join(&png_name);
            let banner = format!(
                "Step {} {} | {}",
                step_index + 1,
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

        // 页面结构文件：原样复制
        let xml = if src_xml.exists() && std::fs::create_dir_all(&self.page_dir).is_ok() {
            let dst = self.page_dir.join(&xml_name);
            std::fs::copy(&src_xml, &dst)
                .ok()
                .map(|_| format!("page/{}", xml_name))
        } else {
            None
        };

        (screenshot, xml)
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
