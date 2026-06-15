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

                let text = if !value.is_empty() { value.as_str() } else { label.as_str() };
                xml.push_str(&format!(
                    "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"true\" bounds=\"[{},{}][{},{}]\" />\n",
                    escape_attr(&typ),
                    escape_attr(&name),
                    escape_attr(&label),
                    escape_attr(text),
                    clickable,
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

