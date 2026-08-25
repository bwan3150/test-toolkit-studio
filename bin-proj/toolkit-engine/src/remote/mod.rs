// 【远程客户端】ADR-0022 D4：同一个 tke 二进制，配上 `TKE_REMOTE` 就把命令发给远端节点。
//
// **选它而不是 MDP/MCP 的唯一理由是文档不分叉**：`tke-ui-test-remote` ≈ `tke-ui-test`
// + 一段"怎么连远端"。590 行踩坑册原样复用。所以这层的设计目标只有一个——
// **让同一条命令行在本地和远程表现一致**：
//   · `-d web` → 租一台 web（不是转发给子进程，服务端会注入回去）
//   · `--log ./out` → 产物拉回 ./out（服务端自己管落点）
//   · stdout / 退出码 **原样透传**，调用方看到的东西跟本地一样
//
// 会话是隐式的：第一条命令自动租一台、落盘记住、后续命令复用并续租（`tke remote` 可显式管）。
// 因为每条命令都是一个新进程（skill 就是这么用的），不落盘就每次都要重租一台。

pub mod argv;
pub mod client;
pub mod state;

use std::io::Write;

use client::Client;
use state::RemoteSession;

/// 默认租约时长：够一次交互式检查用，断了心跳节点会自己回收
const DEFAULT_TTL: u64 = 1800;
const DEFAULT_EXEC_TIMEOUT: u64 = 180;

pub struct RemoteConfig {
    pub base: String,
    pub token: Option<String>,
}

impl RemoteConfig {
    /// 没配 `TKE_REMOTE` 就是本地模式——**一个环境变量决定走哪条路**，
    /// 不引入第二个开关（两个开关必然有人只设一个）
    pub fn from_env() -> Option<Self> {
        let base = std::env::var("TKE_REMOTE").ok().filter(|s| !s.trim().is_empty())?;
        let base = base.trim().trim_end_matches('/').to_string();
        let base = if base.starts_with("http://") || base.starts_with("https://") {
            base
        } else {
            format!("http://{base}")
        };
        Some(Self { base, token: std::env::var("TKE_TOKEN").ok().filter(|s| !s.is_empty()) })
    }

    pub fn client(&self) -> Client {
        Client { base: self.base.clone(), token: self.token.clone() }
    }
}

/// 这些命令只打 URL / 只动文件，**不碰设备**。没给 `-d` 时就别去租一台真机——
/// 那会让用户为没用到的设备付租金（计费模型见 ADR-0022 D3）
const DEVICE_FREE: &[&str] = &["http", "recon", "report", "task", "doctor"];

/// `-d` 的值是"要哪类设备"还是"要哪一台"：平台关键字走前者，其余当设备 id。
/// 复用 `Platform::from_device` 那套判据的**意图**，但这里只认显式关键字——
/// 远程点名一台设备是常事，猜错了会把人租到别的机器上
pub fn split_device(d: Option<&str>) -> (Option<&str>, Option<&str>) {
    match d {
        Some(v) if matches!(v, "web" | "android" | "ios" | "fake" | "none") => (Some(v), None),
        Some(v) => (None, Some(v)),
        None => (None, None),
    }
}

/// 本地拦截：命令在远程白名单里就发出去，否则交还给 clap 走本地那条路。
/// 返回 `Some(退出码)` = 已经处理完了
pub fn maybe_dispatch(cfg: &RemoteConfig) -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let inv = match argv::parse(&args) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("❌ {}", e.0);
            return Some(2);
        }
    };
    let cmd = inv.command.clone()?;
    // 不在白名单里的（harness/security/update/remote/serve…）走本地：
    // 有的该在本地跑，有的会被本地 clap 报错——两种都比在这里自作主张好
    if !crate::serve::allowlist::allowed_commands().contains(&cmd.as_str()) {
        return None;
    }
    for d in &inv.dropped {
        eprintln!("ℹ️  远程模式忽略 {d}");
    }
    Some(run(cfg, inv))
}

