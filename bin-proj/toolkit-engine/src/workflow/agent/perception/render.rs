// 把页面元素列表渲染成喂给 AI 的文本（带序号 = element_id）

use crate::UIElement;

use super::known::KnownHit;

/// 页面元素列表最多展示给 AI 的条数（防 token 爆炸）
const MAX_ELEMENTS: usize = 120;

/// 渲染元素列表：`[序号] 控件描述 @(中心坐标)`
/// known[i] 为该元素在元素库中已存在的记录（命中结构/ocr）：标注其 name 让 AI 复用、
/// 不重复造；并附上库里的 desc，帮 AI 判断这元素是干啥的、要不要操作它。
pub fn render_element_list(elements: &[UIElement], known: &[Option<KnownHit>]) -> String {
    if elements.is_empty() {
        return "(当前页面未解析到任何元素)".to_string();
    }
    let mut out = String::new();
    for (i, el) in elements.iter().enumerate().take(MAX_ELEMENTS) {
        let c = el.center();
        let tag = match known.get(i).and_then(|k| k.as_ref()) {
            Some(hit) => match &hit.desc {
                Some(d) => format!(" ←已知元素「{}」（{}），name 请复用它", hit.name, d),
                None => format!(" ←已知元素「{}」，name 请复用它", hit.name),
            },
            None => String::new(),
        };
        out.push_str(&format!("[{}] {} @({},{}){}\n", i, el.to_ai_text(), c.x, c.y, tag));
    }
    if elements.len() > MAX_ELEMENTS {
        out.push_str(&format!("... 还有 {} 个元素未列出\n", elements.len() - MAX_ELEMENTS));
    }
    out
}
