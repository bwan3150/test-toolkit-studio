// 终端排版：按**显示宽度**算，不按字符数。
//
// `format!("{:<10}", "iOS模拟器")` 是按 char 数填充的，而中文在等宽终端里占**两格**——
// 混着排必然错位。tke 的表格里中英混排到处都是，所以这两个函数是排版的地基。
// （TUI 那边用的是 ratatui 的 Span::width；CLI 不该为了这点事把 ratatui 拖进来。）

/// 显示宽度：CJK / 全角标点算 2，其余算 1。
///
/// 不引 `unicode-width`：那个 crate 处理的是完整的东亚宽度表 + 组合字符，
/// 而这里排的是设备名、机型、状态这类东西——覆盖 CJK 与全角标点就够了。
pub fn disp_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let u = c as u32;
    // 覆盖：CJK 统一表意、扩展 A、兼容表意、假名、谚文、全角/半角形式、
    // CJK 标点（，。「」）、以及常见符号（·—✓✗ 之类仍按 1，它们在等宽终端里是窄的）
    let wide = (0x1100..=0x115F).contains(&u)      // 谚文字母
        || (0x2E80..=0x303E).contains(&u)          // CJK 部首 + 标点（含，。）
        || (0x3041..=0x33FF).contains(&u)          // 假名、注音、兼容
        || (0x3400..=0x4DBF).contains(&u)          // 扩展 A
        || (0x4E00..=0x9FFF).contains(&u)          // 统一表意
        || (0xA000..=0xA4CF).contains(&u)          // 彝文
        || (0xAC00..=0xD7A3).contains(&u)          // 谚文音节
        || (0xF900..=0xFAFF).contains(&u)          // 兼容表意
        || (0xFE30..=0xFE6F).contains(&u)          // 竖排/小写变体
        || (0xFF00..=0xFF60).contains(&u)          // 全角 ASCII（含（）：）
        || (0xFFE0..=0xFFE6).contains(&u)          // 全角符号
        || (0x1F300..=0x1FAFF).contains(&u); // emoji
    if wide { 2 } else { 1 }
}

/// 左对齐补到指定显示宽度（超宽就原样返回，不截断——截断设备 ID 比错位更糟）
pub fn pad_right(s: &str, width: usize) -> String {
    let w = disp_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_counts_as_two_columns() {
        assert_eq!(disp_width("abc"), 3);
        assert_eq!(disp_width("iOS模拟器"), 3 + 6, "三个汉字 = 6 格");
        assert_eq!(disp_width("（无头）"), 8, "全角括号也是 2 格");
    }

    /// 这两行的第二列必须从同一个位置开始——**表格对齐的全部意义**
    #[test]
    fn pads_by_display_width_not_char_count() {
        let a = pad_right("安卓", 10);
        let b = pad_right("iOS模拟器", 10);
        assert_eq!(disp_width(&a), 10);
        assert_eq!(disp_width(&b), 10);
        // 按字符数补的话这俩会差 3 格（"安卓"2 字 vs "iOS模拟器"6 字）
        assert_ne!(a.chars().count(), b.chars().count(), "字符数本来就不同");
    }

    #[test]
    fn does_not_truncate_when_too_wide() {
        assert_eq!(pad_right("sim:很长的设备名", 4), "sim:很长的设备名");
    }
}
