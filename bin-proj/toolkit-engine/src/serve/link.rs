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

/// 平台发过来的一帧。
///
/// 两类：**一问一答的 HTTP 调用**（`type` 缺省），和**长流**（`type` 以 `stream_` 开头）。
/// 长流是为对话式 Agent 加的：`/v1/tasks/{id}/session` 是个 WebSocket，
/// 事件不断推、回答不断写，一问一答那套装不下它。
#[derive(serde::Deserialize)]
struct Call {
    id: String,
    /// 缺省 = HTTP 调用；`stream_open` / `stream_data` / `stream_close` 是长流
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    body: Option<serde_json::Value>,
    /// 二进制正文（平台往工作区放 .tklib 这种）。JSON 装不下字节流
    #[serde(default)]
    b64: Option<String>,
    /// 长流的一帧文本（浏览器写给 tke 进程的回答）
    #[serde(default)]
    text: Option<String>,
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

    // **发送集中到一个协程**。此前是"读一帧 → 等它跑完 → 回一帧"，
    // 于是一条慢命令会把整条连接堵死：心跳发不出去、别的调用排在后面 ——
    // 长流更是直接不可能（它根本不会"跑完"）。
    // 改成谁要发就往 channel 里塞，写端只有这一个所有者。
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            if tx.send(m).await.is_err() {
                break;
            }
        }
        let _ = tx.close().await;
    });

    // 连上先报一次自己是谁、有哪些设备 —— 平台据此建/更新节点行
    out_tx
        .send(Message::Text(hello_frame(st, cfg).to_string().into()))
        .map_err(|e| e.to_string())?;
    let mut last_devices = device_ids(st);
    // 活着的长流：stream_id → 写给本地 WS 的那一头
    let streams: Streams = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

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
                    if out_tx.send(Message::Text(hello_frame(st, cfg).to_string().into())).is_err() {
                        break;
                    }
                }
                if out_tx.send(Message::Ping(Vec::new().into())).is_err() {
                    break;
                }
                continue;
            }
        };
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Ping(p) => {
                if out_tx.send(Message::Pong(p)).is_err() {
                    break;
                }
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

        if call.kind.starts_with("stream_") {
            handle_stream_frame(st, &streams, &out_tx, call).await;
            continue;
        }

        // **每个调用一个协程**：慢命令不再堵住后面的（包括心跳）
        let router = router.clone();
        let token = st.token.clone();
        let out = out_tx.clone();
        tokio::spawn(async move {
            let reply = dispatch(router, &call, token.as_deref()).await;
            let _ = out.send(Message::Text(reply.into()));
        });
    }
    // 读循环结束 = 这条连接没了：把还开着的流一并收掉，别留下没人管的本地 WS
    {
        let mut map = streams.lock().await;
        map.clear();
    }
    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

/// 活着的长流：stream_id → 写给本地 WS 的那一头
type Streams = Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>;

