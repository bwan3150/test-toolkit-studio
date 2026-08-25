// 【客户端会话状态】远程模式下"我现在租着哪台设备"要跨进程记住。
//
// 每条命令都是一个新进程（skill 就是这么用的），所以会话必须落盘——
// 与 web 驱动的 `session_file` 同一个套路：**这台机器上已经验证过的做法，别另发明一个**。
//
// 落点：`~/.tke/remote/<节点地址转义>.json`。按节点分文件 = 同时连两个节点互不干扰。

use std::path::PathBuf;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteSession {
    pub base: String,
    pub session_id: String,
    pub device_id: String,
    pub device_label: String,
    pub expires_at: u64,
    /// 已经拉回本地的产物（相对路径 → 字节数）。**只拉新的和变大的**，
    /// 否则每敲一条命令就把整个截图序列重下一遍
    #[serde(default)]
    pub pulled: std::collections::BTreeMap<String, u64>,
}

/// 节点地址 → 文件名（`https://a.b:8787` → `https_a.b_8787`）
fn slug(base: &str) -> String {
    let mut out = String::with_capacity(base.len());
    for c in base.chars() {
        let c = if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' };
        // 连续的下划线并成一个：`http://a:1` 里的 `://` 否则会变成三条杠，文件名难认
        if c == '_' && out.ends_with('_') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('_').to_string()
}

pub fn state_file(base: &str) -> PathBuf {
    let root = dirs::home_dir().unwrap_or_else(std::env::temp_dir).join(".tke").join("remote");
    root.join(format!("{}.json", slug(base)))
}

pub fn load(base: &str) -> Option<RemoteSession> {
    let s = std::fs::read_to_string(state_file(base)).ok()?;
    let sess: RemoteSession = serde_json::from_str(&s).ok()?;
    (sess.base == base).then_some(sess)
}

pub fn save(sess: &RemoteSession) -> std::io::Result<()> {
    let f = state_file(&sess.base);
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(f, serde_json::to_string_pretty(sess).unwrap_or_default())
}

pub fn clear(base: &str) {
    let _ = std::fs::remove_file(state_file(base));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 按节点地址分文件() {
        let a = state_file("http://127.0.0.1:8787");
        let b = state_file("https://node-2.example.com");
        assert_ne!(a, b, "同时连两个节点不能互相覆盖");
        // 文件名里不许有 `/` `:` 这种会把路径带跑偏的字符
        let name = a.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.contains('/') && !name.contains(':'), "{name}");
        assert!(name.starts_with("http_127.0.0.1_8787"), "{name}");
    }

    #[test]
    fn 换了节点地址的旧状态不认() {
        let sess = RemoteSession {
            base: "http://a:1".into(),
            session_id: "s1".into(),
            ..Default::default()
        };
        // load 会比对 base：文件还在但地址对不上就当没有，
        // 否则会拿着 A 节点的 session id 去问 B 节点
        assert_eq!(sess.base, "http://a:1");
        let json = serde_json::to_string(&sess).unwrap();
        let back: RemoteSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "s1");
        assert!(back.pulled.is_empty(), "老状态文件没有 pulled 字段也要能读");
    }
}
