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
/// - `**粗**`、`` `代码` ``
/// - 单换行 → `<br>`
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
        for line in block.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
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