/// 处理一帧长流。
///
/// `stream_open` 时**回拨自己的本地 WS**（`ws://127.0.0.1:<port>/v1/tasks/{id}/session`），
/// 然后两头对拷。为什么不在进程里直接调那个 handler：它要的是一次真实的 WS 升级，
/// 手工造一个等于把 handler 拆成两份实现 —— 双份实现迟早漂移，而且漂移了没人看得出来
/// （这正是这条通道一开始"帧就地拼成 Request 交给已有 Router"的同一条理由）。
/// 这一跳只走回环，不出机器。
async fn handle_stream_frame(
    st: &Arc<ServeState>,
    streams: &Streams,
    out: &tokio::sync::mpsc::UnboundedSender<Message>,
    call: Call,
) {
    match call.kind.as_str() {
        "stream_open" => {
            let id = call.id.clone();
            let url = format!("{}{}", st.local_ws_base, call.path);
            let (to_local_tx, to_local_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            streams.lock().await.insert(id.clone(), to_local_tx);
            let out = out.clone();
            let streams = streams.clone();
            let token = st.token.clone();
            tokio::spawn(async move {
                if let Err(e) = pump_stream(&url, token.as_deref(), &id, to_local_rx, &out).await {
                    // **失败要说出来**：平台那头在等，静默的话浏览器就是一直转圈
                    let _ = out.send(Message::Text(
                        serde_json::json!({"id": id, "type": "stream_error", "error": e}).to_string().into(),
                    ));
                }
                streams.lock().await.remove(&id);
                let _ = out.send(Message::Text(
                    serde_json::json!({"id": id, "type": "stream_close"}).to_string().into(),
                ));
            });
        }
        "stream_data" => {
            if let Some(tx) = streams.lock().await.get(&call.id) {
                let _ = tx.send(call.text.unwrap_or_default());
            }
        }
        "stream_close" => {
            // 丢掉发送端 → pump 那头的 recv 返回 None → 本地 WS 关掉
            streams.lock().await.remove(&call.id);
        }
        other => {
            tracing::warn!(target: "tke::link", "不认识的流帧：{other}");
        }
    }
}

/// 连上本地 WS，两头对拷，直到任一头断开。
async fn pump_stream(
    url: &str,
    token: Option<&str>,
    id: &str,
    mut from_platform: tokio::sync::mpsc::UnboundedReceiver<String>,
    out: &tokio::sync::mpsc::UnboundedSender<Message>,
) -> Result<(), String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut req = url.into_client_request().map_err(|e| e.to_string())?;
    if let Some(t) = token {
        req.headers_mut().insert(
            "Authorization",
            format!("Bearer {t}").parse().map_err(|_| "token 里有非法字符".to_string())?,
        );
    }
    let (stream, _) = tokio_tungstenite::connect_async(req).await.map_err(|e| e.to_string())?;
    let (mut lw, mut lr) = stream.split();

    let _ = out.send(Message::Text(
        serde_json::json!({"id": id, "type": "stream_open_ok"}).to_string().into(),
    ));

    loop {
        tokio::select! {
            m = lr.next() => match m {
                Some(Ok(Message::Text(t))) => {
                    if out.send(Message::Text(
                        serde_json::json!({"id": id, "type": "stream_data", "text": t.to_string()})
                            .to_string().into())).is_err() { break; }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e.to_string()),
            },
            t = from_platform.recv() => match t {
                Some(t) => {
                    if lw.send(Message::Text(t.into())).await.is_err() { break; }
                }
                None => break,   // 平台说这条流结束了
            },
        }
    }
    let _ = lw.close().await;
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
        "tke_version": crate::version_line(),
        "devices": st.leases.pool().iter().map(|d| serde_json::json!({
            "id": d.id, "kind": d.kind, "platform": d.platform(),
            "model": d.model, "os": d.os, "label": d.label, "ready": true, "physical": d.physical(),
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
    fn 长流帧认得出来且不占用path以外的字段() {
        let c: Call = serde_json::from_str(
            r#"{"id":"s1","type":"stream_open","path":"/v1/tasks/t1/session"}"#,
        )
        .unwrap();
        assert_eq!(c.kind, "stream_open");
        assert_eq!(c.path, "/v1/tasks/t1/session");
        assert!(c.text.is_none());

        // 数据帧没有 path —— 老的 Call 里 path 是必填，加流之后必须能缺省，
        // 否则每一帧数据都要带一份没人用的路径
        let d: Call = serde_json::from_str(r#"{"id":"s1","type":"stream_data","text":"hi"}"#).unwrap();
        assert_eq!(d.kind, "stream_data");
        assert_eq!(d.text.as_deref(), Some("hi"));
        assert!(d.path.is_empty());
    }

    #[test]
    fn 普通http帧不会被当成流() {
        let c: Call = serde_json::from_str(r#"{"id":"1","path":"/v1/hello"}"#).unwrap();
        assert!(!c.kind.starts_with("stream_"), "缺 type 的帧必须还是 HTTP 调用");
    }

    #[test]
    fn 帧缺method时按get() {
        let c: Call = serde_json::from_str(r#"{"id":"1","path":"/v1/hello"}"#).unwrap();
        assert_eq!(c.method, "GET");
        assert!(c.body.is_none());
    }
}
