// DOM 归一化 - 把页面可见元素转成 uiautomator 风格扁平 XML
// 让网页元素与 App 元素进入同一套解析/识别/标注体系：
//   resource-id=DOM id, content-desc=aria-label, text=直接文本, class=标签名,
//   bounds=截图像素坐标（CSS 坐标 × devicePixelRatio）。
// DOM_WALK_JS 在浏览器内执行提取元素，dom_elements_to_xml 在本地拼装 XML。

/// 注入浏览器执行的元素提取脚本：遍历可见元素，返回 {tag,id,aria,text,clickable,x1..y2} 列表
pub(super) const DOM_WALK_JS: &str = r#"
const dpr = window.devicePixelRatio || 1;
const out = [];
const walk = (el) => {
  for (const child of el.children) {
    const r = child.getBoundingClientRect();
    const style = getComputedStyle(child);
    const visible = r.width > 0 && r.height > 0 &&
      style.visibility !== 'hidden' && style.display !== 'none' &&
      r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth;
    if (visible) {
      // 仅取直接文本（不含子元素文本），避免父容器吞掉所有文字
      let ownText = '';
      for (const n of child.childNodes) {
        if (n.nodeType === 3) ownText += n.textContent;
      }
      ownText = ownText.trim().slice(0, 120);
      // 输入框取 placeholder/value 兜底
      if (!ownText && (child.tagName === 'INPUT' || child.tagName === 'TEXTAREA')) {
        ownText = (child.value || child.placeholder || '').slice(0, 120);
      }
      const clickable = ['A','BUTTON','SELECT'].includes(child.tagName) ||
        ['INPUT','TEXTAREA'].includes(child.tagName) ||
        child.onclick != null || style.cursor === 'pointer' ||
        child.getAttribute('role') === 'button';
      out.push({
        tag: child.tagName.toLowerCase(),
        id: child.id || '',
        aria: child.getAttribute('aria-label') || '',
        text: ownText,
        clickable: clickable,
        x1: Math.round(r.left * dpr), y1: Math.round(r.top * dpr),
        x2: Math.round(r.right * dpr), y2: Math.round(r.bottom * dpr),
      });
    }
    walk(child);
  }
};
walk(document.body);
return out;
"#;

use crate::utils::xml::escape_attr;

/// 把 DOM_WALK_JS 返回的元素列表归一化为 uiautomator 风格 XML
pub(super) fn dom_elements_to_xml(elements: &serde_json::Value) -> String {
    let empty = vec![];
    let list = elements.as_array().unwrap_or(&empty);

    let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
    for e in list {
        xml.push_str(&format!(
            "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
            escape_attr(e["tag"].as_str().unwrap_or("")),
            escape_attr(e["id"].as_str().unwrap_or("")),
            escape_attr(e["aria"].as_str().unwrap_or("")),
            escape_attr(e["text"].as_str().unwrap_or("")),
            e["clickable"].as_bool().unwrap_or(false),
            e["x1"].as_i64().unwrap_or(0),
            e["y1"].as_i64().unwrap_or(0),
            e["x2"].as_i64().unwrap_or(0),
            e["y2"].as_i64().unwrap_or(0),
        ));
    }
    xml.push_str("</hierarchy>\n");
    xml
}

