// 【极简 Markdown】把 AI 写的总结渲染进报告。
//
// 为什么**不**在 HTML 里塞个 JS 渲染器：报告是**纯静态自包含文件**——离线能看、
// 内网能看、CI 里能看、转成 PDF 也能看、邮件客户端里也能看。塞进 JS 就意味着
// "得有个肯执行脚本的浏览器"，那是白白给交付物加前提。在 Rust 侧编译期转完，
// 产物仍然只是一段 HTML。
//
// 为什么**不**引 pulldown-cmark：AI 写总结用到的就那几样——段落、列表、加粗、行内代码。
// 为这点东西多一个依赖不划算（同 tke fix 用 curl 而不是 reqwest 的理由）。
//
// ⚠️ 安全：**先整体转义，再认标记**。summary 是 AI 生成的文本，直接拼进 HTML 就是
// 注入口子。转义在前，后面只把我们自己生成的标签放回去，AI 写什么都注入不进来。

/// 支持的子集：
/// - 段落（空行分隔）
/// - `- ` / `* ` 无序列表，`1. ` 有序列表
/// - **表格**（GFM 风格 `| a | b |`）—— AI 做对照结论最爱用这个，不支持等于逼它压成一行
/// - `**粗**`、`` `代码` ``
/// - `#` 标题（当小标题渲染）
pub fn to_html(src: &str) -> String {
    let mut out = String::new();
    let mut list: Option<&'static str> = None; // 当前列表类型 ul/ol

    let close_list = |out: &mut String, list: &mut Option<&'static str>| {
        if let Some(tag) = list.take() {
            out.push_str(&format!("</{}>", tag));
        }
    };

    for block in src.split("\n\n") {
        let block = block.trim_matches('\n');
        if block.trim().is_empty() {
            continue;
        }
        // 表格要整块看（一行行处理认不出来），先把它挑出来
        let lines: Vec<&str> = block.lines().collect();
        let mut i = 0usize;
        while i < lines.len() {
            let t = lines[i].trim();
            if t.is_empty() {
                i += 1;
                continue;
            }
            // 表格：`| a | b |` 起头，第二行是 `|---|---|` 分隔
            if is_table_row(t) && i + 1 < lines.len() && is_table_divider(lines[i + 1].trim()) {
                close_list(&mut out, &mut list);
                let mut rows = vec![t];
                let mut j = i + 2;
                while j < lines.len() && is_table_row(lines[j].trim()) {
                    rows.push(lines[j].trim());
                    j += 1;
                }
                out.push_str(&table_html(&rows));
                i = j;
                continue;
            }
            i += 1;
            // `#`/`##`/`###` 小标题
            if let Some(rest) = t.strip_prefix('#') {
                let lvl = 1 + rest.chars().take_while(|c| *c == '#').count();
                let text = rest.trim_start_matches('#').trim();
                if !text.is_empty() {
                    close_list(&mut out, &mut list);
                    // 都渲染成 h4/h5：报告里 h1 已经是标题了，结论里不该有更大的字
                    let tag = if lvl <= 2 { "h4" } else { "h5" };
                    out.push_str(&format!("<{t}>{}</{t}>", inline(text), t = tag));
                    continue;
                }
            }
            let (item, kind) = if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                (Some(rest), "ul")
            } else if let Some(rest) = ordered_item(t) {
                (Some(rest), "ol")
            } else {
                (None, "")
            };

            match item {
                Some(text) => {
                    if list != Some(kind) {
                        close_list(&mut out, &mut list);
                        out.push_str(&format!("<{}>", kind));
                        list = Some(kind);
                    }
                    out.push_str(&format!("<li>{}</li>", inline(text)));
                }
                None => {
                    close_list(&mut out, &mut list);
                    out.push_str(&format!("<p>{}</p>", inline(t)));
                }
            }
        }
        close_list(&mut out, &mut list);
    }
    close_list(&mut out, &mut list);
    out
}

/// `| a | b |` 这样的表格行
fn is_table_row(t: &str) -> bool {
    t.starts_with('|') && t.len() > 1
}

/// `|---|:---:|` 这样的分隔行（GFM 用它区分表头与表体）
fn is_table_divider(t: &str) -> bool {
    is_table_row(t)
        && t.trim_matches('|')
            .split('|')
            .all(|c| {
                let c = c.trim();
                !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')
            })
}

/// 切一行的单元格：去掉首尾竖线再按 `|` 分
fn cells(row: &str) -> Vec<String> {
    row.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| inline(c.trim()))
        .collect()
}

