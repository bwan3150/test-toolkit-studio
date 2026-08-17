// 【脱敏】命令原文落进证据前把敏感值打码。
//
// 为什么必须在**代码里**做，而不是嘱咐 AI「别把密码写进命令」：
// `--log` 会把每一步的命令写进 log.json、渲染进 report.html，**还会烧进标注截图的顶部横幅**。
// 实测过一次：代填的密码就那样明晃晃印在图上——而报告正是拿去分享的东西
// （这个会话里就把报告传到过公网 URL）。靠提示词当护栏，这个项目已经失败过两次。
//
// 判据不在这里：是执行时按**真实页面**认出密码框（`UIElement::is_password`，
// 安卓 uiautomator 原生属性 / web 的 `type=password`），不是猜命令里有没有"密码"二字。
// 本模块只负责"已经确定这步敏感"之后，把值换掉。

/// 打码后的占位符。用圆点而不是 `***`：与浏览器密码框的显示一致，一眼看得出是"有值但不给看"
const MASK: &str = "••••••";

/// 把命令里的**值**换成占位符，保留结构与目标，这样报告仍读得懂"这步在干什么"。
///
/// ```text
/// 输入 ["密码", "hunter2"]        → 输入 ["密码", "••••••"]
/// 输入 [{640, 380}, "hunter2"]    → 输入 [{640, 380}, "••••••"]
/// 输入 ["密码", "hunter2"] # 登录 → 输入 ["密码", "••••••"] # 登录
/// ```
///
/// 做法：只替换**最后一个**带引号的字符串——`输入` 的值永远是最后一个参数，
/// 而目标（第一个参数）要留着，否则报告里就成了 `输入 ["••••", "••••"]`，
/// 人根本看不出这步点的是哪个框。行内注释（`#` 之后）原样保留：那是 AI 写的意图，不是值。
pub fn mask_command_values(raw: &str) -> String {
    // 先把行内注释切出去——注释里出现引号不该干扰定位（`# 用"测试账号"登录`）
    let (cmd, comment) = split_comment(raw);

    let quotes: Vec<usize> = cmd.char_indices().filter(|(_, c)| *c == '"').map(|(i, _)| i).collect();
    // ⚠️ **拿不准就多打码**：这里的失败方向是不对称的——少打一次码就是把密码写进了
    // 要分享出去的报告，而多打一次只是让人少看到一个参数。所以任何不符合预期的结构
    // 一律整条打掉，绝不"原样退回"（原样退回 = 明文漏出去）。
    if quotes.len() < 2 || quotes.len() % 2 != 0 {
        return mask_whole(cmd, comment);
    }
    let open = quotes[quotes.len() - 2];
    let close = quotes[quotes.len() - 1];
    // 最后一对引号之后只该剩 `]` 和空白。不是的话说明引号并非成对包裹参数
    // （例如值没闭合：`输入 ["密码, "hunter2]`），按上面的原则整条打掉——
    // 这种输入曾让"只替换最后一对"把明文留在了外面。
    if cmd[close + 1..].chars().any(|c| c != ']' && !c.is_whitespace()) {
        return mask_whole(cmd, comment);
    }
    let mut out = String::with_capacity(cmd.len() + MASK.len());
    out.push_str(&cmd[..open + 1]);
    out.push_str(MASK);
    out.push_str(&cmd[close..]);
    out.push_str(comment);
    out
}

/// 兜底：只留命令名，参数整体打掉。报告里仍看得出"这步在做什么"，但一个字符都不漏
fn mask_whole(cmd: &str, comment: &str) -> String {
    let name = cmd
        .split(|c: char| c == '[' || c.is_whitespace())
        .find(|s| !s.is_empty())
        .unwrap_or("");
    format!("{} [{}]{}", name, MASK, comment)
}

/// 按**引号外**的 `#` 切出行内注释（与解析器同一口径：引号内的 `#` 不是注释）
fn split_comment(raw: &str) -> (&str, &str) {
    let mut in_quote = false;
    for (i, c) in raw.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '#' if !in_quote => return (&raw[..i], &raw[i..]),
            _ => {}
        }
    }
    (raw, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_only_the_value_keeps_the_target() {
        // 目标要留着，否则报告里看不出这步点的是哪个框
        assert_eq!(
            mask_command_values(r#"输入 ["密码", "hunter2"]"#),
            r#"输入 ["密码", "••••••"]"#
        );
        assert_eq!(
            mask_command_values(r#"输入 [{640, 380}, "hunter2"]"#),
            r#"输入 [{640, 380}, "••••••"]"#
        );
    }

    #[test]
    fn keeps_inline_comment() {
        // `#` 之后是 AI 写的意图，报告要显示它——那不是值
        assert_eq!(
            mask_command_values(r#"输入 ["密码", "hunter2"] # 用测试账号登录"#),
            r#"输入 ["密码", "••••••"] # 用测试账号登录"#
        );
        // 引号内的 # 不算注释（与解析器同口径）
        assert_eq!(
            mask_command_values(r#"输入 ["标签#1", "hunter2"]"#),
            r#"输入 ["标签#1", "••••••"]"#
        );
    }

    /// 畸形输入必须**整条打掉**，不能原样退回。
    /// 这条是从一次真实的失败里来的：`输入 ["密码, "hunter2]`（值没闭合）引号数是偶数，
    /// "只替换最后一对"会把明文 `hunter2` 留在外面——少打一次码就是泄一次密码。
    #[test]
    fn malformed_input_is_fully_masked() {
        for odd in [
            r#"输入 ["密码, "hunter2]"#,
            r#"输入 ["密码", "hunter2]"#,
            r#"输入 [密码, hunter2]"#,
        ] {
            let out = mask_command_values(odd);
            assert!(!out.contains("hunter2"), "明文漏出去了：{} → {}", odd, out);
            assert!(out.starts_with("输入"), "命令名要留着：{}", out);
        }
    }

    #[test]
    fn mask_does_not_leak_length() {
        // 占位符长度固定——泄露密码长度也是泄露
        let short = mask_command_values(r#"输入 ["密码", "a"]"#);
        let long = mask_command_values(r#"输入 ["密码", "correct-horse-battery-staple"]"#);
        assert_eq!(short, r#"输入 ["密码", "••••••"]"#);
        assert_eq!(long, r#"输入 ["密码", "••••••"]"#);
    }
}
