// 文本查找模块 - 根据文本内容查找元素（脚本中的纯文本参数，如 `点击 ["Sign in"]`）

use crate::{Result, TkeError, Point, Bounds, Fetcher, UIElement};
use std::path::Path;

/// 根据文本查找元素，返回 (中心点, 实时边界框)
pub fn find_by_text(ui_tree_path: &Path, text: &str) -> Result<(Point, Bounds)> {
    let xml_content = std::fs::read_to_string(ui_tree_path)
        .map_err(TkeError::IoError)?;

    // 提取所有UI元素
    let fetcher = Fetcher::new();
    let elements = fetcher.fetch_elements_from_xml(&xml_content)?;

    let element = pick(&elements, text)
        .ok_or_else(|| TkeError::ElementNotFound(format!("未找到包含文本 '{}' 的元素", text)))?;

    Ok((element.center(), element.bounds.clone()))
}

/// 同一段文字在页面上出现多次时，挑哪一个。
///
/// **这里曾经是 `.find()`——取 DOM 序第一个，于是踩了一次真事故**：
/// 登录页标题是 `<h1>Sign In`，下面才是 `<button>Sign in`。`点击 ["Sign in"]`
/// 每次都点在标题上——**点得中、报成功、什么也没发生**。AI 连点三轮不同的账号组合，
/// 得出「这个表单点了完全没反应，是死表单」的结论，还写进了报告。
/// 花了一个多小时，全程没人怀疑过是点错了元素（P-35）。
///
/// 排序键（依次比较，全都是「更像用户会点的那个」）：
/// 1. **可点击的优先** —— 决定性的一条。点标题永远没用
/// 2. **精确匹配优先** —— `Sign in` 该选按钮本身，不是 `Sign in with Google`
/// 3. **文字短的优先** —— 包着按钮的那层容器文字通常更长
/// 4. DOM 序 —— 前三项都平手时保持稳定，不要今天点这个明天点那个
///
/// 没有任何可点击候选时**照旧返回第一个匹配**：断言文本存在之类的用法不需要可点击。
pub fn pick<'a>(elements: &'a [UIElement], text: &str) -> Option<&'a UIElement> {
    elements
        .iter()
        .filter(|e| e.matches_text(text))
        .min_by_key(|e| rank(e, text))
}

fn rank(e: &UIElement, text: &str) -> (u8, u8, usize, usize) {
    let own_len = [&e.text, &e.content_desc, &e.hint]
        .into_iter()
        .flatten()
        .map(|s| s.chars().count())
        .min()
        .unwrap_or(usize::MAX);
    (
        u8::from(!e.clickable),
        u8::from(!exact(e, text)),
        own_len,
        e.index,
    )
}

/// 元素自身的文字**就是**要找的那串（忽略大小写和首尾空白），而不是包含它
fn exact(e: &UIElement, text: &str) -> bool {
    let want = text.trim().to_lowercase();
    [&e.text, &e.content_desc, &e.hint]
        .into_iter()
        .flatten()
        .any(|s| s.trim().to_lowercase() == want)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Bounds;

    fn el(index: usize, class: &str, text: &str, clickable: bool) -> UIElement {
        UIElement {
            child_text: None,
            index,
            class_name: class.into(),
            bounds: Bounds::new(0, index as i32 * 100, 100, index as i32 * 100 + 40),
            text: Some(text.into()),
            content_desc: None,
            resource_id: None,
            hint: None,
            clickable,
            checkable: false,
            checked: false,
            focusable: false,
            focused: false,
            scrollable: false,
            selected: false,
            enabled: true,
            xpath: None,
            options: None,
            is_password: false,
            z_index: None,
            parent_index: None,
            depth: 0,
            sibling_index: 1,
        }
    }

    /// 真事故的最小复现：标题在前、按钮在后，必须点按钮
    #[test]
    fn prefers_the_clickable_one_over_a_heading() {
        let page = vec![
            el(0, "h1", "Sign In", false),      // 标题——点它等于什么都没做
            el(1, "button", "Sign in", true),   // 用户真正要点的
        ];
        let hit = pick(&page, "Sign in").expect("要能找到");
        assert_eq!(hit.class_name, "button", "选错了：{:?}", hit.text);
    }

    /// 都可点击时，精确匹配的那个才是本尊
    #[test]
    fn prefers_exact_match_over_a_longer_label() {
        let page = vec![
            el(0, "button", "Sign in with Google", true),
            el(1, "button", "Sign in", true),
        ];
        assert_eq!(pick(&page, "Sign in").unwrap().index, 1);
    }

    /// 外层容器把按钮的文字也包进去了——选文字短的那个（更贴近按钮本身）
    #[test]
    fn prefers_the_tighter_element() {
        let page = vec![
            el(0, "div", "已有账号？ Sign in 或注册", true),
            el(1, "span", "Sign in", true),
        ];
        assert_eq!(pick(&page, "Sign in").unwrap().index, 1);
    }

    /// 一个可点击的都没有时照旧给第一个——断言"页面上有这段文字"不需要可点击
    #[test]
    fn falls_back_to_first_match_when_nothing_is_clickable() {
        let page = vec![
            el(0, "h1", "Sign In", false),
            el(1, "p", "Sign in below", false),
        ];
        assert_eq!(pick(&page, "Sign in").unwrap().index, 0);
    }

    #[test]
    fn returns_none_when_absent() {
        assert!(pick(&[el(0, "h1", "Dashboard", false)], "Sign in").is_none());
    }
}
