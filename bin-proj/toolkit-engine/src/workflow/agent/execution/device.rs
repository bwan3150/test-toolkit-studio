// 执行原语：把一个 ControlAction 落到设备，返回简短执行详情

use crate::{Control, ControlAction, Result};

/// 执行一个 ControlAction，返回简短的执行详情字符串
pub async fn exec(device: &str, action: ControlAction) -> Result<String> {
    let control = Control::new(device.to_string())?;
    let v = control.execute(action).await?;
    let detail = v
        .get("action")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| v.to_string());
    Ok(detail)
}
