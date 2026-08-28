// 【端点】P1 的 HTTP 面：hello / health / devices / sessions / exec / artifacts / workspace。
//
// 这一层只做"翻译 + 编排"，判断逻辑都在 allowlist / lease / exec 里——
// 跟 `cli/` 的定位一样（INV-10：参数翻译层禁止业务逻辑）。
//
// 鉴权走中间件而不是每个 handler 自己查：**忘记查**是这类代码最典型的洞，
// 中间件让人没有忘记的机会。

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde_json::json;

use super::exec::ExecRequest;
use super::task::SpawnTask;
use super::lease::{AcquireError, Lease};
use super::{allowlist, exec, ServeState};

type St = State<Arc<ServeState>>;

/// 统一错误出口：**HTTP 状态码要能区分"你写错了"和"这儿没有"**，
/// 否则调用方只能靠读错误文本猜该重试还是该改参数
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}
fn not_found(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, msg.into())
}

pub fn router(state: Arc<ServeState>) -> Router {
    let upload_limit = state.max_upload_bytes;
    Router::new()
        .route("/v1/hello", get(hello))
        .route("/v1/health", get(health))
        .route("/v1/devices", get(devices))
        .route("/v1/sessions", post(session_create).get(session_list))
        .route("/v1/sessions/{sid}", get(session_get))
        .route("/v1/sessions/{sid}", delete(session_delete))
        .route("/v1/sessions/{sid}/heartbeat", post(session_heartbeat))
        .route("/v1/sessions/{sid}/exec", post(session_exec))
        // 「现在屏幕长什么样」—— 一张**没有标注**的原图。
        // 平台的云设备页要它:steps 落的截图带着 tke 报告用的顶部横幅
        // （"Step 2 OK | 等待 [1ms]"），糊在实况屏幕上没人看得懂
        .route("/v1/sessions/{sid}/screen", get(session_screen))
        // 能跑哪些命令 —— 给调用方做输入联想。**白名单仍然只有这一份**(INV-16),
        // 这里只是把它读出来,不是让平台去维护第二份
        .route("/v1/commands", get(commands))
        // 不带路径 = 整个工作区（拉产物时最常用的那一次，别让人非得先知道有哪些目录）
        .route("/v1/sessions/{sid}/artifacts", get(artifact_root))
        .route("/v1/sessions/{sid}/artifacts/{*path}", get(artifact_get))
        .route(
            "/v1/sessions/{sid}/workspace/{*path}",
            put(workspace_put).layer(DefaultBodyLimit::max(upload_limit)),
        )
        // ===== L2 任务层（ADR-0022 D3）=====
        .route("/v1/tasks", post(task_create).get(task_list))
        .route("/v1/tasks/{id}", get(task_get))
        .route("/v1/tasks/{id}/events", get(task_events))
        .route("/v1/tasks/{id}/session", get(task_ws))
        .route("/v1/tasks/{id}/report", get(task_report))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}

/// Bearer 校验。没配 token 的节点只绑回环（见 `serve::run`），这里就直接放行
async fn auth(State(st): St, req: Request, next: Next) -> Result<Response, ApiError> {
    let Some(expect) = st.token.as_deref() else {
        return Ok(next.run(req).await);
    };
    let got = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    // 长度先比：不同长度直接判否，省得下面的逐字节比较泄露长度以外的信息
    if got.len() == expect.len() && got.bytes().zip(expect.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0
    {
        Ok(next.run(req).await)
    } else {
        Err(ApiError(StatusCode::UNAUTHORIZED, "凭据无效：请带 `Authorization: Bearer <token>`。".into()))
    }
}

// ===================== 元信息 =====================

/// 版本握手。**比 build 戳**（沿用 ADR-0014 的判据）——client/node 不一致要能立刻看出来，
/// 沉默会让人得出"没改善"的假结论（Q-11 / P-41 的教训，远程会放大它）
async fn hello(State(st): St) -> Json<serde_json::Value> {
    Json(json!({
        "api_version": "v1",
        "tke_version": crate::version_line(),
        "host_os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "devices": st.leases.pool().len(),
        "allowed_commands": allowlist::allowed_commands(),
    }))
}

/// 节点体检 = `tke doctor --json`，不新造一套判据
async fn health(State(st): St) -> Result<Json<serde_json::Value>, ApiError> {
    let dirs = super::lease::SessionDirs {
        root: std::env::temp_dir(),
        workspace: std::env::temp_dir(),
        logs: std::env::temp_dir(),
        cache: std::env::temp_dir(),
    };
    let validated = allowlist::validate(&["doctor".to_string()]).map_err(|e| bad(e.0))?;
    let req = ExecRequest { validated, dirs, device: None, timeout: Duration::from_secs(60) };
    let out = exec::run(&st.bin, &req).await.map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({
        "ok": out.exit_code == Some(0),
        "doctor": out.stdout_json.or_else(|| Some(json!(out.stdout_raw))),
    })))
}

