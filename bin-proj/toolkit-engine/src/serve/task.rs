// 【任务层 L2】ADR-0022 D3 的第二层：**跑 tke 自带 AI 的编排**（harness / security），
// 用节点的 key，token 计入用户账单——与命令层（L1，零 LLM 面）泾渭分明。
//
// 形态：`POST /v1/tasks` 起一个后台任务 → 事件流看进度 → 终态出报告 + webhook 回调。
// 用户下发完就能关掉终端，回头收报告。
//
// **决策点不得自行决定**（ADR-0022 D6 / INV-3）：headless 任务遇到 `awaiting_input`
// 立刻以 `needs_decision` + 问题原文终止回传，由平台推给用户；交互式任务（WS 挂着）
// 才把问题转给人。headless 一旦开始自己拿主意，ADR-0022 就失效了。
//
// 执行仍是子进程（D2 不变）：`tke harness --json` 的 NDJSON 双向协议本来就是给长连接设计的
// （Electron app 在用），这里只是把管子从 stdio 换成 SSE/WebSocket。

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::broadcast;

use super::lease::{Lease, LeaseTable};

/// 五态出口（ADR-0009 的条款，由 ADR-0022 D6 复活）。**别合并成一个 success 布尔**
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    /// 遇到决策点（要凭据 / 要做不可逆的事）——问题回传给平台，不自行决定
    NeedsDecision,
    /// 前提不满足（没登录、缺环境）——诊断清楚，不硬修（INV-12）
    Blocked,
    Error,
}

impl Outcome {
    /// 映射成退出码，与 ADR-0009 一致（平台侧脚本可以直接用）
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Passed => 0,
            Self::Failed => 1,
            Self::NeedsDecision => 2,
            Self::Blocked => 3,
            Self::Error => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Running,
    Finished,
}

pub struct Task {
    pub id: String,
    pub kind: String,
    pub target: Option<String>,
    pub mode: Option<String>,
    pub interactive: bool,
    pub lease: Lease,
    pub state: TaskState,
    pub outcome: Option<Outcome>,
    /// 终态说明：为什么是这个 outcome（needs_decision 时装着问题原文）
    pub detail: Option<serde_json::Value>,
    /// 事件重放缓冲：晚来的订阅者也要能从头看（否则"关掉终端再回来"就断片了）
    pub events: Vec<serde_json::Value>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub callback_url: Option<String>,
    /// 调用方带来的归账标签，原样回传
    pub meta: Option<serde_json::Value>,
    /// 往子进程 stdin 写（交互式任务回答问题用）
    pub answer_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    /// 全程 token 用量（引擎的 `summary` 事件给）。**测不到时是 null 不是 0**——
    /// 0 会被平台当成"这次没花钱"，而真相是"没测量到"（INV-9：查不了要说出来）
    pub usage: Option<serde_json::Value>,
    /// stderr 的最后几行。它不是给调用方的协议（所以不进事件流），
    /// 但任务挂掉时**它就是唯一的线索**（缺 API key 之类）——吞掉等于把最有用的那句话删了（P-46）
    pub stderr_tail: Arc<Mutex<Vec<String>>>,
}

impl Task {
    pub fn view(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.id,
            "kind": self.kind,
            "target": self.target,
            "mode": self.mode,
            "interactive": self.interactive,
            "state": self.state,
            "outcome": self.outcome,
            "exit_code": self.outcome.map(|o| o.exit_code()),
            "detail": self.detail,
            "device": self.lease.device.id,
            "session_id": self.lease.id,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "usage": self.usage,
            "meta": self.meta,
            "events": self.events.len(),
            "report": format!("/v1/tasks/{}/report", self.id),
        })
    }
}

pub struct TaskTable {
    tasks: Mutex<HashMap<String, Task>>,
    /// 每个任务一个广播通道：SSE / WS 订阅它拿实时事件
    channels: Mutex<HashMap<String, broadcast::Sender<String>>>,
    seq: std::sync::atomic::AtomicU64,
}

