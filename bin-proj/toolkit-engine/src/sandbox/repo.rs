// 源码沙盒的仓库管理：按 App 存一份裸镜像，按会话开 worktree。
//
// 布局（ADR-0025）：
// ```
// ~/.tke/sandbox/<app_id>/
//   ├── repo.git/          裸镜像，只 fetch 不 checkout
//   └── wt/<session_id>/   每个会话一个 worktree
// ```
//
// **为什么是 worktree 而不是直接 checkout**：同一个 repo 会被多个会话同时用，
// 而每个人测的分支不一样。共用一份工作区意味着 A 的 checkout 会把 B 的代码换掉 ——
// 而这**不会报错**，表现是"线索莫名其妙对不上"（跟 P-63 一个味道）。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Result, TkeError};

/// 一次任务带来的源码坐标（平台随任务下发，见 ADR-0025）
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct SourceSpec {
    /// 仓库地址（https，或本地路径 —— 本地路径用于开发自测）
    pub repo: String,
    /// 被测的那个 ref（分支名或 sha）
    #[serde(default)]
    pub r#ref: String,
    /// 变更面的对照基线（通常是主干）
    #[serde(default)]
    pub base: String,
    /// 短期只读凭据（一小时、仅此 repo）。**不落盘**
    #[serde(default)]
    pub token: String,
    /// 对账用的 commit sha
    #[serde(default)]
    pub commit: String,
}

impl SourceSpec {
    /// 够不够用来开沙盒 —— 缺 repo 或 ref 就当没配（源码是增益，不是前置条件）
    pub fn usable(&self) -> bool {
        !self.repo.trim().is_empty() && !self.r#ref.trim().is_empty()
    }
}

/// 一个 App 的源码沙盒
pub struct Sandbox {
    root: PathBuf,
    app_id: String,
}

impl Sandbox {
    /// 按 App 隔离目录 —— **不按 repo**：ACL 的粒度是 App（ADR-0025 安全边界）
    pub fn new(root: impl AsRef<Path>, app_id: &str) -> Self {
        Self { root: root.as_ref().to_path_buf(), app_id: safe_seg(app_id) }
    }

    /// 默认根目录 `~/.tke/sandbox`
    pub fn default_root() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".tke").join("sandbox")
    }

    fn app_dir(&self) -> PathBuf {
        self.root.join(&self.app_id)
    }
    fn mirror(&self) -> PathBuf {
        self.app_dir().join("repo.git")
    }
    fn worktree(&self, session: &str) -> PathBuf {
        self.app_dir().join("wt").join(safe_seg(session))
    }

    /// 准备一个会话的工作树：确保镜像在、fetch 到最新、开出 worktree。
    ///
    /// checkout 到 **sha**（`spec.commit`）而不是分支名 —— 分支会动，
    /// 而线索必须对得上正在被测的那个包（ADR-0025「版本对账」）。
    /// 没给 sha 就用 ref，并在返回值里带出实际的 sha 供上层对账。
    pub fn prepare(&self, spec: &SourceSpec, session: &str) -> Result<PreparedTree> {
        if !spec.usable() {
            return Err(TkeError::InvalidArgument("源码坐标不全（要 repo 和 ref）".into()));
        }
        std::fs::create_dir_all(self.app_dir())?;
        self.ensure_mirror(spec)?;
        self.fetch(spec)?;

        let wt = self.worktree(session);
        if wt.exists() {
            // 同一个会话重复 prepare：先摘掉旧的，别在半个工作树上接着用
            let _ = self.remove(session);
        }
        std::fs::create_dir_all(wt.parent().unwrap())?;

        let target = if spec.commit.trim().is_empty() { spec.r#ref.clone() } else { spec.commit.clone() };
        git(&self.mirror(), &["worktree", "add", "--detach", &wt.to_string_lossy(), &target])?;

        let sha = git(&wt, &["rev-parse", "HEAD"])?.trim().to_string();
        Ok(PreparedTree { path: wt, sha })
    }

    /// 摘掉一个会话的工作树。**镜像留着** —— 下次 fetch 是增量
    pub fn remove(&self, session: &str) -> Result<()> {
        let wt = self.worktree(session);
        if !wt.exists() {
            return Ok(());
        }
        // 先让 git 摘掉登记，再删目录 —— 只删目录会在镜像里留一条悬空的 worktree 记录
        let _ = git(&self.mirror(), &["worktree", "remove", "--force", &wt.to_string_lossy()]);
        if wt.exists() {
            std::fs::remove_dir_all(&wt)?;
            let _ = git(&self.mirror(), &["worktree", "prune"]);
        }
        Ok(())
    }

    fn ensure_mirror(&self, spec: &SourceSpec) -> Result<()> {
        let m = self.mirror();
        if m.join("HEAD").exists() {
            return Ok(());
        }
        let url = with_token(&spec.repo, &spec.token);
        git(&self.root, &["clone", "--bare", &url, &m.to_string_lossy()])?;
        Ok(())
    }

    fn fetch(&self, spec: &SourceSpec) -> Result<()> {
        let url = with_token(&spec.repo, &spec.token);
        // 显式给 refspec：裸镜像默认不一定带 fetch 配置
        let mut args = vec!["fetch".to_string(), "--prune".to_string(), url];
        args.push("+refs/heads/*:refs/heads/*".to_string());
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        git(&self.mirror(), &refs)?;
        // base 若是个远端分支，上一条已经拿到了；这里只做存在性检查，缺了让上层退回无基线模式
        if !spec.base.trim().is_empty() {
            let _ = git(&self.mirror(), &["rev-parse", "--verify", &spec.base]);
        }
        Ok(())
    }
}