async fn devices(State(st): St) -> Json<serde_json::Value> {
    let pool = st.leases.pool();
    let rows: Vec<serde_json::Value> = pool
        .iter()
        .map(|d| {
            let holder = st.leases.holder_of(&d.id);
            json!({
                "id": d.id, "kind": d.kind, "platform": d.platform(), "label": d.label,
                // "被谁租着"要给出来：平台调度靠它判断这台还能不能派
                "leased_by": holder,
                "available": holder.is_none(),
            })
        })
        .collect();
    Json(json!({"devices": rows}))
}

// ===================== 租约 =====================

#[derive(serde::Deserialize, Default)]
struct Caps {
    platform: Option<String>,
    device_id: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct CreateSession {
    #[serde(default)]
    capabilities: Caps,
    ttl_s: Option<u64>,
    /// 原样带回的归账标签。设备租赁与 AI 计费用**同一条**透传路——
    /// tke 不认识"用户"，归账靠调用方自己带的这张纸条（ADR-0022 D1）
    meta: Option<serde_json::Value>,
}

fn lease_view(l: &Lease) -> serde_json::Value {
    json!({
        "session_id": l.id,
        "device": {"id": l.device.id, "kind": l.device.kind, "platform": l.device.platform(), "label": l.device.label},
        "created_at": l.created_at,
        "expires_at": l.expires_at,
        "workspace": format!("/v1/sessions/{}/workspace", l.id),
        "artifacts": format!("/v1/sessions/{}/artifacts", l.id),
        "launched_apps": l.launched_apps,
        "meta": l.meta,
    })
}

async fn session_create(
    State(st): St,
    body: Option<Json<CreateSession>>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let ttl = req.ttl_s.map(Duration::from_secs);
    match st.leases.acquire(req.capabilities.platform.as_deref(), req.capabilities.device_id.as_deref(), ttl) {
        Ok(mut l) => {
            st.leases.set_meta(&l.id, req.meta.clone());
            l.meta = req.meta;
            Ok((StatusCode::CREATED, Json(lease_view(&l))))
        }
        // 409 而不是 404：设备存在、只是被占着，调用方该等而不是换节点
        Err(e @ AcquireError::AllBusy(_)) => Err(ApiError(StatusCode::CONFLICT, e.to_string())),
        Err(e) => Err(not_found(e.to_string())),
    }
}

async fn session_list(State(st): St) -> Json<serde_json::Value> {
    Json(json!({"sessions": st.leases.active().iter().map(lease_view).collect::<Vec<_>>()}))
}

fn need_lease(st: &ServeState, sid: &str) -> Result<Lease, ApiError> {
    st.leases
        .get(sid)
        .ok_or_else(|| not_found(format!("会话 {sid} 不存在或已过期（TTL 到了 / 心跳断了）。")))
}

async fn session_get(State(st): St, Path(sid): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(lease_view(&need_lease(&st, &sid)?)))
}

#[derive(serde::Deserialize, Default)]
struct Heartbeat {
    ttl_s: Option<u64>,
}

async fn session_heartbeat(
    State(st): St,
    Path(sid): Path<String>,
    body: Option<Json<Heartbeat>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let ttl = body.and_then(|Json(b)| b.ttl_s).map(Duration::from_secs);
    match st.leases.heartbeat(&sid, ttl) {
        Some(exp) => Ok(Json(json!({"session_id": sid, "expires_at": exp}))),
        None => Err(not_found(format!("会话 {sid} 不存在或已过期。"))),
    }
}

/// 释放 = 摘租约 + **复位设备**（INV-17）。复位结果如实回报，包括耗时——
/// Q-19 还没定"复位要做到什么程度"，先把事实量出来
async fn session_delete(State(st): St, Path(sid): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    let lease = st.leases.take(&sid).ok_or_else(|| not_found(format!("会话 {sid} 不存在。")))?;
    let reset = run_reset(&st, &lease).await;
    Ok(Json(json!({"session_id": sid, "released": true, "reset": reset})))
}

