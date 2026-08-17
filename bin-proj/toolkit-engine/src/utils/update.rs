// 【版本新鲜度】本地这套东西是不是已经落后于分发源了。
//
// 存在理由（Q-11，实打实撞过）：用户机器上装好的 skill **装完就不动了**，没有任何东西会
// 告诉他有新版。一次会话里改了 SKILL.md 的四个修复，用户重跑时拿到的仍是两天前的旧文档，
// 于是必然得出"没改善"的结论——而我们会以为是修复无效。**改完文档不等于送到了用户手上。**
//
// 只读不写、只提醒不代劳（用户 2026-08-17 拍板）：
//   - 发现旧版**只打印一行**，附上更新命令，绝不自己去覆盖二进制——
//     覆盖正在运行的 exe 在三个平台各有各的坑（Windows 锁文件 / Linux ETXTBSY /
//     macOS 签名失配），install.sh 已经把这些踩平了，没必要在 Rust 里再踩一遍
//   - 这不违反 ADR-0012「下载只在 tke fix 里发生」：**这里只读一个几十字节的 VERSION**，
//     不下载任何依赖
//
// 结果缓存 4 小时：`tke steps` 每批都会问一次，但真正联网每 4 小时最多一次——
// 既防住"一直用旧版"，又不给每条命令加几百 ms。离线/内网一律静默跳过。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 缓存多久算新鲜。与分发源 Cloudflare 的 4h 缓存对齐——查得更勤也拿不到更新的答案
pub const MAX_AGE_SECS: u64 = 4 * 3600;

const DEFAULT_BASE_URL: &str =
    "https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke";

/// 分发源 VERSION 的内容：
/// ```text
/// tke 0.7.4-beta
/// ocr: online
/// build: 20260815-040547
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Remote {
    /// 首行的版本号
    pub tke: String,
    /// `build:` 戳。**这个才是每次发布都会变的东西**——版本号只在 bump 时才动，
    /// 只改 skill 文档的发布根本不会让它变，比版本号就永远看不出 skill 过期了
    pub build: String,
}

/// 本地与分发源的差距
#[derive(Debug, Clone, Default)]
pub struct Staleness {
    pub local_tke: String,
    pub remote: Remote,
    /// 本地 skill 的 build 戳（装的时候写进 skill 目录；老版本装的没有这个文件 → None）
    pub local_skill_build: Option<String>,
    /// skill 目录位置（提示里要说清是哪一份）
    pub skill_dir: Option<PathBuf>,
    pub tke_stale: bool,
    pub skill_stale: bool,
}

impl Staleness {
    pub fn any_stale(&self) -> bool {
        self.tke_stale || self.skill_stale
    }

    /// 给人看的一行提醒（没落后就是 None）。
    /// **只说有更新 + 怎么更新**，版本号那些细节留给 `tke doctor`——
    /// 这行是缀在别人正干着的活后面的，不该抢戏，更不该甩一条一百多字符的 curl 命令。
    pub fn hint(&self) -> Option<String> {
        if !self.any_stale() {
            return None;
        }
        let what = match (self.tke_stale, self.skill_stale) {
            (true, true) => "tke 与 skill",
            (true, false) => "tke",
            (false, true) => "skill",
            (false, false) => return None, // 上面已提前返回，这里只是让编译器满意
        };
        Some(format!("{} 有可用更新　更新：tke update", what))
    }
}

