//! 安全域提示词体系：与 harness 同构，但**独立一套**（ADR-0019：工具/角色/提示词另起一套）。
//!
//! - 内置默认：`builtin/agents/<role>.md`、`builtin/tools/<role>/<name>.md`，编译期 `include_str!` 内嵌，
//!   单二进制自包含。
//! - 外部覆盖：给一个目录，布局与 builtin **完全一致**，即可逐角色/逐工具覆盖——拷 builtin 出去改即可。
//! - 回落链：外部目录文件 > 内置默认。
//! - 空串守卫：解析为空一律炸/占位，杜绝「静默给 LLM 发空 system」（harness P 教训同款）。
//!
//! 角色：prober（先落）/ analyst / reporter（后续）。占位由调用点用 `render` 填充。

use std::path::PathBuf;

/// 已解析的安全提示词集合。
#[derive(Clone, Default)]
pub struct SecurityPrompts {
    /// 外部覆盖目录（None = 全走内置默认）。
    override_dir: Option<PathBuf>,
}

impl SecurityPrompts {
    /// override_dir 布局须与 builtin 一致：`agents/<role>.md`、`tools/<role>/<name>.md`。
    pub fn load(override_dir: Option<PathBuf>) -> Self {
        Self { override_dir }
    }

    /// 某角色的系统提示词（外部 `agents/<role>.md` 覆盖 > 内置默认）。
    pub fn system(&self, role: &str) -> String {
        let raw = self
            .override_dir
            .as_ref()
            .and_then(|d| read_opt(&d.join("agents").join(format!("{role}.md"))))
            .unwrap_or_else(|| default_agent(role).to_string());
        guard_nonempty(raw, &format!("agents/{role}.md"))
    }

    /// 某角色某工具的 description（外部 `tools/<role>/<name>.md` 覆盖 > 内置默认）。
    pub fn tool(&self, role: &str, name: &str) -> String {
        let raw = self
            .override_dir
            .as_ref()
            .and_then(|d| read_opt(&d.join("tools").join(role).join(format!("{name}.md"))))
            .unwrap_or_else(|| default_tool(role, name).to_string());
        guard_nonempty(raw, &format!("tools/{role}/{name}.md"))
    }
}

/// 渲染模板：把 `{key}` 替换为给定值（沿用 harness 那套，无外部模板引擎；未列出的 `{...}` 原样保留）。
pub fn render(tmpl: &str, vars: &[(&str, &str)]) -> String {
    let mut s = tmpl.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

fn read_opt(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn guard_nonempty(s: String, what: &str) -> String {
    let s = s.trim().to_string();
    if !s.is_empty() {
        return s;
    }
    debug_assert!(false, "内置安全提示词缺失或为空：{what}（检查 include_str! 登记与文件内容）");
    format!("【内置安全提示词缺失：{what}】")
}

// ── 内置默认（一提示词一文件，布局同外部覆盖目录）──────────────────────

fn default_agent(role: &str) -> &'static str {
    match role {
        "prober" => include_str!("builtin/agents/prober.md"),
        _ => include_str!("builtin/agents/prober.md"), // 未知角色兜底（不应出现）
    }
}

fn default_tool(role: &str, name: &str) -> &'static str {
    match (role, name) {
        ("prober", "http") => include_str!("builtin/tools/prober/http.md"),
        ("prober", "recon") => include_str!("builtin/tools/prober/recon.md"),
        ("prober", "record_finding") => include_str!("builtin/tools/prober/record_finding.md"),
        ("prober", "finish") => include_str!("builtin/tools/prober/finish.md"),
        _ => "（工具说明缺失）",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_prober_prompt_nonempty_and_has_placeholders() {
        let p = SecurityPrompts::load(None);
        let sys = p.system("prober");
        assert!(sys.contains("{target}"), "prober 系统提示词应含 {{target}} 占位");
        assert!(!p.tool("prober", "http").is_empty());
        assert!(!p.tool("prober", "finish").is_empty());
    }

    #[test]
    fn render_fills_only_given_keys() {
        let out = render("打 {target}（{mode} 档）", &[("target", "https://x"), ("mode", "safe")]);
        assert_eq!(out, "打 https://x（safe 档）");
    }
}