/// 执行复位计划。**尽力而为但如实回报**：某一条失败不阻断后面的，
/// 但不许静默（INV-9：失败必须可见）
pub async fn run_reset(st: &ServeState, lease: &Lease) -> serde_json::Value {
    let started = std::time::Instant::now();
    let mut done = Vec::new();
    for argv in lease.reset_plan().actions {
        let line = argv.join(" ");
        let Ok(validated) = allowlist::validate(&argv) else {
            done.push(json!({"cmd": line, "ok": false, "error": "复位命令没过白名单（这是 bug）"}));
            continue;
        };
        let req = ExecRequest {
            validated,
            dirs: lease.dirs.clone(),
            device: Some(lease.device.id.clone()),
            timeout: Duration::from_secs(30),
        };
        match exec::run(&st.bin, &req).await {
            Ok(o) => done.push(json!({"cmd": line, "ok": o.exit_code == Some(0), "error": o.stderr})),
            Err(e) => done.push(json!({"cmd": line, "ok": false, "error": e})),
        }
    }
    json!({"actions": done, "elapsed_ms": started.elapsed().as_millis() as u64})
}

// ===================== 命令执行（L1） =====================

#[derive(serde::Deserialize)]
struct ExecBody {
    argv: Vec<String>,
    timeout_s: Option<u64>,
}

async fn session_exec(
    State(st): St,
    Path(sid): Path<String>,
    Json(body): Json<ExecBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let lease = need_lease(&st, &sid)?;
    // 白名单在最前面：没过关的东西连子进程都不该起（INV-16）
    let validated = allowlist::validate(&body.argv).map_err(|e| bad(e.0))?;
    let timeout = body
        .timeout_s
        .map(Duration::from_secs)
        .unwrap_or(st.default_timeout);

    let req = ExecRequest {
        validated,
        dirs: lease.dirs.clone(),
        // 无设备会话不注入 `-d`：注入一个空的设备 id 会让下游按默认安卓设备去连
        device: (!lease.device.id.is_empty()).then(|| lease.device.id.clone()),
        timeout,
    };
    let out = exec::run(&st.bin, &req).await.map_err(bad)?;
    // 启动过的 App 记下来，释放时要停掉——依据来自事实（argv），不靠调用方申报
    st.leases.note_launch(&sid, &body.argv);

    Ok(Json(serde_json::to_value(&out).unwrap_or_else(|e| json!({"error": e.to_string()}))))
}

/// 能跑哪些命令。不需要租约 —— 它跟某台设备无关,是这个节点的能力清单。
async fn commands() -> Json<serde_json::Value> {
    let list: Vec<serde_json::Value> = allowlist::command_catalog()
        .into_iter()
        .map(|(name, subs)| json!({ "name": name, "subs": subs }))
        .collect();
    Json(json!({ "commands": list }))
}

/// 当前屏幕的原图。
///
/// 先 `refresh` 采一次，再把设备缓存区里的那张 PNG 原样回去。
///
/// **为什么不让调用方自己 `exec refresh` 然后去产物里取**：`refresh` 写的是
/// 设备缓存区（`--cache` 下），而产物接口只服务会话**工作区**，两者是兄弟目录。
/// 调用方够不着它 —— 这正是平台此前只能绕道 `steps` 的原因，而 steps 的截图
/// 是给报告用的、带标注横幅。
async fn session_screen(State(st): St, Path(sid): Path<String>) -> Result<Response, ApiError> {
    let lease = need_lease(&st, &sid)?;
    let validated = allowlist::validate(&["refresh".to_string()]).map_err(|e| bad(e.0))?;
    let req = ExecRequest {
        validated,
        dirs: lease.dirs.clone(),
        device: (!lease.device.id.is_empty()).then(|| lease.device.id.clone()),
        timeout: st.default_timeout,
    };
    exec::run(&st.bin, &req).await.map_err(bad)?;

    // 落点与 Workarea::for_device 同一套算法。**这里不另写一份路径拼装**——
    // 两份拼装迟早不一致，而不一致的表现是"截图突然取不到了"，查起来很贵
    let wa = crate::utils::Workarea::for_device_under(&lease.dirs.cache, Some(&lease.device.id))
        .map_err(|e| bad(e.to_string()))?;
    let path = wa.screenshot_path();
    let bytes = std::fs::read(&path).map_err(|e| {
        not_found(format!("这台设备上还没有可截的画面（{}）", e))
    })?;
    Ok(([(axum::http::header::CONTENT_TYPE, "image/png")], bytes).into_response())
}

// ===================== 产物 / 工作区 =====================

#[derive(serde::Deserialize)]
struct ListQuery {
    #[serde(default)]
    list: bool,
}