impl Default for TaskTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskTable {
    pub fn new() -> Self {
        Self { tasks: Mutex::new(HashMap::new()), channels: Mutex::new(HashMap::new()), seq: Default::default() }
    }

    fn next_id(&self) -> String {
        let n = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("t{:x}{:03x}", super::lease::now_secs(), n & 0xfff)
    }

    pub fn get<T>(&self, id: &str, f: impl FnOnce(&Task) -> T) -> Option<T> {
        self.tasks.lock().expect("tasks 锁中毒").get(id).map(f)
    }

    pub fn list(&self) -> Vec<serde_json::Value> {
        self.tasks.lock().expect("tasks 锁中毒").values().map(|t| t.view()).collect()
    }

    pub fn subscribe(&self, id: &str) -> Option<broadcast::Receiver<String>> {
        self.channels.lock().expect("channels 锁中毒").get(id).map(|tx| tx.subscribe())
    }

    /// 回答一个正在等的问题（交互式任务）
    pub fn answer(&self, id: &str, line: String) -> Result<(), String> {
        let tasks = self.tasks.lock().expect("tasks 锁中毒");
        let t = tasks.get(id).ok_or_else(|| format!("任务 {id} 不存在"))?;
        let tx = t.answer_tx.as_ref().ok_or("这个任务不接受输入（不是交互式任务，或已经结束）")?;
        tx.send(line).map_err(|_| "任务已经结束了".to_string())
    }
}

/// 起任务用的参数（路由层校验后传进来）
/// 调用方交下来的 AI 凭据（ADR-0023 D3 修订：平台把 App 自己的 key 交下来，
/// token 计到那个 App 账上）。**key 只走环境变量**——进 argv 会被 `ps aux` 看见，
/// 写配置文件会落到磁盘上
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct AiOverride {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub reasoning_effort: Option<String>,
}

pub struct SpawnTask {
    pub kind: String,
    pub target: Option<String>,
    pub testcase: Option<String>,
    pub mode: Option<String>,
    pub interactive: bool,
    pub max_rounds: Option<u32>,
    pub timeout: Duration,
    pub callback_url: Option<String>,
    /// 调用方的 AI 凭据；None = 用节点自己的配置
    pub ai: Option<AiOverride>,
    /// **不解释、原样带回**的标签（平台的 app_id / user_id / 计费单号…）。
    /// tke 不认识"用户"（ADR-0022 D1），归账靠调用方自己带的这张纸条——
    /// 设备租赁与 AI 计费共用同一条透传路
    pub meta: Option<serde_json::Value>,
}

/// 把任务参数翻译成子进程 argv。**注意与 L1 的区别**：这里是服务端主动跑 AI 编排，
/// 所以 `harness`/`security` 出现在这儿是对的——它们只是不在**命令层**白名单里
pub fn build_task_argv(req: &SpawnTask, lease: &Lease) -> Result<Vec<String>, String> {
    let mut argv: Vec<String> = vec![
        "--json".into(),
        "--log".into(),
        lease.dirs.logs.to_string_lossy().into_owned(),
        "--cache".into(),
        lease.dirs.cache.to_string_lossy().into_owned(),
        "--current-dir".into(),
        lease.dirs.workspace.to_string_lossy().into_owned(),
        "--scripts".into(),
        lease.dirs.workspace.to_string_lossy().into_owned(),
    ];
    if !lease.device.id.is_empty() {
        argv.push("-d".into());
        argv.push(lease.device.id.clone());
    }

    match req.kind.as_str() {
        "ui" => {
            argv.push("harness".into());
            if let Some(tc) = &req.testcase {
                argv.push("--testcase".into());
                argv.push(tc.clone());
            }
            if let Some(n) = req.max_rounds {
                argv.push("--max-rounds".into());
                argv.push(n.to_string());
            }
        }
        "security" => {
            argv.push("security".into());
            let target = req.target.as_ref().ok_or("security 任务必须给 target")?;
            argv.push(target.clone());
            if let Some(m) = &req.mode {
                argv.push("--mode".into());
                argv.push(m.clone());
            }
        }
        other => return Err(format!("不认识的任务类型 `{other}`（只有 ui / security）")),
    }
    Ok(argv)
}