/// 准备好的工作树
pub struct PreparedTree {
    pub path: PathBuf,
    /// 实际 checkout 到的 sha —— 报告里要写清它（复盘时才说得清"线索为什么不准"）
    pub sha: String,
}

/// 把短期 token 塞进 https URL。
///
/// **只在内存里拼，不写进 git config**：写进去就等于落盘了，
/// 而这把凭据的全部意义就是"用完即弃"（ADR-0025）。
fn with_token(repo: &str, token: &str) -> String {
    let t = token.trim();
    if t.is_empty() || !repo.starts_with("https://") {
        return repo.to_string();
    }
    format!("https://x-access-token:{t}@{}", &repo["https://".len()..])
}

/// 目录名里不能有路径分隔符 —— app_id / session_id 来自外部，
/// 不做这一步就是一条目录穿越
fn safe_seg(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// 跑一条 git，失败带上 stderr（成功路径也别把 stderr 丢了 —— P-65 的教训）
pub(crate) fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| TkeError::InvalidArgument(format!("跑不起来 git：{e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        // **不回显完整命令行** —— 它可能带着短期 token
        return Err(TkeError::InvalidArgument(format!(
            "git {} 失败：{}",
            args.first().copied().unwrap_or("?"),
            err.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token只在内存里拼() {
        assert_eq!(
            with_token("https://github.com/acme/app.git", "ghs_x"),
            "https://x-access-token:ghs_x@github.com/acme/app.git"
        );
        // 本地路径不动
        assert_eq!(with_token("/srv/repo", "ghs_x"), "/srv/repo");
        // 没 token 不动
        assert_eq!(with_token("https://a/b.git", ""), "https://a/b.git");
    }

    #[test]
    fn 目录名不许穿越() {
        assert_eq!(safe_seg("../../etc"), "______etc");
        assert_eq!(safe_seg("app-1_2"), "app-1_2");
    }

    #[test]
    fn 坐标不全就当没配() {
        assert!(!SourceSpec::default().usable());
        assert!(!SourceSpec { repo: "x".into(), ..Default::default() }.usable());
        assert!(SourceSpec { repo: "x".into(), r#ref: "main".into(), ..Default::default() }.usable());
    }
}
