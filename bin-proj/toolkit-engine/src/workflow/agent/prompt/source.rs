// 外部提示词来源：读取 .md 文件 / 按目录约定取覆盖
//
// 目录约定（存在即覆盖默认，缺失用默认）：
//   <prompts_dir>/agents/<role>.md    角色(subagent)系统提示词，当前角色名为 explorer
//   <prompts_dir>/tools/<name>.md     单个工具的 description

use std::path::Path;

use crate::{Result, TkeError};

/// 读取一个提示词文件（去除首尾空白）
pub fn read_file(path: &Path) -> Result<String> {
    let s = std::fs::read_to_string(path).map_err(|e| {
        TkeError::InvalidArgument(format!("读取提示词文件失败 {}: {}", path.display(), e))
    })?;
    Ok(s.trim().to_string())
}

/// 取某角色(subagent)的系统提示词覆盖：<dir>/agents/<role>.md
pub fn role_override(dir: &Path, role: &str) -> Option<String> {
    read_optional(&dir.join("agents").join(format!("{}.md", role)))
}

/// 取某工具的 description 覆盖：<dir>/tools/<name>.md
pub fn tool_override(dir: &Path, name: &str) -> Option<String> {
    read_optional(&dir.join("tools").join(format!("{}.md", name)))
}

/// 文件存在则读取（非空），否则 None；读取失败也返回 None（容错，回落默认）
fn read_optional(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
