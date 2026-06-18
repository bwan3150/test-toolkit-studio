// Element 工具 - 元素库管理
//
// add: 按坐标从当前页面取元素，自动落库——
//   ① 当前平台的结构通道（android/ios/web，按 -d 推断）
//   ② img 通道：按元素 bounds 从截图 crop 出模板图，存 <库目录>/img/<元素名>.png
//   ③ ocr 通道：结构中的文本，无文本时对 crop 图跑 OCR 兜底
// 已有元素则合并：只更新当前平台通道；img/ocr 仅在为 null 时填充（--force 强制覆盖）

use crate::{Result, TkeError, Controller, Fetcher, Platform, UIElement, Bounds};
use crate::utils::Workarea;
use std::path::Path;

// 元素库默认查找已收敛到参数层 utils::params（单一来源）；本模块只接收解析好的库路径。

/// 元素名 → 安全文件名（保留中文/字母/数字，其余替换为 _）
fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// 空字符串归一化为 None
fn non_empty(s: &Option<String>) -> Option<String> {
    s.as_ref().filter(|v| !v.trim().is_empty()).cloned()
}

/// 按平台构造结构通道 JSON（从归一化 XML 解析出的 UIElement 反推各平台字段）
fn build_channel(platform: Platform, el: &UIElement) -> (&'static str, serde_json::Value) {
    let text = non_empty(&el.text);
    let desc = non_empty(&el.content_desc);
    let rid = non_empty(&el.resource_id);
    let class = if el.class_name.trim().is_empty() { None } else { Some(el.class_name.clone()) };

    match platform {
        Platform::Android => (
            "android",
            serde_json::json!({
                "xpath": el.xpath,
                "resource_id": rid,
                "text": text,
                "content_desc": desc,
                "class_name": class,
            }),
        ),
        // 归一化映射的逆向: resource-id=name, content-desc=label, text=value|label
        Platform::Ios => (
            "ios",
            serde_json::json!({
                "xpath": null,
                "name": rid,
                "label": desc,
                // text 与 label 相同说明 value 为空（归一化时 text=value||label）
                "value": if text == desc { serde_json::Value::Null } else { serde_json::json!(text) },
                "class_name": class,
            }),
        ),
        // 归一化映射的逆向: resource-id=DOM id, content-desc=aria-label, class=tag
        Platform::Web => (
            "web",
            serde_json::json!({
                "css": null,
                "xpath": null,
                "id": rid,
                "text": text,
                "aria": desc,
                "tag": class,
            }),
        ),
    }
}

/// 对图片文件跑 OCR（tke 自调用子进程，任何失败/崩溃都安全返回 None）
fn ocr_via_subprocess(image_path: &Path) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let output = std::process::Command::new(exe)
        .arg("ocr")
        .arg("--image")
        .arg(image_path)
        .output()
        .ok()?;
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let joined = result["texts"]
        .as_array()?
        .iter()
        .filter_map(|t| t["text"].as_str())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() { None } else { Some(joined) }
}

