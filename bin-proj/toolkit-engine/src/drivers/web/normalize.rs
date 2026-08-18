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
// SVG 的内部构件：图标画出来靠它们，但**没有一个是该单独点的目标**。
// 它们会因为 cursor:pointer 从父按钮继承而全部变成 "clickable"，于是一个图标按钮
// 能刷出 svg+path+rect+g 四五条记录 —— 实测某后台一屏 43 个元素里 30 个是这种噪音。
const GRAPHIC = ['SVG','PATH','RECT','CIRCLE','ELLIPSE','LINE','POLYGON','POLYLINE',
                 'G','USE','DEFS','MASK','CLIPPATH','TSPAN','STOP','LINEARGRADIENT'];

// ox/oy = 这层文档相对**顶层页面**的像素偏移(iframe 内部文档的 rect 是相对它自己的视口的);
// vw/vh = 这层文档的视口大小(iframe 里判"在不在视口内"要用 iframe 的尺寸,不是主窗口的)
// fp = frame 前缀:iframe 内元素的 xpath 是相对**内部文档**的,拿到主文档去找必然落空。
// 前缀成 `iframe[2]>>/*[@id='x']`,一眼看得出"它在第 2 个 iframe 里",也不会被误用
const walk = (el, clickableAncestor, ox, oy, vw, vh, fp) => {
  for (const child of el.children) {
    // 声明在 if 外：下面递归时要把它传给子元素（块作用域里的 const 出不来）
    let selfClickable = false;
    const r = child.getBoundingClientRect();
    const style = getComputedStyle(child);
    // 读屏专用元素（sr-only / screen-reader-text）必须排除：它们**人看不见但带着那行文字**，
    // 典型实现是 1×1 像素 + clip 裁掉。不排除的话文字定位会优先命中这个 1×1 幽灵点，
    // 点下去自然什么也没发生——实测撞过：某百科首页的 `<label>Search Wikipedia</label>`
    // 就是 1×1，于是 `输入 ["Search Wikipedia", …]` 点空，报的还是「没有聚焦的输入框」，
    // 极难查。人点不到的东西，就不该出现在给 AI 的元素表里。
    const srOnly = r.width <= 1 || r.height <= 1 || style.opacity === '0' ||
      (style.clipPath && style.clipPath !== 'none' && style.clipPath.indexOf('inset(50%') === 0) ||
      (style.clip && style.clip !== 'auto' && /^rect\((0px,\s*){3}0px\)$/.test(style.clip));
    const visible = r.width > 0 && r.height > 0 && !srOnly &&
      style.visibility !== 'hidden' && style.display !== 'none' &&
      r.bottom > 0 && r.top < vh && r.right > 0 && r.left < vw;
    if (visible) {
      // 仅取直接文本（不含子元素文本），避免父容器吞掉所有文字
      let ownText = '';
      for (const n of child.childNodes) {
        if (n.nodeType === 3) ownText += n.textContent;
      }
      ownText = ownText.trim().slice(0, 120);
      // 输入框取 placeholder/value 兜底。
      // ⚠️ 密码框**永远不取 value**：采集结果会落进 page/*.xml、进报告、进 AI 上下文，
      // 而这些是要发给别人看的证据。密码框里有什么，谁也不需要知道（只需要知道"填了"）。
      const isPassword = child.tagName === 'INPUT' && child.type === 'password';
      if (!ownText && (child.tagName === 'INPUT' || child.tagName === 'TEXTAREA')) {
        ownText = isPassword
          ? (child.value ? '••••••' : (child.placeholder || ''))
          : (child.value || child.placeholder || '').slice(0, 120);
      }
      // 可及名称兜底：输入框/图标按钮常常**一个字都没有**——没有直接文本、没有 placeholder，
      // 可见的那行字其实来自 <label for>（实测撞过：某百科首页搜索框只有 label，text 全空，
      // 于是文字定位必失败、调用方只能回落坐标，语义定位这条路等于断了）。
      // 顺序照可及性标准：aria-label(已单列) > aria-labelledby > label > title。
      if (!ownText) {
        let acc = '';
        const by = child.getAttribute('aria-labelledby');
        if (by) {
          acc = by.split(/\s+/)
            .map(id => { const n = document.getElementById(id); return n ? (n.innerText || n.textContent || '') : ''; })
            .join(' ').trim();
        }
        // .labels 是原生属性，<label for=id> 与 <label> 包裹两种写法都能拿到
        if (!acc && child.labels && child.labels.length) {
          acc = (child.labels[0].innerText || child.labels[0].textContent || '').trim();
        }
        if (!acc) acc = (child.getAttribute('title') || '').trim();
        ownText = acc.slice(0, 120);
      }
      // <select> 特判：文字全在子 <option> 里，只取直接文本节点会得到空；
      // 而闭合状态下 option 的 getBoundingClientRect 是 0，走不到这一层就被可见性
      // 过滤掉了 —— 结果是 AI 既看不到当前值、也不知道有哪些选项可选（实测撞过）。
      // 所以这里把"当前值"当文本、把"全部选项"单列一个字段带出去。
      let optionList = null;
      if (child.tagName === 'SELECT') {
        const opts = Array.from(child.options || []);
        optionList = opts.map(o => (o.text || '').trim()).filter(Boolean).slice(0, 50);
        const cur = child.selectedOptions && child.selectedOptions[0];
        ownText = ((cur && cur.text) || '').trim().slice(0, 120);
      }
      let aria = child.getAttribute('aria-label') || '';
      const tag = child.tagName.toUpperCase();
      // 跨域 iframe:里面一个元素都采不到。**必须留一条标记**(INV-9)——
      // 不然它跟"这块本来就是空的"长得一模一样,AI 只会往被测对象身上找原因。
      // 标记拼进这个 iframe 自己那条记录里,不另外 push,否则同一个 iframe 会出现两次
      let crossOrigin = false;
      if (tag === 'IFRAME' || tag === 'FRAME') {
        let d = null;
        try { d = child.contentDocument; } catch (e) { d = null; }
        if (!d || !d.body) {
          crossOrigin = true;
          aria = (aria || child.getAttribute('title') || child.getAttribute('name') || '')
                 + ' [跨域内容，采不到内部元素]';
        }
      }
      const isGraphic = GRAPHIC.includes(tag);
      const isFormControl = ['INPUT','TEXTAREA','SELECT'].includes(tag);
      // 这个元素**自己**是可点的吗。`cursor: pointer` 会被子元素继承，所以只有在
      // **没有可点祖先**时才认它——否则按钮里的每个 span/svg/path 都会各算一条，
      // 而它们点起来效果完全一样（实测：一个图标按钮刷出 4 条记录）
      selfClickable = ['A','BUTTON'].includes(tag) || isFormControl ||
        child.onclick != null || child.getAttribute('role') === 'button' ||
        (style.cursor === 'pointer' && !clickableAncestor);
      // 只收**人会去点、或人看得见的文字**：有自身文字 / aria 标注 / 表单控件 / 自身可点。
      // 图形构件一律排除（除非它自己带了 aria/title —— 那种是真图标按钮）。
      const meaningful = crossOrigin ? true : (isGraphic
        ? !!(aria || child.getAttribute('title'))
        : (selfClickable || ownText || aria || isFormControl));
      if (meaningful) {
        out.push({
          tag: child.tagName.toLowerCase(),
          id: child.id || '',
          aria: aria,
          text: ownText,
          xpath: fp + xpathOf(child),
          clickable: selfClickable,
          password: isPassword,
          options: optionList,
          x1: Math.round((r.left + ox) * dpr), y1: Math.round((r.top + oy) * dpr),
          x2: Math.round((r.right + ox) * dpr), y2: Math.round((r.bottom + oy) * dpr),
        });
      }
    }
    // —— iframe：跨进这层文档，否则里面的东西一个都采不到 ——
    // 支付、第三方登录、富文本编辑器、验证码都常住在 iframe 里。不进去的话
    // AI 拿到的是一张「什么都没有」的页面,然后得出「功能没渲染出来」的假结论。
    if (visible && (child.tagName === 'IFRAME' || child.tagName === 'FRAME')) {
      let doc = null;
      try { doc = child.contentDocument; } catch (e) { doc = null; }   // 跨域会抛
      if (doc && doc.body) {
        // 内部 rect 相对 iframe 自己的视口 → 累加 iframe 在外层的位置。
        // 还要加边框宽度:内容区从边框内侧开始
        const bl = parseFloat(style.borderLeftWidth) || 0;
        const bt = parseFloat(style.borderTopWidth) || 0;
        walk(doc.body, false, ox + r.left + bl, oy + r.top + bt,
             child.clientWidth || r.width, child.clientHeight || r.height,
             fp + xpathOf(child) + '>>');
      }
    }
    // 把"祖先里已经有可点的了"传下去，子元素就不会因为继承 cursor:pointer 而重复上榜
    walk(child, clickableAncestor || selfClickable, ox, oy, vw, vh, fp);
  }
};
walk(document.body, false, 0, 0, innerWidth, innerHeight, '');
return out;
"#;