/// 强度阶梯的远程口子：`red-team` **服务端硬拒**（ADR-0022 D5）——
/// 破坏性、不可逆的向量需要"人就在那台机器前"这个物理前提
pub fn check_mode(mode: Option<&str>) -> Result<(), String> {
    match mode {
        Some("red-team") => Err(
            "`red-team` 不对远程开放：破坏性/不可逆的向量需要人在场（ADR-0022 D5）。\
             可用 passive / safe（默认）/ aggressive。"
                .into(),
        ),
        Some(m) if !matches!(m, "passive" | "safe" | "aggressive") => {
            Err(format!("不认识的强度档 `{m}`（passive / safe / aggressive）。"))
        }
        _ => Ok(()),
    }
}

/// 从 `summary` 事件里抽全程用量（计费要它，ADR-0023 D3）
pub fn usage_from_event(ev: &serde_json::Value) -> Option<serde_json::Value> {
    if ev.get("type").and_then(|t| t.as_str()) != Some("summary") {
        return None;
    }
    let t = ev.get("tokens")?;
    let prompt = t.get("prompt").and_then(|v| v.as_i64()).unwrap_or(0);
    let completion = t.get("completion").and_then(|v| v.as_i64()).unwrap_or(0);
    Some(serde_json::json!({
        "prompt_tokens": prompt,
        "completion_tokens": completion,
        "total_tokens": prompt + completion,
        "model": ev.get("model"),
    }))
}

/// 一次性命令（`tke security --json` 这类）的终局输出：**没有 `type` 字段的那个 JSON**。
/// 它不是 UiEvent 流的一部分——安全轨无头跑完只打一个结果对象就退出。
/// 判据是它的 `success` 字段；认不出来就返回 None，交给"没有 done = 没跑完"兜底
pub fn oneshot_outcome(ev: &serde_json::Value) -> Option<(Outcome, serde_json::Value)> {
    if ev.get("type").is_some() {
        return None;
    }
    let ok = ev.get("success")?.as_bool()?;
    Some((if ok { Outcome::Passed } else { Outcome::Failed }, ev.clone()))
}

/// 从一行 NDJSON 事件推断终态。返回 None = 还没到终局
pub fn outcome_from_event(ev: &serde_json::Value, interactive: bool) -> Option<(Outcome, serde_json::Value)> {
    match ev.get("type").and_then(|t| t.as_str()) {
        // **决策点**：headless 下立刻回传，不自行决定（D6 / INV-3）
        Some("awaiting_input") if !interactive => Some((
            Outcome::NeedsDecision,
            serde_json::json!({
                "question": ev.get("question"),
                "options": ev.get("options"),
                "round": ev.get("round"),
                "hint": "把答案交给用户或平台决定后，用同一份上下文重新下发任务。",
            }),
        )),
        Some("done") => {
            let ok = ev.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
            Some((
                if ok { Outcome::Passed } else { Outcome::Failed },
                serde_json::json!({"script": ev.get("script"), "conversation": ev.get("conversation")}),
            ))
        }
        _ => None,
    }
}

