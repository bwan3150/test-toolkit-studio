// .tks 脚本：文本工具 + 写出

use std::path::Path;

use crate::{Result, TkeError};

/// 写出 .tks 脚本（含用例注释 + 步骤）
pub fn write_script(path: &Path, case: &str, lines: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
    }
    let mut content = String::new();
    content.push_str("# 由 tke case 自动生成\n");
    for l in case.lines() {
        content.push_str(&format!("# 用例: {}\n", l));
    }
    content.push_str("步骤:\n");
    for line in lines {
        content.push_str(line);
        content.push('\n');
    }
    std::fs::write(path, content).map_err(TkeError::IoError)?;
    Ok(())
}

/// 方向英文 → .tks 用的中文（定向滑动参数）
pub fn direction_cn(d: &str) -> &'static str {
    match d {
        "up" => "上",
        "down" => "下",
        "left" => "左",
        "right" => "右",
        _ => "上",
    }
}

/// .tks 文本参数里的引号转义（简单处理：双引号→单引号）
pub fn escape_text(s: &str) -> String {
    s.replace('"', "'")
}
