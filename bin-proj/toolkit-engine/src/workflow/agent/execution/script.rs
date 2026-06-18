// .tks 脚本：文本工具 + 写出

use std::path::Path;

use crate::{Result, TkeError};

/// 写出 .tks 脚本（含用例注释 + 步骤）
pub fn write_script(path: &Path, case: &str, lines: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(TkeError::IoError)?;
    }
    let mut content = String::new();
    content.push_str("# 由 tke harness 自动生成\n");
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

// 方向中英映射、文本转义已收敛到 Phase 2 序列化器（parser/serialize.rs），此处不再重复。
