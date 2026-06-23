//! 滚动查找的「多候选关键词」匹配。
//!
//! target 允许用 `|` 分隔多个候选关键词——应对探索时不确定目标的确切文字：
//! 词与词之间有没有空格、措辞是否一致、OCR 大小写/识别是否准。任一候选命中即算找到；
//! 探索命中后可**收敛**到真正匹配上的那个候选（见 first_hit 返回原文），定稿脚本就精确了。

/// 归一化：小写 + 去掉所有空白（与 OCR 渲染的空格/大小写差异脱钩）。
pub fn normalize(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect()
}

/// 把 target 按 `|` 拆成候选列表：(原文trim, 归一needle)。空段忽略。单关键词时只有一项。
pub fn targets(target: &str) -> Vec<(String, String)> {
    target
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| (s.to_string(), normalize(s)))
        .collect()
}

/// 在一批页面元素文本里找：哪个候选被某元素文本（归一后）包含。
/// 命中返回该候选的**原文**（用于把多候选收敛成单一确定关键词）；都没命中返回 None。
/// 按候选给定顺序优先（AI 列在前面的更可能是它最想要的）。
pub fn first_hit<'a>(el_texts: &[String], cands: &'a [(String, String)]) -> Option<&'a str> {
    let hays: Vec<String> = el_texts.iter().map(|t| normalize(t)).collect();
    cands
        .iter()
        .find(|(_, needle)| hays.iter().any(|h| h.contains(needle.as_str())))
        .map(|(orig, _)| orig.as_str())
}