/// 起一个任务：租会话 → spawn 子进程 → 后台泵事件 → 终态回调
pub async fn spawn(
    st: Arc<super::ServeState>,
    leases: &LeaseTable,
    req: SpawnTask,
) -> Result<serde_json::Value, (u16, String)> {
    check_mode(req.mode.as_deref()).map_err(|e| (400u16, e))?;
    // **先校验参数再租设备**：不然一个拼错的 kind 会先撞上"没有 android 设备可租"，
    // 把"你写错了"报成"这儿没有"——调用方会去查错误的方向
    if !matches!(req.kind.as_str(), "ui" | "security") {
        return Err((400u16, format!("不认识的任务类型 `{}`（只有 ui / security）", req.kind)));
    }
    if req.kind == "security" && req.target.is_none() {
        return Err((400u16, "security 任务必须给 target（要测哪个 URL）".into()));
    }

    // ui 任务要设备；security 只打 URL，开无设备会话（不计设备时长）
    let platform = if req.kind == "security" { "none" } else { "android" };
    let want = req.target.as_deref().filter(|_| req.kind == "ui");
    let lease = leases
        .acquire(Some(platform), want, Some(Duration::from_secs(req.timeout.as_secs().max(600))))
        .map_err(|e| (409u16, e.to_string()))?;

    let argv = build_task_argv(&req, &lease).map_err(|e| (400u16, e))?;
    let id = st.tasks.next_id();

    let (tx, _) = broadcast::channel::<String>(1024);
    st.tasks.channels.lock().expect("channels 锁中毒").insert(id.clone(), tx.clone());

    let (ans_tx, mut ans_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let task = Task {
        id: id.clone(),
        kind: req.kind.clone(),
        target: req.target.clone(),
        mode: req.mode.clone(),
        interactive: req.interactive,
        lease: lease.clone(),
        state: TaskState::Running,
        outcome: None,
        detail: None,
        events: Vec::new(),
        started_at: super::lease::now_secs(),
        finished_at: None,
        callback_url: req.callback_url.clone(),
        meta: req.meta.clone(),
        answer_tx: req.interactive.then_some(ans_tx),
        usage: None,
        stderr_tail: Arc::new(Mutex::new(Vec::new())),
    };
    let view = task.view();
    st.tasks.tasks.lock().expect("tasks 锁中毒").insert(id.clone(), task);

    let mut cmd = tokio::process::Command::new(&st.bin);
    cmd.args(&argv)
        .current_dir(&lease.dirs.workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("TKE_ALLOW_IOS")
        // 上一次任务的凭据不能漏进这一次：一律先清干净再按本次的设
        .env_remove("TKE_AI_PROVIDER")
        .env_remove("TKE_AI_MODEL")
        .env_remove("TKE_AI_KEY")
        .env_remove("TKE_AI_BASE_URL")
        .env_remove("TKE_AI_REASONING")
        // 节点自己也可能配了 TKE_REMOTE（比如运维在同一台机器上试过客户端）——
        // 不清掉的话，任务里的 tke 会把命令再转发出去，兜圈子
        .env_remove("TKE_REMOTE");
    // **key 只走环境变量**（argv 会被 `ps aux` 看见，配置文件会落到磁盘上）
    let secret = req.ai.as_ref().and_then(|a| a.api_key.clone()).filter(|k| k.len() >= 8);
    if let Some(ai) = &req.ai {
        for (k, v) in [
            ("TKE_AI_PROVIDER", &ai.provider),
            ("TKE_AI_MODEL", &ai.model),
            ("TKE_AI_KEY", &ai.api_key),
            ("TKE_AI_BASE_URL", &ai.base_url),
            ("TKE_AI_REASONING", &ai.reasoning_effort),
        ] {
            if let Some(v) = v.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                cmd.env(k, v);
            }
        }
    }
    let mut child = cmd.spawn().map_err(|e| (500u16, format!("起任务进程失败: {e}")))?;

    let mut stdin = child.stdin.take().expect("stdin 已接管");
    let stdout = child.stdout.take().expect("stdout 已接管");
    let stderr = child.stderr.take().expect("stderr 已接管");

    // 交互式：把 WS 收到的回答写进子进程 stdin（NDJSON，一行一条 UiCommand）
    tokio::spawn(async move {
        while let Some(line) = ans_rx.recv().await {
            if stdin.write_all(format!("{line}\n").as_bytes()).await.is_err() {
                break;
            }
            let _ = stdin.flush().await;
        }
    });

    // stderr 不进事件流（它是节点侧日志，不是给调用方的协议），但**留最后 50 行**——
    // 任务挂了的时候它是唯一的线索
    let tail = st.tasks.get(&id, |t| t.stderr_tail.clone()).unwrap_or_default();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            tracing::debug!(target: "tke::task", "{l}");
            // stderr 会原样回给调用方（任务挂了它是唯一线索），
            // 所以凭据必须在**进缓冲之前**抹掉——一旦进去就会经 detail 流出去
            let mut t = tail.lock().expect("stderr_tail 锁中毒");
            t.push(scrub(&l, secret.as_deref()));
            if t.len() > 50 {
                t.remove(0);
            }
        }
    });

    let st2 = st.clone();
    let id2 = id.clone();
    let timeout = req.timeout;
    tokio::spawn(async move {
        let pump = pump_events(st2.clone(), &id2, stdout, tx, req.interactive);
        let final_outcome = match tokio::time::timeout(timeout, pump).await {
            Ok(o) => o,
            Err(_) => {
                let _ = child.start_kill();
                Some((
                    Outcome::Error,
                    serde_json::json!({"why": format!("超时（{}s）——任务已被杀掉", timeout.as_secs())}),
                ))
            }
        };
        // 决策点回传时子进程还活着，得收掉它——否则它会一直占着设备
        let _ = child.start_kill();
        let code = child.wait().await.ok().and_then(|s| s.code());
        finish(st2, &id2, final_outcome, code).await;
    });

    Ok(view)
}