/// 按坐标添加元素到元素库
/// 返回结果 JSON（含落库内容摘要）
pub async fn add_element(
    device_id: String,
    lib_path: &Path,
    name: &str,
    desc: Option<String>,
    x: i32,
    y: i32,
    cached: bool,
    force: bool,
) -> Result<serde_json::Value> {
    let platform = Platform::from_device(Some(&device_id));
    let workarea = Workarea::for_device(Some(&device_id))?;

    // 1. 采集当前页面（--cached 跳过）
    if !cached {
        let controller = Controller::new(Some(device_id))?;
        controller.capture_ui_state(&workarea).await?;
    }

    // 2. 解析页面元素，取包含该坐标的最小面积元素；
    //    带标识（text/desc/id）的元素优先——略大（≤4 倍面积）的有标识元素
    //    好过紧贴的无标识容器（如文字外层的装饰 div）
    let fetcher = Fetcher::new();
    let elements = fetcher.fetch_elements_from_file(&workarea.ui_tree_path())?;
    fn area(e: &UIElement) -> i64 {
        let b = &e.bounds;
        (b.x2 - b.x1) as i64 * (b.y2 - b.y1) as i64
    }
    fn has_identifier(e: &UIElement) -> bool {
        non_empty(&e.text).is_some()
            || non_empty(&e.content_desc).is_some()
            || non_empty(&e.resource_id).is_some()
    }
    let containing: Vec<&UIElement> = elements
        .iter()
        .filter(|e| {
            let b = &e.bounds;
            x >= b.x1 && x <= b.x2 && y >= b.y1 && y <= b.y2 && b.x2 > b.x1 && b.y2 > b.y1
        })
        .collect();
    let smallest = containing.iter().min_by_key(|e| area(e)).copied();
    let smallest_with_id = containing
        .iter()
        .filter(|e| has_identifier(e))
        .min_by_key(|e| area(e))
        .copied();
    let target = match (smallest, smallest_with_id) {
        (Some(s), Some(sid)) if area(sid) <= area(s) * 4 => sid,
        (Some(s), _) => s,
        (None, _) => {
            return Err(TkeError::ElementNotFound(format!(
                "坐标 ({}, {}) 处没有可用元素", x, y
            )));
        }
    };

    // 3. 结构通道
    let (channel_key, channel_value) = build_channel(platform, target);

    // 4. img 通道：从截图 crop 元素模板图（lib_path 为参数层解析好的库路径）
    let lib_dir = lib_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let img_dir = lib_dir.join("img");
    std::fs::create_dir_all(&img_dir).map_err(TkeError::IoError)?;

    let screenshot = image::open(workarea.screenshot_path())
        .map_err(|e| TkeError::DeviceError(format!("读取工作区截图失败: {}", e)))?;
    let (sw, sh) = (screenshot.width() as i32, screenshot.height() as i32);
    let b = &target.bounds;
    let (cx1, cy1) = (b.x1.clamp(0, sw - 1), b.y1.clamp(0, sh - 1));
    let (cx2, cy2) = (b.x2.clamp(cx1 + 1, sw), b.y2.clamp(cy1 + 1, sh));
    let crop = screenshot.crop_imm(cx1 as u32, cy1 as u32, (cx2 - cx1) as u32, (cy2 - cy1) as u32);

    let img_file = format!("img/{}.png", safe_filename(name));
    let img_abs = lib_dir.join(&img_file);
    crop.save(&img_abs)
        .map_err(|e| TkeError::DeviceError(format!("保存元素模板图失败: {}", e)))?;

    // 5. ocr 通道：结构文本优先，无文本时对 crop 图跑 OCR 兜底
    //    （OCR 走 tke 子进程隔离：tesseract 环境问题导致的崩溃不影响落库主流程）
    //    防护：大容器（>30% 屏幕面积）或过长文本不作为 ocr 通道——整页文字会污染匹配
    let area_ratio = ((cx2 - cx1) as f64 * (cy2 - cy1) as f64) / (sw as f64 * sh as f64);
    let is_container_pick = area_ratio > 0.3;
    let mut ocr_text = non_empty(&target.text).or_else(|| non_empty(&target.content_desc));
    if ocr_text.is_none() && !is_container_pick {
        ocr_text = ocr_via_subprocess(&img_abs).filter(|t| t.chars().count() <= 60);
    }

    // 6. 读取/新建元素库，合并写入（preserve_order 保持文件原有元素顺序）
    // 空文件/空白（如库被清空）按空库处理，不报错——否则每次落库都失败
    let mut lib: serde_json::Value = match std::fs::read_to_string(lib_path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("元素库解析失败 {}: {}", lib_path.display(), e)))?,
        _ => serde_json::json!({ "elements": {} }),
    };
    if !lib["elements"].is_object() {
        lib["elements"] = serde_json::json!({});
    }

    let existed = lib["elements"][name].is_object();
    if !existed {
        // 新元素：所有通道键齐全（可 null 不可缺）
        lib["elements"][name] = serde_json::json!({
            "desc": null,
            "android": null,
            "ios": null,
            "web": null,
            "img": null,
            "ocr": null,
        });
    }
    let entry = &mut lib["elements"][name];

    if let Some(d) = &desc {
        entry["desc"] = serde_json::json!(d);
    }
    // 当前平台通道更新（本次采集是最新事实）；
    // 但不允许无标识的新通道覆盖已有的有效通道（class/tag 不算标识）
    let new_has_id = ["resource_id", "text", "content_desc", "name", "label", "value", "id", "aria", "xpath"]
        .iter()
        .any(|k| !channel_value[k].is_null());
    let old_channel_valid = entry[channel_key].is_object() && !new_has_id;
    if !old_channel_valid {
        entry[channel_key] = channel_value.clone();
    }
    // img/ocr 为通用通道：仅在为 null 时填充，--force 强制覆盖。
    // 注意：不删 img_abs——文件名按元素名固定，重复 add 同名元素会把首次存好的模板误删。
    // 已有元素时本次裁的图覆盖了同名文件（内容是当前页，无害），不更新 entry 即可。
    let img_written = force || entry["img"].is_null();
    if img_written {
        entry["img"] = serde_json::json!(img_file);
    }
    let ocr_written = (force || entry["ocr"].is_null()) && ocr_text.is_some();
    if ocr_written {
        entry["ocr"] = serde_json::json!(ocr_text);
    }

    let pretty = serde_json::to_string_pretty(&lib).map_err(TkeError::JsonError)?;
    std::fs::write(lib_path, pretty).map_err(TkeError::IoError)?;

    let mut result = serde_json::json!({
        "success": true,
        "element": name,
        "library": lib_path.to_string_lossy(),
        "updated": existed,
        "channel": channel_key,
        channel_key: channel_value,
        "img": if img_written { serde_json::json!(img_file) } else { serde_json::json!("已有, 未覆盖 (--force 覆盖)") },
        "ocr": if ocr_written { serde_json::json!(ocr_text) } else { serde_json::Value::Null },
        "bounds": target.bounds,
    });
    if is_container_pick {
        result["hint"] = serde_json::json!(
            "该坐标命中的是大容器（无更小的可见元素），建议换更精确的坐标或依赖 img 通道"
        );
    }
    Ok(result)
}

