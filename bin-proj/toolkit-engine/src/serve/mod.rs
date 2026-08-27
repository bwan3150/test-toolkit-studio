// 【serve】把这台机器的能力开成 HTTP 接口（ADR-0022 P1）。
//
// **语义就是"一台机器的能力被远程调用"**：单节点、单租户、不认识"用户"、不做跨节点分配。
// 池化调度 / 计费 / 多租户在云平台那边（D1）——tke 一旦开始管账号配额，
// 就变成平台的一半，之后每条 INVARIANTS 都要为多租户重新论证。
//
// 分层（每层一个文件，都能单独测）：
//   allowlist  命令白名单 + 参数过滤（INV-16，安全承重墙）
//   lease      租约：设备独占 + 目录隔离 + TTL/心跳 + 复位计划（INV-17）
//   exec       子进程执行 + 参数注入 + 分层计时（D2、Q-17）
//   task       L2 任务层：服务端跑 harness/security + 事件流 + 五态出口 + webhook（D3/D6）
//   routes     端点：hello/health/devices/sessions/exec/artifacts/workspace/tasks
//
// 三层测试（ADR-0008）：① 各文件内 `#[cfg(test)]` 单测（无网无设备）
// ② `tests/serve.rs` 黑盒接口测试（起真二进制、发真 HTTP、跑真子进程，仍不需要设备）
// ③ `tests/e2e/serve-smoke.sh` 真设备 e2e（接口调用真的把浏览器/手机操作了）

pub mod allowlist;
pub mod exec;
pub mod heartbeat;
pub mod link;
pub mod lease;
pub mod routes;
pub mod task;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use lease::{LeaseTable, PoolDevice};

pub struct ServeState {
    /// L2 任务层（ADR-0022 D3）：服务端跑 AI 编排，token 计用户账
    pub tasks: task::TaskTable,
    /// Bearer token；None = 只允许回环地址（见 `run`）
    pub token: Option<String>,
    /// 要 fork 的 tke 自身
    pub bin: PathBuf,
    pub leases: LeaseTable,
    pub default_timeout: Duration,
    pub max_upload_bytes: usize,
}

pub struct ServeOptions {
    pub bind: String,
    pub port: u16,
    pub token: Option<String>,
    pub root: PathBuf,
    pub session_ttl: Duration,
    pub exec_timeout: Duration,
    /// 同时能开几个无头浏览器（`web:1` … `web:N`）
    pub web_slots: u8,
    /// 测试专用：往池里塞 `fake:` 设备（不参与真实调度）
    pub fake_devices: Vec<String>,
    /// 向测试管理平台报到（不配则不报到，节点照常独立可用）
    pub platform: Option<heartbeat::PlatformLink>,
    /// 走**反向通道**：节点主动连平台，平台不再需要够得着节点（ADR-0024）。
    /// 给内网机器用。**与心跳二选一，不自动切换** ——
    /// 自动切换会让"到底走的哪条路"变成运行时才知道的事
    pub link: bool,
    pub max_upload_bytes: usize,
}

/// 组装设备池。浏览器是**槽位**（一个槽一份会话文件，天然可并发），
/// 真机/模拟器来自 `tools::discover`——与 `tke device list` 同一套，
/// 同一个问题不该有两套答案
pub fn build_pool(web_slots: u8, fake_devices: &[String]) -> Vec<PoolDevice> {
    let mut pool: Vec<PoolDevice> = (1..=web_slots.max(1))
        .map(|i| PoolDevice {
            id: format!("web:{i}"),
            kind: "web".into(),
            label: format!("Chrome 无头 #{i}"),
            model: "Chrome 无头".into(),
            os: String::new(),
        })
        .collect();

    for t in crate::tools::discover::discover().targets {
        // "web" / "web --headless=off" 这两行是给人看的用法提示，不是设备 id——
        // 服务器上也不该有有头浏览器（没有显示器）
        if t.kind == "web" || !t.ready {
            continue;
        }
        pool.push(PoolDevice {
            id: t.id,
            kind: t.kind.to_string(),
            label: format!("{} · {}", t.model, t.os),
            model: t.model,
            os: t.os,
        });
    }

    for id in fake_devices {
        pool.push(PoolDevice {
            id: id.clone(), kind: "fake".into(), label: id.clone(),
            model: "fake".into(), os: String::new(),
        });
    }
    pool
}

