// XCUI 元素树归一化 - 把 WDA 的 /source XML 转成 uiautomator 风格扁平 XML
// 让 iOS 元素与 App/Web 元素进入同一套解析/识别/标注体系：
//   class=XCUIElementType*, resource-id=name, content-desc=label,
//   text=value|label, bounds=点×scale 像素；仅保留可见且有面积的元素。
// 纯文本转换，不依赖驱动实例。

use crate::utils::xml::escape_attr;
use crate::{Result, TkeError};

/// XCUI 元素树 → uiautomator 风格扁平 XML（仅可见元素，bounds 换算为像素）
pub(super) fn normalize_xcui_xml(source: &str, scale: f64) -> Result<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let mut typ = String::new();
                let mut name = String::new();
                let mut label = String::new();
                let mut value = String::new();
                let mut visible = false;
                let mut accessible = false;
                let (mut x, mut y, mut w, mut h) = (0i64, 0i64, 0i64, 0i64);

                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    match key.as_str() {
                        "type" => typ = val,
                        "name" => name = val,
                        "label" => label = val,
                        "value" => value = val,
                        "visible" => visible = val == "true",
                        "accessible" => accessible = val == "true",
                        "x" => x = val.parse().unwrap_or(0),
                        "y" => y = val.parse().unwrap_or(0),
                        "width" => w = val.parse().unwrap_or(0),
                        "height" => h = val.parse().unwrap_or(0),
                        _ => {}
                    }
                }

                // 只保留可见且有面积的元素；跳过 Application/Window 等纯容器
                let is_container = matches!(
                    typ.as_str(),
                    "XCUIElementTypeApplication" | "XCUIElementTypeWindow" | "XCUIElementTypeOther"
                ) && name.is_empty() && label.is_empty() && value.is_empty();
                if !visible || w <= 0 || h <= 0 || is_container {
                    buf.clear();
                    continue;
                }

                // 可交互类型（uiautomator clickable 语义）
                let clickable = matches!(
                    typ.as_str(),
                    "XCUIElementTypeButton" | "XCUIElementTypeCell" | "XCUIElementTypeLink"
                        | "XCUIElementTypeSwitch" | "XCUIElementTypeTextField"
                        | "XCUIElementTypeSecureTextField" | "XCUIElementTypeSearchField"
                        | "XCUIElementTypeTabBar" | "XCUIElementTypeSegmentedControl"
                ) || accessible;

                // **密码框必须标出来**（同安卓原生的 password="true" / web 归一化后的同名属性）。
                // 有它，命令原文里的值才会在落盘前打码——log.json、报告、**截图顶部横幅**
                // 都是要发给别人看的证据。
                // ⚠️ 这里曾经漏掉，而安卓与 web 都做了，注释还写着"三平台同一条路"——
                // 于是 iOS 上 `输入 ["Password", "真密码"]` 一路明文写进报告，没人发现（P-45）。
                let is_password = typ == "XCUIElementTypeSecureTextField";

                let text = if !value.is_empty() { value.as_str() } else { label.as_str() };
                xml.push_str(&format!(
                    "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"true\"{} bounds=\"[{},{}][{},{}]\" />\n",
                    escape_attr(&typ),
                    escape_attr(&name),
                    escape_attr(&label),
                    escape_attr(text),
                    clickable,
                    if is_password { " password=\"true\"" } else { "" },
                    (x as f64 * scale) as i64,
                    (y as f64 * scale) as i64,
                    ((x + w) as f64 * scale) as i64,
                    ((y + h) as f64 * scale) as i64,
                ));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(TkeError::DeviceError(format!("XCUI XML 解析失败: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    xml.push_str("</hierarchy>\n");
    Ok(xml)
}


#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"<XCUIElementTypeApplication type="XCUIElementTypeApplication" visible="true" x="0" y="0" width="393" height="852">
  <XCUIElementTypeTextField type="XCUIElementTypeTextField" name="Email" label="Email" visible="true" x="36" y="151" width="330" height="22"/>
  <XCUIElementTypeSecureTextField type="XCUIElementTypeSecureTextField" name="Password" label="Password" visible="true" x="36" y="213" width="306" height="18"/>
  <XCUIElementTypeButton type="XCUIElementTypeButton" name="Sign In" label="Sign In" visible="true" x="177" y="273" width="48" height="18"/>
</XCUIElementTypeApplication>"#;

    /// **密码框必须标出来**：命令原文里的值靠它才会在落盘前打码，
    /// 而 log.json / 报告 / 截图顶部横幅都是要发给别人看的证据。
    /// 这条曾经漏了整整一个平台——安卓原生有、web 归一化时对齐了，唯独 iOS 没做，
    /// 于是真密码一路明文写进报告（P-45）
    #[test]
    fn marks_secure_text_field_as_password() {
        let xml = normalize_xcui_xml(SRC, 3.0).unwrap();
        let pwd = xml.lines().find(|l| l.contains("SecureTextField")).expect("要有密码框");
        assert!(pwd.contains(r#"password="true""#), "密码框要标出来:{}", pwd);
        let mail = xml.lines().find(|l| l.contains(r#"text="Email""#)).unwrap();
        assert!(!mail.contains("password"), "普通输入框别误标:{}", mail);
    }

    /// 坐标要 ×scale 换成截图像素——点击全靠它
    #[test]
    fn scales_bounds_to_screenshot_pixels() {
        let xml = normalize_xcui_xml(SRC, 3.0).unwrap();
        assert!(xml.contains(r#"bounds="[531,819][675,873]""#), "坐标要×3:{}", xml);
    }
}
