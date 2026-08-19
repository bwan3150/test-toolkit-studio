// AX 元素树归一化 - 把 `idb ui describe-all` 的 JSON 转成 uiautomator 风格扁平 XML。
//
// 让模拟器上的元素与 App/Web 元素进入同一套解析/识别/标注体系,跟 wda/normalize.rs
// 是同一件事的两个来源（那边是 XCUI 树,这边是 AX 树）。
//
// 与 XCUI 的对应关系：
//   AXLabel   ←→ label     → content-desc
//   AXValue   ←→ value     → text（输入框的占位符/当前值也在这儿）
//   role/type ←→ type      → class
//   frame     ←→ x/y/w/h   → bounds（**单位是「点」,要 ×scale 才是截图像素**）
//
// 纯文本转换,不依赖驱动实例——所以能拿真机数据当夹具直接测。

use crate::utils::xml::escape_attr;
use crate::{Result, TkeError};

/// AX 元素数组 → uiautomator 风格扁平 XML（bounds 换算为截图像素）
pub(super) fn normalize_ax_json(json: &str, scale: f64) -> Result<String> {
    let list: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| TkeError::DeviceError(format!("idb 元素树解析失败: {}", e)))?;
    let Some(items) = list.as_array() else {
        return Err(TkeError::DeviceError("idb 元素树不是数组".into()));
    };

    let mut xml = String::from("<?xml version='1.0' encoding='UTF-8'?>\n<hierarchy rotation=\"0\">\n");
    for e in items {
        let typ = e["type"].as_str().unwrap_or_default();
        let label = e["AXLabel"].as_str().unwrap_or_default();
        let value = e["AXValue"].as_str().unwrap_or_default();
        let uid = e["AXUniqueId"].as_str().unwrap_or_default();
        let f = &e["frame"];
        let (x, y, w, h) = (
            f["x"].as_f64().unwrap_or(0.0),
            f["y"].as_f64().unwrap_or(0.0),
            f["width"].as_f64().unwrap_or(0.0),
            f["height"].as_f64().unwrap_or(0.0),
        );

        // 整块 App 容器（跟屏幕一样大、没有自己的文字）不是元素,采它只会挡住命中
        if typ == "Application" || w <= 0.0 || h <= 0.0 {
            continue;
        }
        // 既没文字也没标注的纯容器：留着只会让元素表变长，AI 还是认不出它是什么
        if label.is_empty() && value.is_empty() && uid.is_empty() {
            continue;
        }

        // ⚠️ **不能拿 traits 判可点击**：实测 `Scrollable` 连 StaticText 都带,
        // 一律当可点会把满屏文字都标成能点的（P-35 那条"可点击优先"就废了）。
        // 看 role/type 才准。
        let clickable = matches!(
            typ,
            "Button" | "Cell" | "Link" | "Switch" | "TextField" | "SecureTextField"
                | "SearchField" | "SegmentedControl" | "Slider" | "Stepper"
        );

        // 密码框：subrole 或 traits 里都可能标,取其一即可。
        // 有它才能在**证据落盘前**把值打码（同 Android 的 password="true"）
        let is_password = e["subrole"].as_str() == Some("AXSecureTextField")
            || e["traits"]
                .as_array()
                .is_some_and(|t| t.iter().any(|x| x.as_str() == Some("SecureTextField")));

        // text 取 value 优先（输入框里显示的是它）,否则退回 label——与 XCUI 那边一致
        let text = if !value.is_empty() { value } else { label };

        xml.push_str(&format!(
            "  <node class=\"{}\" resource-id=\"{}\" content-desc=\"{}\" text=\"{}\" clickable=\"{}\" enabled=\"{}\"{} bounds=\"[{},{}][{},{}]\" />\n",
            escape_attr(typ),
            escape_attr(uid),
            escape_attr(label),
            escape_attr(text),
            clickable,
            e["enabled"].as_bool().unwrap_or(true),
            if is_password { " password=\"true\"" } else { "" },
            (x * scale) as i64,
            (y * scale) as i64,
            ((x + w) * scale) as i64,
            ((y + h) * scale) as i64,
        ));
    }
    xml.push_str("</hierarchy>\n");
    Ok(xml)
}

