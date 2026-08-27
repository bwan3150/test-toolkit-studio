//! 【反向通道】节点主动连平台的长连接（ADR-0024）。
//!
//! 为什么要有它：任务通道原本是平台 → 节点的 HTTP，节点必须让平台够得着。
//! 真机大多插在内网机器上（IP 会变、没有公网入口），于是要么拉隧道、要么拉 VPN。
//! 反过来之后，**节点只出不进**：连上即注册，断开即离线。
//!
//! 关键设计：**在这条连接上跑 HTTP 语义，帧就地拼成 Request 交给已有的 Router**。
//! 七个 handler 一个字都不用改，也不会出现"HTTP 走一套、WS 走另一套"的双份实现 ——
//! 那种双份实现迟早会漂移，而且漂移了没人看得出来。

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

use super::ServeState;

/// 平台发过来的一帧：一个 HTTP 调用。
#[derive(serde::Deserialize)]
struct Call {
    id: String,
    #[serde(default = "default_method")]
    method: String,
    path: String,
    #[serde(default)]
    body: Option<serde_json::Value>,
    /// 二进制正文（平台往工作区放 .tklib 这种）。JSON 装不下字节流
    #[serde(default)]
    b64: Option<String>,
}
fn default_method() -> String {
    "GET".into()
}

/// 回给平台的一帧。
#[derive(serde::Serialize)]
struct Reply<'a> {
    id: &'a str,
    status: u16,
    /// 正文。文本类原样放这里；二进制产物走 `b64`
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
    /// base64 正文 —— 产物是图片/zip，塞不进 JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    b64: Option<String>,
}

/// 连接配置。
#[derive(Clone)]
pub struct LinkConfig {
    /// 平台地址（http/https 都行，内部换成 ws/wss）
    pub base: String,
    /// 节点报到用的凭据
    pub token: String,
    pub name: String,
}

/// 起反向通道（后台任务）。
///
/// **连不上不影响节点自己干活**：本地 HTTP 口照常听着，只是平台暂时看不见它。
/// 所以这里只退避重试、只记日志，绝不退出进程。
pub fn spawn(st: Arc<ServeState>, cfg: LinkConfig, router: Router) {
    install_crypto_provider();
    tokio::spawn(async move {
        let mut failures = 0u32;
        loop {
            match connect_once(&st, &cfg, router.clone()).await {
                Ok(()) => {
                    tracing::info!(target: "tke::link", "与平台的连接已关闭，准备重连");
                    failures = 0;
                }
                Err(e) => {
                    failures += 1;
                    // 只在第一次和每 20 次失败时喊一声 —— 平台维护一小时的话，
                    // 每 5 秒一条 WARN 会把日志淹掉，而信息量只有第一条
                    if failures == 1 || failures % 20 == 0 {
                        tracing::warn!(target: "tke::link", "连平台失败（第 {failures} 次）：{e}");
                    }
                }
            }
            let backoff = (5 * failures.min(12) as u64).max(1).min(60);
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
        }
    });
}

/// 装 rustls 的加密后端。
///
/// 依赖树里 ring 和 aws-lc-rs 同时存在（别的 crate 各拉了一个），rustls 0.23 遇到
/// 多个候选**不猜、直接 panic**：
///   「Could not automatically determine the process-level CryptoProvider」
///
/// 而且只在真的建 TLS 连接时才炸 —— 平台是 http:// 的话一路正常，
/// 换成 https:// 立刻挂（实测：本地测 http 全绿，用户连线上 https 当场 panic）。
///
/// 已经装过就跳过（install_default 第二次会返回 Err，不是错误）
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn ws_url(base: &str) -> String {
    let b = base.trim_end_matches('/');
    let b = if let Some(rest) = b.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = b.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        format!("ws://{b}")
    };
    format!("{b}/api/v1/node/link")
}

async fn connect_once(st: &Arc<ServeState>, cfg: &LinkConfig, router: Router) -> Result<(), String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let url = ws_url(&cfg.base);
    let mut req = url.as_str().into_client_request().map_err(|e| e.to_string())?;
    req.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", cfg.token).parse().map_err(|_| "token 里有非法字符".to_string())?,
    );
    let (stream, _) = tokio_tungstenite::connect_async(req).await.map_err(|e| e.to_string())?;
    let (mut tx, mut rx) = stream.split();
    tracing::info!(target: "tke::link", "已连上平台（{}）", cfg.base);

    // 连上先报一次自己是谁、有哪些设备 —— 平台据此建/更新节点行
    tx.send(Message::Text(hello_frame(st, cfg).to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let mut last_devices = device_ids(st);

    // 节点这头也定期报一次活。**两边都不说话的连接会被中间层悄悄切掉**，
    // 而谁都不知道 —— 表现就是"连上又断、每秒重连一次"（实测撞过）。
    // 只靠平台 ping 也行，但那样保活就依赖对端的实现，这里自己也发一份更稳
    let (ping_tx, mut ping_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(25)).await;
            if ping_tx.send(()).is_err() {
                return;
            }
        }
    });

    loop {
        let msg = tokio::select! {
            m = rx.next() => match m {
                Some(m) => m.map_err(|e| e.to_string())?,
                None => break,
            },
            _ = ping_rx.recv() => {
                // **设备变了就重报**：起了一台模拟器、插了根线，平台得知道。
                // hello 是全量替换，重发一次就对上了（少发一次不会永久错位）
                let now = device_ids(st);
                if now != last_devices {
                    last_devices = now;
                    if tx.send(Message::Text(hello_frame(st, cfg).to_string().into())).await.is_err() {
                        break;
                    }
                }
                if tx.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
                continue;
            }
        };
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Ping(p) => {
                tx.send(Message::Pong(p)).await.map_err(|e| e.to_string())?;
                continue;
            }
            Message::Close(_) => break,
            _ => continue,
        };
        let call: Call = match serde_json::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(target: "tke::link", "看不懂的帧：{e}");
                continue;
            }
        };
        let reply = dispatch(router.clone(), &call, st.token.as_deref()).await;
        if tx.send(Message::Text(reply.into())).await.is_err() {
            break;
        }
    }
    Ok(())
}

