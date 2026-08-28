// 【源码沙盒】让 harness 拿着图纸测，而不是盲盒探索（ADR-0025）。
//
// **一条红线（INV-19）**：源码只回答「怎么找、去哪看」，绝不回答「结果应该是什么」。
// 它不是靠提示词自觉，是靠**接口形状**兑现的 —— 这里所有对外返回值
// 只含界面名、文件路径、改动规模，**没有一行代码**。
// 拿不到实现，就编不出"点了之后应该显示什么"，也就写不出
// "根据 SettingsActivity.kt:142，此处应显示成功提示 → 通过" 这种同义反复。
//
// P1 只做 `changed_surfaces`（变更面聚焦），判据是探索轮数 / token 降幅 ——
// 降不下来就该及早停，而不是把三个阶段都做完才发现这条路不值钱。

pub mod repo;
pub mod surfaces;

pub use repo::{PreparedTree, Sandbox, SourceSpec};
pub use surfaces::{ChangedFile, Surface};

use std::path::Path;

use crate::Result;

/// 算一个工作树相对 base 的变更面。
///
/// base 为空或对不上时**返回空**而不是报错 —— 源码是增益不是前置条件，
/// "算不出变更面"该退回今天的盲盒模式，不该让整次探索停摆。
pub fn changed_surfaces(tree: &Path, base: &str) -> Result<Vec<Surface>> {
    let base = base.trim();
    if base.is_empty() {
        return Ok(vec![]);
    }
    // `base...HEAD`（三点）= 从共同祖先算起，只看这条分支自己改了什么。
    // 两点会把 base 上的新提交也算进来 —— 那些不是"这次改动"
    let spec = format!("{base}...HEAD");
    let out = match repo::git(tree, &["diff", "--numstat", &spec]) {
        Ok(o) => o,
        // base 不存在（浅克隆没拿到、名字写错）—— 说不出变更面，但别拦住探索
        Err(_) => return Ok(vec![]),
    };
    Ok(surfaces::surfaces_of(&surfaces::parse_numstat(&out)))
}

/// 一个工作树当前在哪个 sha 上。读不出来就空 —— 上层拿它做对账，
/// 空值的含义是"对不了账"，不是"对上了"
pub fn head_sha(tree: &Path) -> String {
    repo::git(tree, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// 渲染给 AI 看的一段话。
///
/// **刻意很短**：它的全部作用是"把注意力先放到这几个界面上"。
/// 长了反而占掉本来要省的那些 token（P1 的判据就是 token 降幅）。
pub fn render_for_ai(surfaces: &[Surface], limit: usize) -> String {
    if surfaces.is_empty() {
        return "这次改动没有识别出界面变更（或没有配置基线）。".into();
    }
    let mut s = format!("这次改动涉及 {} 个界面（按改动规模排序）：\n", surfaces.len());
    for x in surfaces.iter().take(limit) {
        s.push_str(&format!("- {}（{}，{} 行）\n", x.name, x.kind, x.churn));
    }
    if surfaces.len() > limit {
        // **说出来**：静默截断会让 AI 以为这就是全部
        s.push_str(&format!("…还有 {} 个没列出\n", surfaces.len() - limit));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sf(name: &str, churn: u32) -> Surface {
        Surface { name: name.into(), kind: "page".into(), files: vec![], churn }
    }

    #[test]
    fn 没有变更面也要说人话() {
        assert!(render_for_ai(&[], 5).contains("没有识别出"));
    }

    #[test]
    fn 截断要说出来() {
        let v: Vec<Surface> = (0..10).map(|i| sf(&format!("P{i}"), 10)).collect();
        let s = render_for_ai(&v, 3);
        assert!(s.contains("还有 7 个没列出"), "{s}");
    }

    #[test]
    fn 渲染里不能有代码() {
        // 红线的回归测试：返回值里只有名字/类型/规模
        let s = render_for_ai(&[sf("Checkout", 42)], 5);
        assert!(s.contains("Checkout") && s.contains("42"));
        assert!(!s.contains("fn ") && !s.contains("class "));
    }
}