/// 屏幕缩放：**截图像素 ÷ AX 坐标**。
///
/// idb 的坐标（describe-all 的 frame、`ui tap` 的参数）单位都是「点」,
/// 而 tke 对外一律用**截图像素**。两者差一个 scale,靠 `AXApplication` 那条元素
/// 的宽度与截图宽度相除就能算出来——**不用再调一次接口**（WDA 那边要问 /wda/screen）。
pub(super) fn scale_from(json: &str, screenshot_width: u32) -> Option<f64> {
    let list: serde_json::Value = serde_json::from_str(json).ok()?;
    let app_w = list
        .as_array()?
        .iter()
        .find(|e| e["type"].as_str() == Some("Application"))?["frame"]["width"]
        .as_f64()?;
    if app_w <= 0.0 {
        return None;
    }
    Some(screenshot_width as f64 / app_w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真机（模拟器）实采的一页登录界面——iPhone、iOS 26、截图 1206×2622、AX 宽 402（scale=3）
    const REAL: &str = r#"[
      {"role":"AXApplication","AXUniqueId":null,"title":null,"frame":{"height":874,"x":0,"y":0,"width":402},"AXValue":null,"enabled":true,"type":"Application","AXLabel":"Example App","subrole":null,"traits":["None"]},
      {"role":"AXStaticText","AXUniqueId":null,"enabled":true,"traits":["StaticText","Scrollable"],"subrole":null,"type":"StaticText","AXValue":null,"AXLabel":"Sign in","frame":{"y":86,"width":100.66,"height":38.33,"x":24}},
      {"AXValue":"Email","role":"AXTextField","AXLabel":null,"AXUniqueId":null,"enabled":true,"type":"TextField","subrole":null,"frame":{"width":330,"y":151.33,"x":36,"height":22},"traits":["TextOperationsAvailable","Scrollable","TextEntry"]},
      {"enabled":true,"AXUniqueId":null,"role":"AXTextField","subrole":"AXSecureTextField","traits":["TextEntry","SecureTextField","TextOperationsAvailable","Scrollable"],"AXValue":"Password","AXLabel":null,"type":"TextField","frame":{"height":18,"width":306,"y":213.33,"x":36}},
      {"role":"AXButton","enabled":true,"traits":["Button","Scrollable"],"AXLabel":"Sign in","subrole":null,"AXValue":null,"type":"Button","frame":{"height":18,"y":273.33,"x":177,"width":48.33}},
      {"role":"AXButton","enabled":true,"traits":["Button","Scrollable"],"AXLabel":"Skip sign in (demo)","subrole":null,"AXValue":null,"type":"Button","frame":{"height":18,"x":132.33,"width":137.33,"y":393.33}}
    ]"#;

    #[test]
    fn scale_comes_from_the_app_element() {
        // 截图 1206 像素宽 ÷ AX 402 点 = 3
        assert_eq!(scale_from(REAL, 1206), Some(3.0));
        assert_eq!(scale_from("[]", 1206), None, "没有 Application 那条就算不出来");
    }

    #[test]
    fn converts_ax_tree_to_uiautomator_xml() {
        let xml = normalize_ax_json(REAL, 3.0).unwrap();
        // 按钮：文字进 content-desc 与 text，可点击
        assert!(
            xml.contains(r#"content-desc="Skip sign in (demo)""#),
            "按钮文字要留下:{}", xml
        );
        // 坐标换算成截图像素：x=132.33*3≈396, y=393.33*3≈1179
        assert!(xml.contains(r#"bounds="[396,1179][808,1233]""#), "坐标要×scale:{}", xml);
        // 整块 App 容器不该出现（它跟屏幕一样大，会挡住所有命中）
        assert!(!xml.contains(r#"class="Application""#), "Application 容器要跳过:{}", xml);
    }

    /// 密码框必须认出来——证据（命令原文/页面结构/截图横幅）落盘前要靠它打码
    #[test]
    fn marks_secure_text_field_as_password() {
        let xml = normalize_ax_json(REAL, 3.0).unwrap();
        let pwd_line = xml.lines().find(|l| l.contains(r#"text="Password""#)).expect("要有密码框");
        assert!(pwd_line.contains(r#"password="true""#), "密码框要标出来:{}", pwd_line);
        let mail_line = xml.lines().find(|l| l.contains(r#"text="Email""#)).unwrap();
        assert!(!mail_line.contains("password"), "普通输入框别误标:{}", mail_line);
    }

    /// **不能拿 traits 判可点击**：实测 Scrollable 连 StaticText 都带，
    /// 一律当可点会把满屏文字都标成能点的
    #[test]
    fn static_text_is_not_clickable_despite_scrollable_trait() {
        let xml = normalize_ax_json(REAL, 3.0).unwrap();
        let txt = xml.lines().find(|l| l.contains(r#"class="StaticText""#)).expect("要有静态文字");
        assert!(txt.contains(r#"clickable="false""#), "静态文字不是按钮:{}", txt);
        let btn = xml.lines().find(|l| l.contains(r#"content-desc="Sign in""#) && l.contains("Button")).unwrap();
        assert!(btn.contains(r#"clickable="true""#), "按钮要可点:{}", btn);
    }
}
