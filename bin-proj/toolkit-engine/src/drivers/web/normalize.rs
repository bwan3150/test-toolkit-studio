// DOM 归一化 - 把页面可见元素转成 uiautomator 风格扁平 XML
// 让网页元素与 App 元素进入同一套解析/识别/标注体系：
//   resource-id=DOM id, content-desc=aria-label, text=直接文本, class=标签名,
//   bounds=截图像素坐标（CSS 坐标 × devicePixelRatio）。
// DOM_WALK_JS 在浏览器内执行提取元素，dom_elements_to_xml 在本地拼装 XML。

/// 注入浏览器执行的元素提取脚本：遍历可见元素，返回 {tag,id,aria,text,clickable,x1..y2} 列表
pub(super) const DOM_WALK_JS: &str = r#"
const dpr = window.devicePixelRatio || 1;
const out = [];
// 给元素算一条**唯一、与分辨率无关**的 DOM 路径，用于回放时精确重定位同一个元素
// （纯 text 会撞名：页面常有两个 "Products"、导航项与页脚同名链接，回放只认 text 会点错）。
// 锚在最近带 id 的祖先上 + 其余用 tag[第几个同类]，DOM 结构与屏幕尺寸无关，跨机型稳定。
const xpathOf = (el) => {
  const segs = [];
  let node = el;
  while (node && node.nodeType === 1) {
    if (node.id) { segs.unshift("*[@id='" + node.id + "']"); break; }
    if (node === document.body) { segs.unshift('body'); break; }
    let i = 1, sib = node;
    while ((sib = sib.previousElementSibling)) { if (sib.tagName === node.tagName) i++; }
    segs.unshift(node.tagName.toLowerCase() + '[' + i + ']');
    node = node.parentElement;
  }
  return '/' + segs.join('/');
};
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
      // 只收"有意义"的元素：可点击 / 有自身文字 / aria 标注 / 输入框；
      // 纯包裹层(无文字、不可点的 div/svg/use 等)不入列表，避免噪音淹没 AI。
      const aria = child.getAttribute('aria-label') || '';
      const meaningful = clickable || ownText || aria ||
        child.tagName === 'INPUT' || child.tagName === 'TEXTAREA';
      if (meaningful) {
        out.push({
          tag: child.tagName.toLowerCase(),
          id: child.id || '',
          aria: aria,
          text: ownText,
          xpath: xpathOf(child),
          clickable: clickable,
          x1: Math.round(r.left * dpr), y1: Math.round(r.top * dpr),
          x2: Math.round(r.right * dpr), y2: Math.round(r.bottom * dpr),
        });
      }
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
            "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" xpath=\"{}\" clickable=\"{}\" enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
            escape_attr(e["tag"].as_str().unwrap_or("")),
            escape_attr(e["id"].as_str().unwrap_or("")),
            escape_attr(e["aria"].as_str().unwrap_or("")),
            escape_attr(e["text"].as_str().unwrap_or("")),
            escape_attr(e["xpath"].as_str().unwrap_or("")),
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