/// 只更新某元素的 desc（探索结束后据实际作用统一回写）。元素不存在则跳过。
pub fn set_element_desc(lib_path: &Path, name: &str, desc: &str) -> Result<()> {
    let mut lib: serde_json::Value = match std::fs::read_to_string(lib_path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("元素库解析失败 {}: {}", lib_path.display(), e)))?,
        _ => return Ok(()), // 空库/无文件：无可更新元素
    };
    if lib["elements"][name].is_object() {
        lib["elements"][name]["desc"] = serde_json::json!(desc);
        let pretty = serde_json::to_string_pretty(&lib).map_err(TkeError::JsonError)?;
        std::fs::write(lib_path, pretty).map_err(TkeError::IoError)?;
    }
    Ok(())
}

/// 元素改名：把库里 old 这一项移到 new 名下（保留其全部通道/截图/desc）。
/// 返回 true=改名成功；old 不存在或 new 已被占用则返回 false（不覆盖已有元素）。
/// 用于 AI 在运行/验证时发现某已知元素当初起错了名（如把 logo 当成导航选项），就地纠正。
pub fn rename_element(lib_path: &Path, old: &str, new: &str) -> Result<bool> {
    if old == new || new.trim().is_empty() {
        return Ok(false);
    }
    let mut lib: serde_json::Value = match std::fs::read_to_string(lib_path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("元素库解析失败 {}: {}", lib_path.display(), e)))?,
        _ => return Ok(false),
    };
    let elements = &mut lib["elements"];
    if !elements[old].is_object() || elements[new].is_object() {
        return Ok(false); // old 不存在 或 new 已占用
    }
    if let Some(obj) = elements.as_object_mut() {
        if let Some(entry) = obj.remove(old) {
            obj.insert(new.to_string(), entry);
        }
    }
    let pretty = serde_json::to_string_pretty(&lib).map_err(TkeError::JsonError)?;
    std::fs::write(lib_path, pretty).map_err(TkeError::IoError)?;
    Ok(true)
}

/// OCR 通道写入方式（add_element_target 用）
pub enum OcrChannel {
    /// 用显式文本（OCR 元素的识别文字）
    Text(String),
    /// 对裁剪图跑 OCR 兜底（结构元素无结构文本时）
    FromCrop,
    /// 不写 ocr（纯视觉元素）
    None,
}