/// 把凭据从一行文本里抹掉。**长度 <8 的不抹**——太短的"密钥"多半是占位符，
/// 拿它做全局替换会把正常文本切得七零八落，反而更难查
pub fn scrub(line: &str, secret: Option<&str>) -> String {
    match secret {
        Some(s) if s.len() >= 8 && line.contains(s) => line.replace(s, "••••••"),
        _ => line.to_string(),
    }
}

/// 逐行读子进程 stdout：进事件缓冲 + 广播 + 认终局
async fn pump_events(
    st: Arc<super::ServeState>,
    id: &str,
    stdout: tokio::process::ChildStdout,
    tx: broadcast::Sender<String>,
    interactive: bool,
) -> Option<(Outcome, serde_json::Value)> {
    let mut lines = BufReader::new(stdout).lines();
    let mut terminal = None;
    let mut oneshot = None;
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let ev: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            // 解析不出来的行不丢：调用方看得到原文才查得下去（P-46）
            Err(_) => serde_json::json!({"type": "raw", "line": line}),
        };
        if let Some(u) = usage_from_event(&ev) {
            if let Some(t) = st.tasks.tasks.lock().expect("tasks 锁中毒").get_mut(id) {
                t.usage = Some(u);
            }
        }
        // 一次性命令的结果对象:每来一个就覆盖,最后那个才算终局。
        // 用量也可能长在这儿(安全轨不走 summary 事件),一并收下
        if let Some(o) = oneshot_outcome(&ev) {
            if let Some(u) = ev.get("usage").filter(|u| !u.is_null()) {
                if let Some(t) = st.tasks.tasks.lock().expect("tasks 锁中毒").get_mut(id) {
                    t.usage = Some(u.clone());
                }
            }
            oneshot = Some(o);
        }
        if let Some(t) = st.tasks.tasks.lock().expect("tasks 锁中毒").get_mut(id) {
            t.events.push(ev.clone());
        }
        let _ = tx.send(ev.to_string());

        if terminal.is_none() {
            if let Some(o) = outcome_from_event(&ev, interactive) {
                terminal = Some(o);
                // needs_decision 要**立刻**停：继续跑下去就等于让它自己拿主意了
                if terminal.as_ref().map(|(o, _)| *o) == Some(Outcome::NeedsDecision) {
                    break;
                }
            }
        }
    }
    // UiEvent 流的终局优先;没有的话看一次性命令的结果对象(安全轨无头就是这条路)
    terminal.or(oneshot)
}