/// 首行当表头，其余当表体
fn table_html(rows: &[&str]) -> String {
    let mut s = String::from(r#"<div class="t-tw"><table>"#);
    if let Some(head) = rows.first() {
        s.push_str("<thead><tr>");
        for c in cells(head) {
            s.push_str(&format!("<th>{}</th>", c));
        }
        s.push_str("</tr></thead>");
    }
    if rows.len() > 1 {
        s.push_str("<tbody>");
        for r in &rows[1..] {
            s.push_str("<tr>");
            for c in cells(r) {
                s.push_str(&format!("<td>{}</td>", c));
            }
            s.push_str("</tr>");
        }
        s.push_str("</tbody>");
    }
    s.push_str("</table></div>");
    s
}

/// `1. xxx` / `2) xxx` → `xxx`
fn ordered_item(t: &str) -> Option<&str> {
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &t[digits.len()..];
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))
}

/// 行内标记。**先转义**（见文件头的安全说明），再把 `**粗**` / `` `码` `` 换成标签
fn inline(s: &str) -> String {
    let mut h = esc(s);
    h = pair(&h, "**", "<strong>", "</strong>");
    h = pair(&h, "`", "<code>", "</code>");
    h
}

/// 成对标记替换：奇数个（没配上对）就原样留着，不吞字符
fn pair(s: &str, mark: &str, open: &str, close: &str) -> String {
    let parts: Vec<&str> = s.split(mark).collect();
    if parts.len() < 3 {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            out.push_str(p);
        } else if i % 2 == 1 {
            // 最后一段是奇数位 = 这个标记没有闭合，原样还回去
            if i == parts.len() - 1 {
                out.push_str(mark);
                out.push_str(p);
            } else {
                out.push_str(open);
                out.push_str(p);
                out.push_str(close);
            }
        } else {
            out.push_str(p);
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_subset_ai_actually_uses() {
        assert_eq!(to_html("一句话结论"), "<p>一句话结论</p>");
        assert_eq!(
            to_html("- 第一条\n- 第二条"),
            "<ul><li>第一条</li><li>第二条</li></ul>"
        );
        assert_eq!(to_html("1. 甲\n2. 乙"), "<ol><li>甲</li><li>乙</li></ol>");
        assert_eq!(to_html("**重点**在这"), "<p><strong>重点</strong>在这</p>");
        assert_eq!(to_html("跑 `tke doctor`"), "<p>跑 <code>tke doctor</code></p>");
        // 段落之间
        assert_eq!(to_html("前言\n\n- a\n\n收尾"), "<p>前言</p><ul><li>a</li></ul><p>收尾</p>");
    }

    /// AI 做对照结论最爱用表格——不支持的话它只能把整张表压成一行流水账
    /// （实测就这么发生的：对话框里是张漂亮的对照表，报告里成了一句话）
    #[test]
    fn renders_tables() {
        let md = "| 参数 | Sydney | Melbourne |\n|---|---|---|\n| CPU | 1.09 | 1.01 |\n| 温度 | 41.0°C | — |";
        let h = to_html(md);
        assert!(h.contains("<table>"), "要出表格:{}", h);
        assert!(h.contains("<th>参数</th>"), "首行是表头:{}", h);
        assert!(h.contains("<td>41.0°C</td>"), "表体要有数据:{}", h);
        assert_eq!(h.matches("<tr>").count(), 3, "1 表头 + 2 数据行");
        // 表格里的行内标记照常生效
        assert!(to_html("| a |\n|---|\n| **粗** |").contains("<strong>粗</strong>"));
    }

    #[test]
    fn renders_headings() {
        assert!(to_html("## 性能参数对比").contains("<h4>性能参数对比</h4>"));
        assert!(to_html("### 细节").contains("<h5>细节</h5>"));
        // 光一个 # 不算标题
        assert_eq!(to_html("#"), "<p>#</p>");
    }

    /// **AI 写的文本直接进 HTML 就是注入口子**：必须先转义再认标记
    #[test]
    fn escapes_before_parsing() {
        let h = to_html("<script>alert(1)</script>");
        assert!(!h.contains("<script>"), "标签必须被转义:{}", h);
        assert!(h.contains("&lt;script&gt;"));
        // 标记里夹标签同样要转义
        let h = to_html("**<img src=x onerror=1>**");
        assert!(h.contains("<strong>"), "加粗仍要生效:{}", h);
        assert!(!h.contains("<img"), "内容里的标签必须转义:{}", h);
    }

    /// 没配对的标记不能吞字符——AI 写个孤零零的星号是常有的事
    #[test]
    fn unpaired_marks_are_left_alone() {
        assert_eq!(to_html("2 ** 3 = 8"), "<p>2 ** 3 = 8</p>");
        assert_eq!(to_html("反引号 ` 单个"), "<p>反引号 ` 单个</p>");
    }
}