/// 列整个工作区（`/artifacts` 不带路径）
async fn artifact_root(
    State(st): St,
    Path(sid): Path<String>,
) -> Result<Response, ApiError> {
    let lease = need_lease(&st, &sid)?;
    let mut names = Vec::new();
    collect(&lease.dirs.workspace, &lease.dirs.workspace, &mut names);
    names.sort();
    Ok(Json(json!({"files": names})).into_response())
}

/// 下载产物。路径一律过工作区沙箱——`artifacts/../../../etc/passwd` 就是在这儿挡住的
async fn artifact_get(
    State(st): St,
    Path((sid, rel)): Path<(String, String)>,
    Query(q): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let lease = need_lease(&st, &sid)?;
    let path = crate::utils::resolve_in_workspace(&lease.dirs.workspace, &rel).map_err(bad)?;
    if q.list || path.is_dir() {
        let mut names = Vec::new();
        collect(&path, &lease.dirs.workspace, &mut names);
        names.sort();
        return Ok(Json(json!({"files": names})).into_response());
    }
    let bytes = std::fs::read(&path).map_err(|e| not_found(format!("{rel}: {e}")))?;
    let ct = content_type(&rel);
    Ok(([(axum::http::header::CONTENT_TYPE, ct)], bytes).into_response())
}