/// 收尾：写终态 + 广播一条 task_end + 释放会话 + 回调
async fn finish(
    st: Arc<super::ServeState>,
    id: &str,
    outcome: Option<(Outcome, serde_json::Value)>,
    exit_code: Option<i32>,
) {
    let stderr_tail: Vec<String> = st
        .tasks
        .get(id, |t| t.stderr_tail.lock().expect("stderr_tail 锁中毒").clone())
        .unwrap_or_default();
    let (outcome, mut detail) = outcome.unwrap_or_else(|| {
        // 没有 done 也没有决策点 = 进程自己没了。退出码 0 也算异常：
        // 编排本该以 done 收束，没有它就是**没跑完**，不能报成功
        (
            Outcome::Error,
            serde_json::json!({"why": "任务进程没有正常收束（没有 done 事件）", "exit_code": exit_code}),
        )
    });
    // 出错时把 stderr 尾巴一起交出去：调用方看不到节点的日志，
    // 不给这个他就只知道"失败了"，不知道是缺 API key 还是别的
    if outcome == Outcome::Error && !stderr_tail.is_empty() {
        if let Some(obj) = detail.as_object_mut() {
            obj.insert("stderr_tail".into(), serde_json::json!(stderr_tail));
        }
    }

    let (callback, view) = {
        let mut tasks = st.tasks.tasks.lock().expect("tasks 锁中毒");
        let Some(t) = tasks.get_mut(id) else { return };
        t.state = TaskState::Finished;
        t.outcome = Some(outcome);
        t.detail = Some(detail);
        t.finished_at = Some(super::lease::now_secs());
        t.answer_tx = None;
        (t.callback_url.clone(), t.view())
    };

    // 终局事件既广播也**进重放缓冲**：晚来的订阅者要能看到结局，
    // 否则"任务早跑完了才来看"只能看到一片空白
    let end_ev = serde_json::json!({"type": "task_end", "task": view});
    if let Some(t) = st.tasks.tasks.lock().expect("tasks 锁中毒").get_mut(id) {
        t.events.push(end_ev.clone());
    }
    if let Some(tx) = st.tasks.channels.lock().expect("channels 锁中毒").get(id) {
        let _ = tx.send(end_ev.to_string());
    }

    // 设备还回去并复位（INV-17）——任务跑完不还，等于把设备占到 TTL
    let lease = st.tasks.get(id, |t| t.lease.clone());
    if let Some(l) = lease {
        st.leases.take(&l.id);
        super::routes::run_reset(&st, &l).await;
    }

    if let Some(url) = callback {
        post_callback(&url, &view).await;
    }
}

