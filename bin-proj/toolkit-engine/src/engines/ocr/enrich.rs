// OCR 增强：把识别出的文字按坐标合入元素表
// - 给无 text/content-desc 的元素回填 OCR 文字（让纯图标也有可读标签，AI 才认得出）
// - XML 完全没覆盖到的文字区，追加为可点击伪元素
// 失败不致命：调用方降级（无 OCR 即用原始元素表）。

use std::error::Error as StdError;

use crate::{Bounds, Point, UIElement};

use super::{ocr, OcrText};

type OcrErr = Box<dyn StdError + Send + Sync>;

/// OCR 来源：离线 tesseract 或在线服务 URL
#[derive(Debug, Clone)]
pub struct OcrSource {
    /// true=在线服务，false=离线 tesseract
    pub online: bool,
    /// 在线=完整 URL；离线=语言代码（如 chi_sim+eng）
    pub param: String,
}

impl OcrSource {
    /// 在线服务（给定 URL）。
    /// `0.0.0.0` 是**服务端监听地址**、不是客户端可连接的地址——配置里常被照抄服务端的
    /// bind 地址误填进来（连它在多数系统上直接失败），自动改写为本机回环 127.0.0.1。
    pub fn online_url(url: String) -> Self {
        let url = url.replacen("//0.0.0.0:", "//127.0.0.1:", 1).replacen("//0.0.0.0/", "//127.0.0.1/", 1);
        Self { online: true, param: url }
    }

    /// 离线 tesseract（中英文）
    pub fn offline() -> Self {
        Self { online: false, param: "chi_sim+eng".to_string() }
    }

    /// 解析 --ocr 值（不含 "online" 关键字，那个需默认 URL 兜底，见 resolve_ocr）：
    /// - "offline" → 离线 tesseract
    /// - "http(s)://..." → 在线服务（该 URL）
    /// - 其它非空串 → 当作离线语言代码
    /// 空串返回 None（未启用）。
    pub fn parse(spec: &str) -> Option<Self> {
        let s = spec.trim();
        if s.is_empty() {
            return None;
        }
        if s.eq_ignore_ascii_case("offline") {
            Some(Self::offline())
        } else if s.starts_with("http://") || s.starts_with("https://") {
            Some(Self::online_url(s.to_string()))
        } else {
            Some(Self { online: false, param: s.to_string() })
        }
    }
}

/// 解析 --ocr 值，"online" 关键字用 default_url 兜底（取自配置 ocr_url）。
/// default_url 为空且 spec="online" 时返回 None（无可用服务，跳过 OCR）。
pub fn resolve_ocr(spec: &str, default_url: &str) -> Option<OcrSource> {
    if spec.trim().eq_ignore_ascii_case("online") {
        if default_url.trim().is_empty() {
            return None;
        }
        return Some(OcrSource::online_url(default_url.trim().to_string()));
    }
    OcrSource::parse(spec)
}

/// 用 OCR 文字增强元素列表（异步：在线模式走网络）。
/// 返回 (回填数, 新增伪元素数)。出错向上抛，调用方决定降级。
/// 返回 (回填数 filled, 新增伪元素数 added, OCR 识别到的文字总数 recognized)。
/// recognized 用于如实展示 OCR 状态：即使 filled/added 都为 0，只要 recognized>0 就说明 OCR 在工作（文字都已并入已有元素）。
/// OCR 熔断：连续失败达到阈值后，本进程剩余时间不再真调 OCR（立即返回错误，不再等超时）——
/// 服务挂掉时每轮采集各陪一次超时太伤（40 轮探索 = 白等几分钟）。失败/熔断都如实报错、
/// 不静默（沿用"OCR 透明化"原则）；中途成功一次即清零计数；服务恢复后重启 tke 生效。
static OCR_CONSECUTIVE_FAILS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const OCR_TRIP_THRESHOLD: u32 = 3;

pub async fn enrich_with_ocr(
    elements: &mut Vec<UIElement>,
    image_data: &[u8],
    src: &OcrSource,
) -> Result<(usize, usize, usize), OcrErr> {
    use std::sync::atomic::Ordering;
    let fails = OCR_CONSECUTIVE_FAILS.load(Ordering::Relaxed);
    if fails >= OCR_TRIP_THRESHOLD {
        return Err(format!("OCR 已熔断（此前连续失败 {} 次），本次运行不再调用；服务恢复后重启生效", fails).into());
    }
    match ocr(image_data, src.online, &src.param).await {
        Ok(result) => {
            OCR_CONSECUTIVE_FAILS.store(0, Ordering::Relaxed);
            let recognized = result.texts.len();
            let (filled, added) = merge(elements, &result.texts);
            Ok((filled, added, recognized))
        }
        Err(e) => {
            let n = OCR_CONSECUTIVE_FAILS.fetch_add(1, Ordering::Relaxed) + 1;
            if n >= OCR_TRIP_THRESHOLD {
                Err(format!("{}（连续失败已达 {} 次，本次运行剩余轮次熔断停用 OCR）", e, n).into())
            } else {
                Err(e)
            }
        }
    }
}