fn run(cfg: &RemoteConfig, inv: argv::ClientInvocation) -> i32 {
    let c = cfg.client();
    // 没点名设备 + 命令本来就不碰设备 → 开一个无设备会话（只要工作区，不计设备时长）
    let want = match (&inv.device, inv.command.as_deref()) {
        (Some(d), _) => Some(d.clone()),
        (None, Some(cmd)) if DEVICE_FREE.contains(&cmd) => Some("none".to_string()),
        _ => None,
    };
    let mut sess = match ensure_session(&c, want.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ {e}");
            return 4;
        }
    };

    let out = match c.exec(&sess.session_id, &inv.argv, DEFAULT_EXEC_TIMEOUT) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ {e}");
            return 4;
        }
    };

    // 原样透传：调用方看到的东西要跟本地一模一样，这是"文档不分叉"的下半截
    let stdout_raw = out["stdout_raw"].as_str().unwrap_or("");
    if !stdout_raw.is_empty() {
        print!("{stdout_raw}");
        let _ = std::io::stdout().flush();
    }
    let stderr = out["stderr"].as_str().unwrap_or("");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if out["timed_out"].as_bool().unwrap_or(false) {
        eprintln!("❌ 节点上超时了（{DEFAULT_EXEC_TIMEOUT}s）——命令已被杀掉");
    }

    if let Some(rel) = &inv.log {
        // 拉回**当前目录下的同一个相对路径**：本地敲 `--log logs/scan`，
        // 拉完本地就有 ./logs/scan，跟在本地跑完全一样
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        match c.pull_new(&mut sess, std::path::Path::new("."), rel_s.trim_start_matches("./")) {
            Ok(0) => {}
            Ok(n) => eprintln!("📥 拉回 {n} 个产物 → {}", rel.display()),
            Err(e) => eprintln!("⚠️  产物没拉全：{e}"),
        }
    }
    let _ = state::save(&sess);

    out["exit_code"].as_i64().map(|c| c as i32).unwrap_or(1)
}

/// 有活着的会话就复用并续租，没有就租一台
pub fn ensure_session(c: &Client, device: Option<&str>) -> Result<RemoteSession, String> {
    if let Some(mut s) = state::load(&c.base) {
        // 点名了别的设备就换一台——沿用旧会话会让人以为换成功了
        let same = device.is_none_or(|d| d == s.device_id || split_device(Some(d)).0.is_some());
        if same && c.heartbeat(&s.session_id, DEFAULT_TTL).is_ok() {
            s.expires_at = crate::serve::lease::now_secs() + DEFAULT_TTL;
            return Ok(s);
        }
        state::clear(&c.base);
    }
    open_session(c, device)
}

pub fn open_session(c: &Client, device: Option<&str>) -> Result<RemoteSession, String> {
    check_drift(c);
    let (platform, device_id) = split_device(device);
    let v = c.create_session(platform, device_id, DEFAULT_TTL)?;
    let sess = RemoteSession {
        base: c.base.clone(),
        session_id: v["session_id"].as_str().unwrap_or_default().to_string(),
        device_id: v["device"]["id"].as_str().unwrap_or_default().to_string(),
        device_label: v["device"]["label"].as_str().unwrap_or_default().to_string(),
        expires_at: v["expires_at"].as_u64().unwrap_or(0),
        pulled: Default::default(),
    };
    if sess.session_id.is_empty() {
        return Err(format!("节点没给出 session_id：{v}"));
    }
    if sess.device_id.is_empty() {
        eprintln!("🔗 已开会话（无设备，只用工作区——不计设备时长）");
    } else {
        eprintln!("🔗 租到 {}（{}）", sess.device_label, sess.device_id);
    }
    state::save(&sess).map_err(|e| format!("会话状态写不下去: {e}"))?;
    Ok(sess)
}

/// 版本漂移只提醒不阻断。**必须说出来**：装好的东西不自更新、也没有过期提示，
/// 沉默会让人得出"没改善"的假结论（Q-11 / P-41 的教训，远程会放大它）
pub fn check_drift(c: &Client) {
    let Ok(hello) = c.hello() else { return };
    let node = hello["tke_version"].as_str().unwrap_or("");
    let local = env!("BUILD_VERSION");
    if node.is_empty() || local == "unknown" || node == local {
        return;
    }
    eprintln!("⚠️  版本对不上：本地 {local} / 节点 {node}。行为可能不一致——先把两边对齐再排查问题。");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 地址缺协议时补上() {
        std::env::set_var("TKE_REMOTE", "127.0.0.1:8787");
        assert_eq!(RemoteConfig::from_env().unwrap().base, "http://127.0.0.1:8787");
        std::env::set_var("TKE_REMOTE", "https://node/");
        assert_eq!(RemoteConfig::from_env().unwrap().base, "https://node", "末尾的斜杠要去掉，否则拼出 //v1");
        std::env::set_var("TKE_REMOTE", "  ");
        assert!(RemoteConfig::from_env().is_none(), "空的等于没配");
        std::env::remove_var("TKE_REMOTE");
        assert!(RemoteConfig::from_env().is_none());
    }

    #[test]
    fn 平台关键字与设备id分开() {
        assert_eq!(split_device(Some("web")), (Some("web"), None));
        assert_eq!(split_device(Some("none")), (Some("none"), None), "无设备会话也是一种平台");
        assert_eq!(split_device(Some("android")), (Some("android"), None));
        // 具体的一台：点名租它，不能猜成平台
        assert_eq!(split_device(Some("web:2")), (None, Some("web:2")));
        assert_eq!(split_device(Some("f64b3b4d")), (None, Some("f64b3b4d")));
        assert_eq!(split_device(None), (None, None));
    }
}