fn base_url() -> String {
    std::env::var("TKE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

/// 拉分发源 VERSION（只读几十字节）。带随机参数破 Cloudflare 缓存——
/// 它 max-age 4h 且**不认 no-cache 请求头**，唯一可靠的手段就是变化的查询参数（P-19）
pub fn fetch_remote(timeout_secs: u32) -> Option<Remote> {
    let url = format!(
        "{}/VERSION?t={}",
        base_url(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", &timeout_secs.to_string(), &url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_remote(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_remote(text: &str) -> Option<Remote> {
    let tke = text
        .lines()
        .next()?
        .trim()
        .strip_prefix("tke ")?
        .trim()
        .to_string();
    let build = text
        .lines()
        .find_map(|l| l.strip_prefix("build:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    Some(Remote { tke, build })
}

/// 找已安装的 skill 目录。两处都看：用户级（所有项目通用）与项目级（跟着仓库走）。
/// 项目级优先——它更贴近"当前这个仓库用的是哪一份"。
pub fn skill_dir() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join(".claude/skills/tke-ui-test"));
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        cands.push(PathBuf::from(home).join(".claude/skills/tke-ui-test"));
    }
    cands.into_iter().find(|p| p.join("SKILL.md").is_file())
}

/// 读已安装 skill 的 build 戳。安装器把分发源的 VERSION 原样放进 skill 目录；
/// 更早版本装的没有这个文件 → None（当成"不知道"，不当成过期，免得误报）
pub fn local_skill_build(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("VERSION")).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("build:"))
        .map(|v| v.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn cache_file() -> PathBuf {
    let root = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".tke"))
        .unwrap_or_else(std::env::temp_dir);
    root.join("update-check")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 缓存文件极简：一行 `<检查时刻> <tke版本> <build戳>`。
/// 不上 JSON——这点东西不值得为它引一次序列化，出错也只是少提醒一次
fn read_cache(max_age: u64) -> Option<Remote> {
    let text = std::fs::read_to_string(cache_file()).ok()?;
    let mut it = text.split_whitespace();
    let at: u64 = it.next()?.parse().ok()?;
    if now_secs().saturating_sub(at) > max_age {
        return None;
    }
    Some(Remote {
        tke: it.next().unwrap_or_default().to_string(),
        build: it.next().unwrap_or_default().to_string(),
    })
}

fn write_cache(r: &Remote) {
    let p = cache_file();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&p, format!("{} {} {}", now_secs(), r.tke, r.build));
}

/// 比一下本地与分发源。`max_age=0` 强制联网（`tke doctor` 用），否则优先吃缓存。
/// **联网失败一律返回 None**——离线、内网、分发源挂了都不该妨碍人干活。
pub fn check(max_age: u64) -> Option<Staleness> {
    let remote = match if max_age == 0 { None } else { read_cache(max_age) } {
        Some(r) => r,
        None => {
            // 超时给得很短：这是个锦上添花的检查，不值得让人等
            let r = fetch_remote(if max_age == 0 { 20 } else { 5 })?;
            write_cache(&r);
            r
        }
    };

    let local_tke = env!("BUILD_VERSION").to_string();
    let dir = skill_dir();
    let local_skill_build = dir.as_deref().and_then(local_skill_build);

    // 版本号里带 unknown 的是本地随手编的，别拿它去比（会天天报不一致）
    let tke_stale =
        !remote.tke.is_empty() && local_tke != "unknown" && remote.tke != local_tke;
    // 只有**两边都有 build 戳**才比得了：装了老版本 skill 的没有本地戳，
    // 那种情况不报"过期"（无从判断），但 doctor 里会提示"这份 skill 没有版本信息"
    let skill_stale = match (&local_skill_build, remote.build.is_empty()) {
        (Some(local), false) => local != &remote.build,
        _ => false,
    };

    Some(Staleness {
        local_tke,
        remote,
        local_skill_build,
        skill_dir: dir,
        tke_stale,
        skill_stale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_file() {
        let r = parse_remote("tke 0.7.4-beta\nocr: online\nbuild: 20260815-040547\n").unwrap();
        assert_eq!(r.tke, "0.7.4-beta");
        assert_eq!(r.build, "20260815-040547");
        // 老布局没有 build 戳也不能整个解析失败
        let r2 = parse_remote("tke 0.7.0\n").unwrap();
        assert_eq!(r2.tke, "0.7.0");
        assert!(r2.build.is_empty());
        // 拿到的是 SPA 兜底的 HTML（P-19）→ 不该被当成版本
        assert!(parse_remote("<!DOCTYPE html><html>").is_none());
    }

    /// 提醒要短、且指向 `tke update`——它是缀在别人正干着的活后面的，不该甩一条长 URL
    #[test]
    fn hint_is_short_and_points_at_update_command() {
        let s = Staleness {
            local_tke: "0.7.4-beta".into(),
            remote: Remote { tke: "0.7.5-beta".into(), build: "20260817-010101".into() },
            local_skill_build: Some("20260813-010101".into()),
            skill_dir: None,
            tke_stale: true,
            skill_stale: true,
        };
        let h = s.hint().unwrap();
        assert!(h.contains("tke update"), "要给出更新命令:{}", h);
        assert!(!h.contains("http"), "别在这行甩 URL（细节留给 doctor）:{}", h);
        assert!(h.chars().count() < 40, "这行要短:{}", h);

        // 没落后就不该有提醒
        let ok = Staleness::default();
        assert!(ok.hint().is_none());
    }

    /// 本地 skill 没有版本信息时**不报过期**——无从判断，误报比漏报更烦人
    #[test]
    fn unknown_skill_build_is_not_stale() {
        let s = Staleness {
            local_skill_build: None,
            remote: Remote { tke: "0.7.4".into(), build: "20260817-010101".into() },
            ..Default::default()
        };
        assert!(!s.skill_stale);
        assert!(s.hint().is_none());
    }
}
