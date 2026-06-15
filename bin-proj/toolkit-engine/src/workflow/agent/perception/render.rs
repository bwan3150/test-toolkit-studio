// 把页面元素列表渲染成喂给 AI 的文本（带序号 = element_id）

use crate::UIElement;

/// 页面元素列表最多展示给 AI 的条数（防 token 爆炸）
const MAX_ELEMENTS: usize = 120;

/// 渲染元素列表：`[序号] 控件描述 @(中心坐标)`
pub fn render_element_list(elements: &[UIElement]) -> String {
    if elements.is_empty() {
        return "(当前页面未解析到任何元素)".to_string();
    }
    let mut out = String::new();
    for (i, el) in elements.iter().enumerate().take(MAX_ELEMENTS) {
        let c = el.center();
        out.push_str(&format!("[{}] {} @({},{})\n", i, el.to_ai_text(), c.x, c.y));
    }
    if elements.len() > MAX_ELEMENTS {
        out.push_str(&format!("... 还有 {} 个元素未列出\n", elements.len() - MAX_ELEMENTS));
    }
    out
}
