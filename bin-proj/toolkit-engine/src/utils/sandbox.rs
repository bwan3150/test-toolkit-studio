// 【工作区沙箱】把调用方给的相对路径解析到工作区树内，拒绝绝对路径与 `..` 跳出。
//
// 原本长在 `workflow/agent/runner/orchestrator.rs` 里（AI 写文件用）。远程 serve 要用同一条规则
// 挡住 `tke ocr --image /etc/passwd` 这类越界，于是搬到 utils —— **一条规则只能有一处实现**，
// 两处各写一份迟早会漂（历史上"同一个问题两套答案"已经吃过亏）。

use std::path::{Component, Path, PathBuf};

/// 把 `requested` 解析到工作区 `root` 内。
/// 允许相对子目录（如 `docs/policy.md`），**拒绝绝对路径和 `..` 跳出**——
/// 与 coding agent 一致：只能动工作区树内的文件。返回解析后的路径或一句拒绝原因。
pub fn resolve_in_workspace(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let req = Path::new(requested.trim());
    if req.as_os_str().is_empty() {
        return Err("未提供 filename。".to_string());
    }
    if req.is_absolute() {
        return Err("只能保存到当前工作目录树内，不接受绝对路径。请给相对文件名（可含子目录，如 docs/policy.md）。".to_string());
    }
    let mut p = root.to_path_buf();
    for comp in req.components() {
        match comp {
            Component::Normal(c) => p.push(c),
            Component::CurDir => {}
            // ParentDir / RootDir / Prefix 一律拒绝（防跳出工作区）
            _ => return Err("路径不能用 `..` 跳出当前目录。".to_string()),
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 相对路径落在工作区内() {
        let root = Path::new("/w");
        assert_eq!(resolve_in_workspace(root, "a/b.txt").unwrap(), PathBuf::from("/w/a/b.txt"));
        assert_eq!(resolve_in_workspace(root, "./a.txt").unwrap(), PathBuf::from("/w/a.txt"));
    }

    #[test]
    fn 绝对路径与跳出一律拒绝() {
        let root = Path::new("/w");
        assert!(resolve_in_workspace(root, "/etc/passwd").is_err());
        assert!(resolve_in_workspace(root, "../../etc/passwd").is_err());
        assert!(resolve_in_workspace(root, "a/../../b").is_err());
        assert!(resolve_in_workspace(root, "   ").is_err());
    }
}