fn collect(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, root, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// 上传到工作区（APK/IPA、`.tks`+`.tklib` 两件套）。同样过沙箱
async fn workspace_put(
    State(st): St,
    Path((sid, rel)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let lease = need_lease(&st, &sid)?;
    let path = crate::utils::resolve_in_workspace(&lease.dirs.workspace, &rel).map_err(bad)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| bad(e.to_string()))?;
    }
    std::fs::write(&path, &body).map_err(|e| bad(e.to_string()))?;
    Ok(Json(json!({"path": rel, "bytes": body.len()})))
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        "html" | "htm" => "text/html; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "txt" | "md" | "tks" | "log" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 产物按扩展名给类型() {
        assert_eq!(content_type("screenshots/step_001.png"), "image/png");
        assert_eq!(content_type("log.json"), "application/json");
        // 认不出来的别猜成 text（浏览器会当页面渲染），下载就好
        assert_eq!(content_type("x.bin"), "application/octet-stream");
    }

    #[test]
    fn 产物列表给的是相对路径() {
        let tmp = std::env::temp_dir().join(format!("tke-routes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("logs/screenshots")).unwrap();
        std::fs::write(tmp.join("logs/log.json"), "{}").unwrap();
        std::fs::write(tmp.join("logs/screenshots/a.png"), "x").unwrap();
        let mut got = Vec::new();
        collect(&tmp, &tmp, &mut got);
        got.sort();
        assert_eq!(got, vec!["logs/log.json", "logs/screenshots/a.png"]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

// ===================== L2 任务层 =====================

#[derive(serde::Deserialize)]
struct CreateTask {
    kind: String,
    /// 要哪一类设备：web / android / ios（不给按 android）
    platform: Option<String>,
    /// 点名某一台，优先于 platform
    device_id: Option<String>,
    target: Option<String>,
    testcase: Option<String>,
    mode: Option<String>,
    /// true = 有人（或平台 UI）会通过 WebSocket 回答问题；
    /// false（默认）= headless，遇到决策点立刻 `needs_decision` 回传（D6）
    #[serde(default)]
    interactive: bool,
    max_rounds: Option<u32>,
    timeout_s: Option<u64>,
    callback_url: Option<String>,
    /// 调用方的 AI 凭据（平台把 App 自己的 key 交下来，token 记那个 App 账上）
    ai: Option<super::task::AiOverride>,
    /// 原样带回的归账标签（app_id / user_id / 计费单号…）
    meta: Option<serde_json::Value>,
}

async fn task_create(
    State(st): St,
    Json(body): Json<CreateTask>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let req = SpawnTask {
        kind: body.kind,
        platform: body.platform,
        device_id: body.device_id,
        target: body.target,
        testcase: body.testcase,
        mode: body.mode,
        interactive: body.interactive,
        max_rounds: body.max_rounds,
        // 分钟级、几万 token 的东西必须有个头：预算和超时是一等公民，服务端硬执行
        timeout: Duration::from_secs(body.timeout_s.unwrap_or(1800).clamp(30, 7200)),
        callback_url: body.callback_url,
        ai: body.ai,
        meta: body.meta,
    };
    match super::task::spawn(st.clone(), &st.leases, req).await {
        Ok(v) => Ok((StatusCode::ACCEPTED, Json(v))),
        Err((code, why)) => Err(ApiError(StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_REQUEST), why)),
    }
}

async fn task_list(State(st): St) -> Json<serde_json::Value> {
    Json(json!({"tasks": st.tasks.list()}))
}

async fn task_get(State(st): St, Path(id): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    st.tasks.get(&id, |t| Json(t.view())).ok_or_else(|| not_found(format!("任务 {id} 不存在。")))
}

/// 事件流（SSE）。**先重放已经发生的，再接实时的**——
/// 下发完关掉终端、回头再连上的人，不该看不到前半截
async fn task_events(State(st): St, Path(id): Path<String>) -> Result<Response, ApiError> {
    use axum::response::sse::{Event, Sse};
    use futures_util::stream::{self, StreamExt};

    let past: Vec<String> = st
        .tasks
        .get(&id, |t| t.events.iter().map(|e| e.to_string()).collect())
        .ok_or_else(|| not_found(format!("任务 {id} 不存在。")))?;
    let rx = st.tasks.subscribe(&id);

    let replay = stream::iter(past.into_iter().map(|s| Ok::<_, std::convert::Infallible>(Event::default().data(s))));
    let live = match rx {
        Some(rx) => tokio_stream::wrappers::BroadcastStream::new(rx)
            .filter_map(|r| async move { r.ok().map(|s| Ok(Event::default().data(s))) })
            .boxed(),
        None => stream::empty().boxed(),
    };
    Ok(Sse::new(replay.chain(live)).into_response())
}

/// 交互式会话（WebSocket）：事件推给你，你的回答写进任务进程的 stdin。
/// 桥的是 `JsonFrontend` 那套双向 NDJSON——**那协议本来就是给长连接设计的**（app 在用），
/// 这里只是把管子从 stdio 换成 WebSocket
async fn task_ws(State(st): St, Path(id): Path<String>, ws: WebSocketUpgrade) -> Result<Response, ApiError> {
    let interactive = st
        .tasks
        .get(&id, |t| t.interactive)
        .ok_or_else(|| not_found(format!("任务 {id} 不存在。")))?;
    if !interactive {
        // 说清楚而不是接上一个哑口的连接：headless 任务的问题是**回传**的，不是等人回答的
        return Err(bad(format!(
            "任务 {id} 是 headless 的，不接受交互。它遇到决策点会以 needs_decision 回传问题——             建任务时给 `interactive: true` 才有这条通道。"
        )));
    }
    Ok(ws.on_upgrade(move |socket| ws_pump(socket, st, id)))
}

async fn ws_pump(socket: WebSocket, st: Arc<ServeState>, id: String) {
    use futures_util::{SinkExt, StreamExt};
    let (mut sink, mut stream) = socket.split();

    // 先补齐历史，再转实时——与 SSE 同一个道理
    let past: Vec<String> = st.tasks.get(&id, |t| t.events.iter().map(|e| e.to_string()).collect()).unwrap_or_default();
    for e in past {
        if sink.send(Message::Text(e.into())).await.is_err() {
            return;
        }
    }
    let mut rx = match st.tasks.subscribe(&id) {
        Some(rx) => rx,
        None => return,
    };

    let id2 = id.clone();
    let st2 = st.clone();
    // 收：客户端发来的每一行就是一条 UiCommand（{"type":"answer","text":"…"}），原样喂给子进程
    let recv = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Text(t) = msg {
                if let Err(e) = st2.tasks.answer(&id2, t.to_string()) {
                    tracing::warn!(target: "tke::task", "回答没送达: {e}");
                    break;
                }
            }
        }
    });

    while let Ok(line) = rx.recv().await {
        if sink.send(Message::Text(line.into())).await.is_err() {
            break;
        }
    }
    recv.abort();
}

/// 报告：ui 任务出 `report.html`，security 出 `security-report.html`——
/// 调用方不用记是哪一种，这里自己找
async fn task_report(State(st): St, Path(id): Path<String>) -> Result<Response, ApiError> {
    let dirs = st
        .tasks
        .get(&id, |t| t.lease.dirs.clone())
        .ok_or_else(|| not_found(format!("任务 {id} 不存在。")))?;
    for name in ["security-report.html", "report.html"] {
        let p = dirs.logs.join(name);
        if p.exists() {
            let bytes = std::fs::read(&p).map_err(|e| not_found(e.to_string()))?;
            return Ok(([(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")], bytes).into_response());
        }
    }
    Err(not_found(format!(
        "任务 {id} 还没有报告——可能还在跑，也可能没跑到出报告那一步（看 /v1/tasks/{id} 的 outcome）。"
    )))
}