/// 按"已确定的目标"落库——AI harness 三级降级用，**不回查 XML**：
/// - `structure=Some(el)`：结构通道按平台写该元素；`None`：结构通道留空（OCR/视觉元素）
/// - `bounds`：裁剪 img 模板的范围（结构=元素框；视觉=AI 给的框/方块）
/// - `ocr`：ocr 通道写入方式
///
/// 三级对应：结构(Some + Text/FromCrop) / 仅OCR(None + Text) / 仅视觉(None + None)。
/// img/ocr 仅在为 null 时填充，--force 强制覆盖；产出可被 find_auto 跨设备回放。
pub async fn add_element_target(
    device_id: String,
    lib_path: &Path,
    name: &str,
    desc: Option<String>,
    bounds: Bounds,
    structure: Option<&UIElement>,
    ocr: OcrChannel,
    force: bool,
) -> Result<serde_json::Value> {
    let platform = Platform::from_device(Some(&device_id));
    let workarea = Workarea::for_device(Some(&device_id))?;
    let lib_dir = lib_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    // 先读库——决定是否需要写 img/ocr，从而决定要不要裁剪。
    // 空文件/空白（如库被清空）按空库处理，不报错——否则每次落库都失败。
    let mut lib: serde_json::Value = match std::fs::read_to_string(lib_path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("元素库解析失败 {}: {}", lib_path.display(), e)))?,
        _ => serde_json::json!({ "elements": {} }),
    };
    if !lib["elements"].is_object() {
        lib["elements"] = serde_json::json!({});
    }
    let existed = lib["elements"][name].is_object();
    // img/ocr 仅在为 null 时填充（--force 覆盖）；不需写就**根本不裁剪、不碰文件**，
    // 避免重复点击同名元素时把首次存好的模板图误删（同名覆盖+清理孤儿那个老 bug）。
    let need_img = force || lib["elements"][name]["img"].is_null();

    let img_file = format!("img/{}.png", safe_filename(name));
    let img_abs = lib_dir.join(&img_file);

    // 仅在需要写 img 时裁剪并保存
    let mut img_px = serde_json::Value::Null;
    if need_img {
        std::fs::create_dir_all(lib_dir.join("img")).map_err(TkeError::IoError)?;
        let screenshot = image::open(workarea.screenshot_path())
            .map_err(|e| TkeError::DeviceError(format!("读取工作区截图失败: {}", e)))?;
        let (sw, sh) = (screenshot.width() as i32, screenshot.height() as i32);
        // 归一化 bounds：纠正反向、保证最小尺寸——视觉点击/小图标(OCR bbox 仅几像素、
        // 或 click_visual 给的退化 region)否则会裁成 1px 空白图。
        const MIN: i32 = 24;
        let (mut x1, mut y1) = (bounds.x1.min(bounds.x2), bounds.y1.min(bounds.y2));
        let (mut x2, mut y2) = (bounds.x1.max(bounds.x2), bounds.y1.max(bounds.y2));
        if x2 - x1 < MIN {
            let c = (x1 + x2) / 2;
            x1 = c - MIN / 2;
            x2 = c + MIN / 2;
        }
        if y2 - y1 < MIN {
            let c = (y1 + y2) / 2;
            y1 = c - MIN / 2;
            y2 = c + MIN / 2;
        }
        let (cx1, cy1) = (x1.clamp(0, sw - 1), y1.clamp(0, sh - 1));
        let (cx2, cy2) = (x2.clamp(cx1 + 1, sw), y2.clamp(cy1 + 1, sh));
        let crop = screenshot.crop_imm(cx1 as u32, cy1 as u32, (cx2 - cx1) as u32, (cy2 - cy1) as u32);
        crop.save(&img_abs)
            .map_err(|e| TkeError::DeviceError(format!("保存元素模板图失败: {}", e)))?;
        img_px = serde_json::json!([cx2 - cx1, cy2 - cy1]);
    }

    // ocr 通道（FromCrop 兜底仅在本次裁了图时可用）
    let ocr_text = match ocr {
        OcrChannel::Text(t) => non_empty(&Some(t)),
        OcrChannel::FromCrop if need_img => ocr_via_subprocess(&img_abs).filter(|t| t.chars().count() <= 60),
        _ => Option::None,
    };

    // 结构通道（None=不写任何平台结构）
    let channel = structure.map(|el| build_channel(platform, el));

    // 合并写入元素库
    if !existed {
        lib["elements"][name] = serde_json::json!({
            "desc": null, "android": null, "ios": null, "web": null, "img": null, "ocr": null,
        });
    }
    let entry = &mut lib["elements"][name];
    // desc 更新检测：已有元素 + AI 给了与库里不同的 desc → 视为"更新描述"（供人工审核）
    let old_desc = entry["desc"].as_str().map(|s| s.to_string());
    let desc_updated = existed && desc.as_deref().is_some_and(|d| old_desc.as_deref() != Some(d));
    if let Some(d) = &desc {
        entry["desc"] = serde_json::json!(d);
    }
    // 结构通道：仅有结构时写；不让无标识通道覆盖已有有效通道
    if let Some((channel_key, channel_value)) = &channel {
        let new_has_id = ["resource_id", "text", "content_desc", "name", "label", "value", "id", "aria", "xpath"]
            .iter()
            .any(|k| !channel_value[k].is_null());
        let old_channel_valid = entry[*channel_key].is_object() && !new_has_id;
        if !old_channel_valid {
            entry[*channel_key] = channel_value.clone();
        }
    }
    if need_img {
        entry["img"] = serde_json::json!(img_file);
    }
    let ocr_written = (force || entry["ocr"].is_null()) && ocr_text.is_some();
    if ocr_written {
        entry["ocr"] = serde_json::json!(ocr_text);
    }

    let pretty = serde_json::to_string_pretty(&lib).map_err(TkeError::JsonError)?;
    std::fs::write(lib_path, pretty).map_err(TkeError::IoError)?;

    let tier = match (&channel, &ocr_text) {
        (Some(_), _) => "structural",
        (None, Some(_)) => "ocr",
        (None, None) => "visual",
    };
    Ok(serde_json::json!({
        "success": true,
        "element": name,
        "tier": tier,
        "channel": channel.as_ref().map(|(k, _)| *k),
        "img": if need_img { serde_json::json!(img_file) } else { serde_json::Value::Null },
        "img_px": img_px,
        "ocr": ocr_text,
        "bounds": bounds,
        "updated": existed,
        "created": !existed,
        "desc_updated": desc_updated,
        "old_desc": old_desc,
        "new_desc": desc,
    }))
}
