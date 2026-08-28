// 变更面：这次改动碰到了**哪些界面**。
//
// ADR-0025 P1 只做这一件事，而且**只靠文件路径映射，不解析源码**。
// 理由在 ADR 里：P1 要先回答"这条路值不值钱"（探索轮数 / token 降幅），
// 而路径映射是最便宜的那一档 —— 如果连它都带不来降幅，深度解析也救不回来。
//
// **红线（INV-19）在这里的形状**：返回值只有「界面名 + 文件 + 改动规模」，
// **不含任何代码行**。拿不到实现，就编不出"点了之后应该显示什么"。

use std::collections::BTreeMap;
use std::path::Path;

/// 一个被改动的界面
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Surface {
    /// 界面名（从文件名推的，如 SettingsActivity → Settings）
    pub name: String,
    /// 属于哪一类：activity / layout / page / view_controller / route / other
    pub kind: String,
    /// 相关文件（仓库相对路径）
    pub files: Vec<String>,
    /// 改动规模：增 + 删的行数合计。**只是规模，不是内容**
    pub churn: u32,
}

/// diff 里的一行：文件 + 增删行数
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub added: u32,
    pub deleted: u32,
}

/// 把 `git diff --numstat` 的输出解析成文件列表。
///
/// 二进制文件那两列是 `-`，按 0 行算 —— 它们（图片、字体）确实改了界面，
/// 但"改了多少行"对它们没有意义
pub fn parse_numstat(out: &str) -> Vec<ChangedFile> {
    out.lines()
        .filter_map(|line| {
            let mut it = line.split('\t');
            let a = it.next()?;
            let d = it.next()?;
            let p = it.next()?.trim();
            if p.is_empty() {
                return None;
            }
            Some(ChangedFile {
                path: p.to_string(),
                added: a.parse().unwrap_or(0),
                deleted: d.parse().unwrap_or(0),
            })
        })
        .collect()
}

/// 把改动的文件聚成界面。
///
/// 同一个界面常常散在几个文件里（`SettingsActivity.kt` + `activity_settings.xml`），
/// 所以按**归一化之后的名字**合并 —— 否则一次改动会显示成两个界面，
/// 而 AI 会以为有两个地方要看。
pub fn surfaces_of(files: &[ChangedFile]) -> Vec<Surface> {
    let mut by_name: BTreeMap<(String, String), Surface> = BTreeMap::new();
    for f in files {
        let Some((name, kind)) = classify(&f.path) else { continue };
        let key = (name.to_lowercase(), kind.clone());
        let e = by_name.entry(key).or_insert_with(|| Surface {
            name: name.clone(),
            kind: kind.clone(),
            files: vec![],
            churn: 0,
        });
        e.files.push(f.path.clone());
        e.churn += f.added + f.deleted;
    }
    let mut out: Vec<Surface> = by_name.into_values().collect();
    // **改得最多的排前面**：AI 一次看不了二十个界面，得先看最可能出问题的那几个
    out.sort_by(|a, b| b.churn.cmp(&a.churn).then(a.name.cmp(&b.name)));
    out
}