/// 终态 webhook：**尽力而为但不静默**（INV-9）。发不出去要留痕，
/// 否则平台那边只会看到"任务永远没回来"
async fn post_callback(url: &str, view: &serde_json::Value) {
    let url = url.to_string();
    let body = view.to_string();
    let r = tokio::task::spawn_blocking(move || {
        ureq::post(&url)
            .timeout(Duration::from_secs(15))
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await;
    match r {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!(target: "tke::task", "回调失败: {e}"),
        Err(e) => tracing::warn!(target: "tke::task", "回调任务崩了: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::lease::{PoolDevice, SessionDirs};

    fn lease(device: &str) -> Lease {
        Lease {
            id: "s1".into(),
            device: PoolDevice { id: device.into(), kind: "web".into(), label: "x".into() , model: String::new(), os: String::new() },
            dirs: SessionDirs {
                root: "/w".into(),
                workspace: "/w/ws".into(),
                logs: "/w/ws/logs".into(),
                cache: "/w/cache".into(),
            },
            created_at: 0,
            expires_at: u64::MAX,
            launched_apps: vec![],
            meta: None,
        }
    }

    #[test]
    fn red_team不对远程开放() {
        let e = check_mode(Some("red-team")).unwrap_err();
        assert!(e.contains("人在场"), "{e}");
        for ok in ["passive", "safe", "aggressive"] {
            assert!(check_mode(Some(ok)).is_ok(), "{ok} 该放行");
        }
        assert!(check_mode(None).is_ok());
        assert!(check_mode(Some("随便编的")).is_err());
    }

    #[test]
    fn ui任务跑harness安全任务跑security() {
        let l = lease("web:1");
        let ui = build_task_argv(
            &SpawnTask {
                kind: "ui".into(), target: None, testcase: Some("登录能用吗".into()), mode: None,
                interactive: false, max_rounds: Some(20), timeout: Duration::from_secs(60), callback_url: None, ai: None, meta: None,
            },
            &l,
        ).unwrap();
        assert!(ui.contains(&"harness".to_string()) && ui.contains(&"登录能用吗".to_string()));
        // 任务层是**服务端主动跑 AI**，所以这里出现 harness 是对的——
        // 它只是不在命令层（L1）白名单里
        assert!(ui.contains(&"--json".to_string()) && ui.contains(&"-d".to_string()));

        let sec = build_task_argv(
            &SpawnTask {
                kind: "security".into(), target: Some("https://x".into()), testcase: None,
                mode: Some("safe".into()), interactive: false, max_rounds: None,
                timeout: Duration::from_secs(60), callback_url: None, ai: None, meta: None,
            },
            &lease(""),
        ).unwrap();
        assert!(sec.contains(&"security".to_string()) && sec.contains(&"https://x".to_string()));
        assert!(!sec.contains(&"-d".to_string()), "安全任务不碰设备，别注入 -d");
    }

    #[test]
    fn 缺目标的安全任务当场报错() {
        let e = build_task_argv(
            &SpawnTask {
                kind: "security".into(), target: None, testcase: None, mode: None, interactive: false,
                max_rounds: None, timeout: Duration::from_secs(1), callback_url: None, ai: None, meta: None,
            },
            &lease(""),
        ).unwrap_err();
        assert!(e.contains("target"), "{e}");
    }

    #[test]
    fn headless遇到问题就回传不自己拿主意() {
        let ask = serde_json::json!({"type":"awaiting_input","question":"要用哪个账号登录？","options":["A","B"],"round":3});
        let (o, d) = outcome_from_event(&ask, false).expect("headless 必须终止");
        assert_eq!(o, Outcome::NeedsDecision);
        assert_eq!(d["question"], "要用哪个账号登录？");
        assert_eq!(o.exit_code(), 2, "退出码与 ADR-0009 的五态一致");
        // 交互式任务里问题是转给人的，不是终局
        assert!(outcome_from_event(&ask, true).is_none());
    }

    #[test]
    fn done事件区分成败() {
        let ok = serde_json::json!({"type":"done","success":true,"script":"login.tks","conversation":"c.jsonl"});
        assert_eq!(outcome_from_event(&ok, false).unwrap().0, Outcome::Passed);
        let bad = serde_json::json!({"type":"done","success":false});
        assert_eq!(outcome_from_event(&bad, false).unwrap().0, Outcome::Failed);
        // 中途事件不算终局
        assert!(outcome_from_event(&serde_json::json!({"type":"phase"}), false).is_none());
    }

    #[test]
    fn 一次性命令的结果对象也算终局() {
        // `tke security --json` 无头跑完只打一个**没有 type 字段**的结果对象就退出。
        // 不认它的话,一次**成功**的安全扫描会被判成"没跑完"(P3 只测了失败路径,漏了这条)
        let ok = serde_json::json!({"success":true,"target":"https://x","finding_count":3,"report_html":"/p/r.html"});
        let (o, d) = oneshot_outcome(&ok).unwrap();
        assert_eq!(o, Outcome::Passed);
        assert_eq!(d["finding_count"], 3, "结果对象原样带走,平台要靠它拿报告路径");
        assert_eq!(oneshot_outcome(&serde_json::json!({"success":false,"error":"x"})).unwrap().0, Outcome::Failed);
        // UiEvent 流里的东西不走这条路(它们有 type)
        assert!(oneshot_outcome(&serde_json::json!({"type":"done","success":true})).is_none());
        assert!(oneshot_outcome(&serde_json::json!({"foo":1})).is_none());
    }

    #[test]
    fn 安全轨的用量长在结果对象上() {
        // 安全轨不走 `summary` 事件（无头跑完只打一个结果对象），用量在那个对象的 usage 字段里。
        // 这条钉住"两条路都能收到用量"——只认 summary 的话安全任务永远计不了费
        let ev = serde_json::json!({
            "success": true, "target": "https://x",
            "usage": {"prompt_tokens": 800, "completion_tokens": 120, "total_tokens": 920, "model": "m"}
        });
        assert!(usage_from_event(&ev).is_none(), "它不是 summary 事件");
        assert_eq!(oneshot_outcome(&ev).unwrap().0, Outcome::Passed);
        assert_eq!(ev["usage"]["total_tokens"], 920, "收用量的那段读的就是这个字段");
    }

    #[test]
    fn 用量从summary事件里抽() {
        let ev = serde_json::json!({"type":"summary","model":"claude-sonnet-4-6","tokens":{"prompt":1200,"completion":340}});
        let u = usage_from_event(&ev).unwrap();
        assert_eq!(u["prompt_tokens"], 1200);
        assert_eq!(u["completion_tokens"], 340);
        assert_eq!(u["total_tokens"], 1540, "平台按总量计费,别让它自己加");
        assert_eq!(u["model"], "claude-sonnet-4-6");
        // 别的事件不给用量 —— **测不到时是 null 不是 0**,0 会被当成"这次没花钱"
        assert!(usage_from_event(&serde_json::json!({"type":"phase"})).is_none());
    }

    #[test]
    fn 凭据不许进stderr尾巴() {
        // stderr 会原样回给调用方（任务挂了它是唯一线索），所以凭据必须在进缓冲前抹掉。
        // 平台交下来的是**用户自己的 key**，漏一次就是把它送出去了
        let key = "sk-ant-api03-verysecret";
        assert_eq!(scrub(&format!("Error: bad key {key} rejected"), Some(key)), "Error: bad key •••••• rejected");
        // 太短的不抹：那多半是占位符，全局替换会把正常文本切碎，反而更难查
        assert_eq!(scrub("abc in text", Some("abc")), "abc in text");
        assert_eq!(scrub("没有凭据的一行", Some(key)), "没有凭据的一行");
        assert_eq!(scrub("没配 key 的时候", None), "没配 key 的时候");
    }

    #[test]
    fn 归账标签原样带回() {
        // tke 不认识"用户"（ADR-0022 D1），归账靠调用方自己带的这张纸条。
        // 它必须**原样**回去——tke 一旦开始解释它，D1 的边界就松了
        let l = lease("web:1");
        let meta = serde_json::json!({"app_id": "a-1", "user_id": "u-9", "bill_no": 42});
        let t = Task {
            id: "t1".into(), kind: "ui".into(), target: None, mode: None, interactive: false,
            lease: l, state: TaskState::Running, outcome: None, detail: None, events: vec![],
            started_at: 0, finished_at: None, callback_url: None, meta: Some(meta.clone()),
            answer_tx: None, usage: None, stderr_tail: Default::default(),
        };
        assert_eq!(t.view()["meta"], meta);
    }

    #[test]
    fn 五态退出码对得上() {
        assert_eq!(
            [Outcome::Passed, Outcome::Failed, Outcome::NeedsDecision, Outcome::Blocked, Outcome::Error]
                .map(|o| o.exit_code()),
            [0, 1, 2, 3, 4]
        );
    }
}
