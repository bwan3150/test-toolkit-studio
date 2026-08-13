// XML 工具 - 归一化结构文件生成时的属性转义
// 各驱动（wda/web）把元素树归一化为 uiautomator 风格 XML 时共用，避免重复实现。

/// 转义 XML 属性值中的特殊字符（换行折叠为空格，保证单行节点）
pub fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', " ")
}