/// 纯合并逻辑（无 IO，便于测试）
fn merge(elements: &mut Vec<UIElement>, texts: &[OcrText]) -> (usize, usize) {
    // OCR 文本框 → (Bounds, 文字)，过滤空文字 / 不可见框
    let boxes: Vec<(Bounds, &str)> = texts
        .iter()
        .filter_map(|t| {
            let s = t.text.trim();
            if s.is_empty() {
                return None;
            }
            let b = poly_to_bounds(&t.bbox);
            b.is_visible().then_some((b, s))
        })
        .collect();
    let mut used = vec![false; boxes.len()];

    // 1) 回填：无 text/content-desc 的元素 ← 中心相互包含的 OCR 文字（取中心最近者）
    let mut filled = 0usize;
    for el in elements.iter_mut() {
        let unlabeled = el.text.as_deref().unwrap_or("").trim().is_empty()
            && el.content_desc.as_deref().unwrap_or("").trim().is_empty();
        if !unlabeled || !el.bounds.is_visible() {
            continue;
        }
        let ec = el.bounds.center();
        let mut best: Option<(usize, i64)> = None;
        for (i, (b, _)) in boxes.iter().enumerate() {
            if used[i] {
                continue;
            }
            let bc = b.center();
            // OCR 框中心落在元素内，或元素中心落在 OCR 框内 → 视为同一处
            if !contains(&el.bounds, &bc) && !contains(b, &ec) {
                continue;
            }
            let d = dist2(&ec, &bc);
            if best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            el.text = Some(boxes[i].1.to_string());
            used[i] = true;
            filled += 1;
        }
    }

    // 2) 补充伪元素：未被消费、且中心不落在任何元素框内的 OCR 文字（XML 漏掉的）
    let mut added = 0usize;
    let mut next_index = elements.iter().map(|e| e.index).max().unwrap_or(0) + 1;
    for (i, (b, text)) in boxes.iter().enumerate() {
        if used[i] {
            continue;
        }
        let bc = b.center();
        if elements.iter().any(|e| contains(&e.bounds, &bc)) {
            continue;
        }
        elements.push(ocr_element(next_index, b.clone(), text));
        next_index += 1;
        added += 1;
    }

    (filled, added)
}

/// 多边形 bbox → 轴对齐外接 Bounds
fn poly_to_bounds(poly: &[[f32; 2]]) -> Bounds {
    if poly.is_empty() {
        return Bounds::new(0, 0, 0, 0);
    }
    let (mut x1, mut y1, mut x2, mut y2) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in poly {
        x1 = x1.min(p[0]);
        y1 = y1.min(p[1]);
        x2 = x2.max(p[0]);
        y2 = y2.max(p[1]);
    }
    Bounds::new(x1 as i32, y1 as i32, x2 as i32, y2 as i32)
}

fn contains(b: &Bounds, p: &Point) -> bool {
    p.x >= b.x1 && p.x <= b.x2 && p.y >= b.y1 && p.y <= b.y2
}

fn dist2(a: &Point, b: &Point) -> i64 {
    let dx = (a.x - b.x) as i64;
    let dy = (a.y - b.y) as i64;
    dx * dx + dy * dy
}

/// 由 OCR 文字区构造一个可点击伪元素
fn ocr_element(index: usize, bounds: Bounds, text: &str) -> UIElement {
    UIElement {
        index,
        class_name: "OcrText".to_string(),
        bounds,
        text: Some(text.to_string()),
        content_desc: None,
        resource_id: None,
        hint: None,
        clickable: true,
        checkable: false,
        checked: false,
        focusable: false,
        focused: false,
        scrollable: false,
        selected: false,
        enabled: true,
        xpath: None,
        z_index: None,
        parent_index: None,
        depth: 0,
        sibling_index: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(index: usize, x1: i32, y1: i32, x2: i32, y2: i32, text: Option<&str>) -> UIElement {
        let mut e = ocr_element(index, Bounds::new(x1, y1, x2, y2), text.unwrap_or(""));
        e.class_name = "ImageView".to_string();
        e.text = text.map(|s| s.to_string());
        e
    }

    fn ocr_text(text: &str, x1: i32, y1: i32, x2: i32, y2: i32) -> OcrText {
        let (x1, y1, x2, y2) = (x1 as f32, y1 as f32, x2 as f32, y2 as f32);
        OcrText {
            text: text.to_string(),
            bbox: vec![[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
            confidence: 0.9,
        }
    }

    #[test]
    fn 回填无标签元素() {
        // 底部图标无 text；OCR 在其范围内识别出"我的"
        let mut els = vec![el(0, 880, 2250, 980, 2340, None)];
        let texts = vec![ocr_text("我的", 890, 2300, 970, 2330)];
        let (filled, added) = merge(&mut els, &texts);
        assert_eq!((filled, added), (1, 0));
        assert_eq!(els[0].text.as_deref(), Some("我的"));
    }

    #[test]
    fn 已有标签不覆盖() {
        let mut els = vec![el(0, 0, 0, 100, 100, Some("首页"))];
        let texts = vec![ocr_text("别的", 10, 10, 90, 90)];
        let (filled, _) = merge(&mut els, &texts);
        assert_eq!(filled, 0);
        assert_eq!(els[0].text.as_deref(), Some("首页"));
    }

    #[test]
    fn 漏掉的文字补伪元素() {
        let mut els = vec![el(0, 0, 0, 100, 100, Some("首页"))];
        let texts = vec![ocr_text("浮层文字", 500, 500, 700, 560)];
        let (_, added) = merge(&mut els, &texts);
        assert_eq!(added, 1);
        assert_eq!(els.last().unwrap().text.as_deref(), Some("浮层文字"));
        assert!(els.last().unwrap().clickable);
    }
}