/// 「我是谁、有哪些设备」。**每次都是全量**，平台整份替换 ——
/// 事件式（插了一台推一条）漏一条就永久错位，全量替换会自愈
fn hello_frame(st: &Arc<ServeState>, cfg: &LinkConfig) -> serde_json::Value {
    serde_json::json!({
        "event": "hello",
        "name": cfg.name,
        "host_os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "tke_version": env!("BUILD_VERSION"),
        "devices": st.leases.pool().iter().map(|d| serde_json::json!({
            "id": d.id, "kind": d.kind, "platform": d.platform(),
            "model": d.model, "os": d.os, "label": d.label, "ready": true,
        })).collect::<Vec<_>>(),
    })
}

fn device_ids(st: &Arc<ServeState>) -> Vec<String> {
    st.leases.pool().iter().map(|d| d.id.clone()).collect()
}

/// 把一帧翻成 Request 交给 Router，再把 Response 翻回一帧。
async fn dispatch(router: Router, call: &Call, local_token: Option<&str>) -> String {
    // 二进制优先：给了 b64 就是一份字节流（放文件），别再按 JSON 处理
    let (body, ctype) = match (&call.b64, &call.body) {
        (Some(b), _) => {
            use base64::Engine;
            match base64::engine::general_purpose::STANDARD.decode(b) {
                Ok(bytes) => (Body::from(bytes), "application/octet-stream"),
                Err(e) => {
                    return serde_json::to_string(&Reply { id: &call.id, status: 400,
                        body: Some(serde_json::json!({"error": format!("b64 解不开：{e}")})), b64: None })
                        .unwrap_or_default();
                }
            }
        }
        (None, Some(v)) => (Body::from(v.to_string()), "application/json"),
        (None, None) => (Body::empty(), "application/json"),
    };
    // 走这条路的请求**已经被平台认过了**（建连时验的 token），但 Router 上挂着
    // auth 中间件，这里**不绕过它** —— 绕过等于给自己开一个不走鉴权的入口，
    // 以后任何一处改动都可能把它变成真的后门。照常带上节点自己的 token
    let mut b = Request::builder()
        .method(call.method.as_str())
        .uri(&call.path)
        .header("content-type", ctype);
    if let Some(t) = local_token {
        b = b.header("Authorization", format!("Bearer {t}"));
    }
    let req = b.body(body);
    let Ok(req) = req else {
        return serde_json::to_string(&Reply { id: &call.id, status: 400, body: Some(serde_json::json!({"error":"请求拼不出来"})), b64: None }).unwrap_or_default();
    };
    let resp = match router.oneshot(req).await {
        Ok(r) => r,
        Err(e) => {
            return serde_json::to_string(&Reply { id: &call.id, status: 500, body: Some(serde_json::json!({"error": e.to_string()})), b64: None }).unwrap_or_default();
        }
    };
    let status = resp.status().as_u16();
    let is_json = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("json"))
        .unwrap_or(false);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap_or_default();

    let reply = if is_json {
        Reply {
            id: &call.id,
            status,
            body: serde_json::from_slice(&bytes).ok(),
            b64: None,
        }
    } else {
        // 产物是图片 / zip / html —— 塞不进 JSON，走 base64。
        // **别在这里猜编码**：按 content-type 判，判不出就当二进制
        use base64::Engine;
        Reply {
            id: &call.id,
            status,
            body: None,
            b64: Some(base64::engine::general_purpose::STANDARD.encode(&bytes)),
        }
    };
    serde_json::to_string(&reply).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 地址换成ws协议() {
        assert_eq!(ws_url("https://p.example"), "wss://p.example/api/v1/node/link");
        assert_eq!(ws_url("http://127.0.0.1:7777/"), "ws://127.0.0.1:7777/api/v1/node/link");
        // 没写协议的按 ws 处理，别拼出个 `ws://https://…`
        assert_eq!(ws_url("p.example"), "ws://p.example/api/v1/node/link");
    }

    #[test]
    fn 帧缺method时按get() {
        let c: Call = serde_json::from_str(r#"{"id":"1","path":"/v1/hello"}"#).unwrap();
        assert_eq!(c.method, "GET");
        assert!(c.body.is_none());
    }
}