/// 起服务。**没有 token 就只准绑回环**——一个不设防的端口能操作真机、
/// 还能按白名单跑命令，绑到 0.0.0.0 上等于把这台机器送人
pub async fn run(opts: ServeOptions) -> crate::Result<()> {
    use crate::TkeError;

    let loopback = opts.bind == "127.0.0.1" || opts.bind == "::1" || opts.bind == "localhost";
    if opts.token.is_none() && !loopback {
        return Err(TkeError::InvalidArgument(format!(
            "绑 {} 必须给 --token（或环境变量 TKE_SERVE_TOKEN）：不设防的端口能操作真机。",
            opts.bind
        )));
    }

    let bin = std::env::current_exe()
        .map_err(|e| TkeError::InvalidArgument(format!("取不到 tke 自身路径: {e}")))?;
    std::fs::create_dir_all(&opts.root)?;

    let pool = build_pool(opts.web_slots, &opts.fake_devices);
    let state = Arc::new(ServeState {
        tasks: task::TaskTable::new(),
        token: opts.token.clone(),
        bin,
        leases: LeaseTable::new(opts.root.clone(), pool, opts.session_ttl),
        default_timeout: opts.exec_timeout,
        max_upload_bytes: opts.max_upload_bytes,
    });

    let app = routes::router(state.clone());
    let addr = format!("{}:{}", opts.bind, opts.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| TkeError::NetworkError(format!("绑 {addr} 失败: {e}")))?;
    let local = listener
        .local_addr()
        .map_err(|e| TkeError::NetworkError(format!("取监听地址失败: {e}")))?;

    // 这一行是**契约**：`--port 0` 时调用方（和接口测试）靠它拿到真实端口
    crate::JsonOutput::print(serde_json::json!({
        "success": true,
        "listening": local.to_string(),
        "devices": state.leases.pool().len(),
        "auth": if state.token.is_some() { "bearer" } else { "none (loopback only)" },
    }));

    // 向平台报到（节点主动心跳，见 heartbeat 模块头注释）。
    // **连不上平台不影响节点自己干活**——报不上去只是平台看不见它
    if let Some(pl) = opts.platform.clone() {
        if opts.link {
            // 反向通道：节点主动连上去，此后平台的调用都在这条连接上跑。
            // 连上即注册、断开即离线，不需要 --advertise，也不需要公网入口
            link::spawn(
                state.clone(),
                link::LinkConfig { base: pl.base.clone(), token: pl.token.clone(), name: pl.name.clone() },
                app.clone(),
            );
        } else {
            heartbeat::spawn(state.clone(), pl, local.to_string());
        }
    }

    // 清扫过期租约：断了心跳的会话不能永久占着设备，还要顺手复位（INV-17）
    let sweeper = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            for lease in sweeper.leases.sweep() {
                routes::run_reset(&sweeper, &lease).await;
            }
        }
    });

    axum::serve(listener, app)
        .await
        .map_err(|e| TkeError::NetworkError(format!("服务异常退出: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 浏览器按槽位进池() {
        let pool = build_pool(3, &[]);
        let web: Vec<&str> = pool.iter().filter(|d| d.kind == "web").map(|d| d.id.as_str()).collect();
        assert_eq!(web, vec!["web:1", "web:2", "web:3"], "一个槽一份会话文件，天然可并发");
    }

    #[test]
    fn 假设备只在显式要求时进池() {
        assert!(build_pool(1, &[]).iter().all(|d| d.kind != "fake"));
        let pool = build_pool(1, &["fake:a".to_string()]);
        assert!(pool.iter().any(|d| d.id == "fake:a" && d.platform() == "fake"));
    }

    #[tokio::test]
    async fn 无token时不许绑非回环() {
        let opts = ServeOptions {
            bind: "0.0.0.0".into(),
            port: 0,
            token: None,
            root: std::env::temp_dir().join("tke-serve-test-guard"),
            session_ttl: Duration::from_secs(60),
            exec_timeout: Duration::from_secs(5),
            web_slots: 1,
            fake_devices: vec![],
            platform: None,
            link: false,
            max_upload_bytes: 1024,
        };
        let e = run(opts).await.unwrap_err().to_string();
        assert!(e.contains("--token"), "{e}");
    }
}
