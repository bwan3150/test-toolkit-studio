// 已知元素匹配：拿当前页面元素对照元素库，命中的标注库里的 name，
// 让 AI 复用、不重复造新元素。匹配只用结构通道(content_desc/text/resource_id)
// 与 ocr 文本——图像通道不在此匹配（识别太慢）。

use std::path::Path;

use crate::{Platform, UIElement};

/// 对每个当前元素返回命中的库元素名（None=未知/库中没有）
pub fn match_known(elements: &[UIElement], platform: Platform, lib_path: &Path) -> Vec<Option<String>> {
    let none = || vec![None; elements.len()];
    let lib: serde_json::Value = match std::fs::read_to_string(lib_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return none(),
        },
        Err(_) => return none(),
    };
    let map = match lib.get("elements").and_then(|m| m.as_object()) {
        Some(m) => m,
        None => return none(),
    };
    let chan = match platform {
        Platform::Android => "android",
        Platform::Ios => "ios",
        Platform::Web => "web",
    };
    elements
        .iter()
        .map(|e| {
            map.iter()
                .find(|(_, def)| matches(e, def, chan))
                .map(|(name, _)| name.clone())
        })
        .collect()
}

fn norm(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|t| !t.is_empty())
}

/// 当前元素 e 是否命中库元素 def（按平台通道 + ocr 文本精确匹配）
fn matches(e: &UIElement, def: &serde_json::Value, chan: &str) -> bool {
    let ecd = norm(e.content_desc.as_deref());
    let etx = norm(e.text.as_deref());
    let erid = norm(e.resource_id.as_deref());

    // 结构通道：按平台字段名做精确匹配
    let c = &def[chan];
    if c.is_object() {
        // (元素值, 库通道里对应的字段名)
        let pairs: [(Option<&str>, &str); 3] = match chan {
            "android" => [(ecd, "content_desc"), (etx, "text"), (erid, "resource_id")],
            "ios" => [(ecd, "label"), (etx, "value"), (erid, "name")],
            _ => [(ecd, "aria"), (etx, "text"), (erid, "id")],
        };
        for (ev, key) in pairs {
            if let (Some(ev), Some(dv)) = (ev, norm(c.get(key).and_then(|v| v.as_str()))) {
                if dv == ev {
                    return true;
                }
            }
        }
    }

    // ocr 文本：与元素可见文字相等
    if let Some(ocr) = norm(def.get("ocr").and_then(|v| v.as_str())) {
        if [ecd, etx].into_iter().flatten().any(|ev| ev == ocr) {
            return true;
        }
    }

    false
}