use crate::utils::xml::escape_attr;

/// 把 DOM_WALK_JS 返回的元素列表归一化为 uiautomator 风格 XML
pub(super) fn dom_elements_to_xml(elements: &serde_json::Value) -> String {
    let empty = vec![];
    let list = elements.as_array().unwrap_or(&empty);

    let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
    for e in list {
        // <select> 的可选项：闭合状态下 option 自身不可见、采不到，只能挂在 select 上带出来。
        // 没有它 AI 就不知道能选什么（用户实测撞到过：只好绕道 python 读页面）
        let options = e["options"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .unwrap_or_default();
        let options_attr = if options.is_empty() {
            String::new()
        } else {
            format!(" options=\"{}\"", escape_attr(&options))
        };
        // 与安卓 uiautomator 同名属性对齐（那边原生就有 password="true"），
        // 于是上层判断"这是不是密码框"三个平台走同一条路
        let password_attr = if e["password"].as_bool().unwrap_or(false) {
            " password=\"true\""
        } else {
            ""
        };
        xml.push_str(&format!(
            "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" xpath=\"{}\" clickable=\"{}\"{}{} enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
            escape_attr(e["tag"].as_str().unwrap_or("")),
            escape_attr(e["id"].as_str().unwrap_or("")),
            escape_attr(e["aria"].as_str().unwrap_or("")),
            escape_attr(e["text"].as_str().unwrap_or("")),
            escape_attr(e["xpath"].as_str().unwrap_or("")),
            e["clickable"].as_bool().unwrap_or(false),
            options_attr,
            password_attr,
            e["x1"].as_i64().unwrap_or(0),
            e["y1"].as_i64().unwrap_or(0),
            e["x2"].as_i64().unwrap_or(0),
            e["y2"].as_i64().unwrap_or(0),
        ));
    }
    xml.push_str("</hierarchy>\n");
    xml
}