/// 一个文件属于哪个界面。返回 None = 它不是界面文件（构建脚本、工具类、测试……）
///
/// **宁可漏，不可错**：报一个不存在的界面，AI 会去找一个不存在的东西 ——
/// 那比没有线索更慢（ADR-0025「版本对账」同一条道理）。
fn classify(path: &str) -> Option<(String, String)> {
    let p = Path::new(path);
    let file = p.file_name()?.to_str()?;
    let stem = p.file_stem()?.to_str()?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lower = path.to_lowercase();

    // 测试代码不是界面 —— 它改了不代表被测界面改了
    if lower.contains("/test/") || lower.contains("/tests/") || lower.contains("__tests__")
        || stem.ends_with("Test") || stem.ends_with("_test") || stem.ends_with(".spec")
        || stem.ends_with(".test")
    {
        return None;
    }

    // Android：Activity / Fragment / 布局 xml
    if matches!(ext, "kt" | "java") {
        if let Some(base) = stem.strip_suffix("Activity") {
            return Some((base.to_string(), "activity".into()));
        }
        if let Some(base) = stem.strip_suffix("Fragment") {
            return Some((base.to_string(), "fragment".into()));
        }
        return None; // 别的 kt/java 是逻辑，不是界面
    }
    if ext == "xml" && lower.contains("/res/layout") {
        // activity_settings.xml / fragment_home.xml → Settings / Home
        let base = stem
            .strip_prefix("activity_")
            .or_else(|| stem.strip_prefix("fragment_"))
            .unwrap_or(stem);
        return Some((camel(base), "layout".into()));
    }

    // iOS：ViewController / SwiftUI View / storyboard
    if ext == "swift" {
        if let Some(base) = stem.strip_suffix("ViewController") {
            return Some((base.to_string(), "view_controller".into()));
        }
        if let Some(base) = stem.strip_suffix("View") {
            return Some((base.to_string(), "view".into()));
        }
        return None;
    }
    if matches!(ext, "storyboard" | "xib") {
        return Some((stem.to_string(), "storyboard".into()));
    }

    // Web：页面组件（按目录判断，不按后缀 —— 后缀说不了它是不是一个页面）
    if matches!(ext, "vue" | "tsx" | "jsx" | "svelte") {
        if lower.contains("/pages/") || lower.contains("/views/") || lower.contains("/screens/")
            || lower.contains("/routes/")
        {
            return Some((stem.to_string(), "page".into()));
        }
        // components/ 下的是零件，不是界面
        return None;
    }

    // 路由表：改了它意味着**导航结构**变了，值得单列
    if matches!(ext, "js" | "ts") && (file.starts_with("router") || lower.contains("/router/")) {
        return Some(("路由".into(), "route".into()));
    }

    None
}

/// activity_order_detail → OrderDetail
fn camel(s: &str) -> String {
    s.split('_')
        .filter(|x| !x.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cf(p: &str, a: u32, d: u32) -> ChangedFile {
        ChangedFile { path: p.into(), added: a, deleted: d }
    }

    #[test]
    fn numstat_二进制那两列是横杠() {
        let out = "12\t3\tapp/src/A.kt\n-\t-\tapp/res/drawable/logo.png\n";
        let v = parse_numstat(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].added, 12);
        assert_eq!(v[1].added, 0);
    }

    #[test]
    fn 同一个界面的几个文件要合并() {
        let files = vec![
            cf("app/src/main/java/com/x/SettingsActivity.kt", 30, 5),
            cf("app/src/main/res/layout/activity_settings.xml", 10, 2),
        ];
        let s = surfaces_of(&files);
        // Activity 和 layout 归到同一个名字下（kind 不同所以是两条，但名字一致）
        assert!(s.iter().all(|x| x.name == "Settings"), "{s:?}");
    }

    #[test]
    fn 改得最多的排前面() {
        let files = vec![
            cf("src/pages/Home.vue", 2, 0),
            cf("src/pages/Checkout.vue", 100, 40),
        ];
        let s = surfaces_of(&files);
        assert_eq!(s[0].name, "Checkout");
    }

    #[test]
    fn 不是界面的东西不该被报出来() {
        for p in [
            "build.gradle",
            "app/src/main/java/com/x/NetworkClient.kt",   // 逻辑类
            "src/components/Button.vue",                   // 零件不是界面
            "app/src/test/java/com/x/SettingsActivityTest.kt", // 测试
            "src/pages/__tests__/Home.spec.ts",
            "README.md",
        ] {
            assert!(classify(p).is_none(), "{p} 不该被当成界面");
        }
    }

    #[test]
    fn 三个平台各认一种() {
        assert_eq!(classify("a/OrderActivity.kt").unwrap().1, "activity");
        assert_eq!(classify("a/CartViewController.swift").unwrap().1, "view_controller");
        assert_eq!(classify("src/views/Profile.vue").unwrap().1, "page");
        assert_eq!(classify("src/router/index.ts").unwrap().0, "路由");
    }

    #[test]
    fn 布局文件名要还原成界面名() {
        assert_eq!(classify("app/res/layout/activity_order_detail.xml").unwrap().0, "OrderDetail");
    }
}
