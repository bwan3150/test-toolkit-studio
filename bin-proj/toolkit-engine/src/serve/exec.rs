// 【子进程执行】ADR-0022 D2：一个请求 = 一个 tke 子进程。
//
// 为什么不在同进程里直接调 handler：`JsonOutput::success/error` 直接 `process::exit`，
// `main.rs` 还有三处进程级全局态（set_ocr_url / set_web_headless / interrupt::install）——
// 同进程并发跑两条命令根本不可能。而"每命令一个进程"**正是 skill 今天的样子**
// （会话靠 web/infra.rs::session_file 跨进程复用），所以这不是妥协，是行为等价。
//
// 服务端在这里注入四样东西，调用方无权指定（对应 allowlist 的禁用旗标）：
//   --json（机器可读）/ --log·--cache·--current-dir（会话隔离，INV-17）/ -d（本会话租到的那台）
//   / --copilot=false（L1 是零 LLM 面，INV-16 延伸条款——否则用户白嫖平台的 key）
//
// 计时分三段落进响应（Q-17：先量再优化）。网络 RTT 由客户端自己量，这里量不到。

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;

use super::allowlist::Validated;
use super::lease::SessionDirs;

/// 分层计时——**别把它折叠成一个总数**：慢在哪一段决定了下一步该改什么
#[derive(Debug, Clone, serde::Serialize)]
pub struct Timing {
    /// 起进程本身花了多久
    pub spawn_ms: u64,
    /// 进程从起来到退出
    pub run_ms: u64,
    /// 服务端侧总耗时（含路径解析等杂项）
    pub total_ms: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct ExecOutcome {
    /// 超时被杀时为 null
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    /// stdout 是单个 JSON 对象时给这个（绝大多数命令）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_json: Option<serde_json::Value>,
    /// stdout 是 NDJSON 事件流时给这个（run / steps）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stdout_events: Vec<serde_json::Value>,
    /// 原文一律带上——解析不出来时它是唯一线索（P-46：吞掉下层的错就是把最有用的那句删了）
    pub stdout_raw: String,
    pub stderr: String,
    pub timing: Timing,
}

pub struct ExecRequest {
    /// 已过白名单的 argv
    pub validated: Validated,
    pub dirs: SessionDirs,
    /// 注入的 `-d`；None = 不注入（health 这类不认设备的调用）
    pub device: Option<String>,
    pub timeout: Duration,
}

/// 组装最终 argv：**全局参数放在子命令前面**。
/// 放后面会撞 P-44（子命令开关与全局参数撞名）与 `--headless` 吃掉子命令那类坑，
/// 前置是唯一稳的写法。
pub fn build_argv(req: &ExecRequest, resolved_paths: &[(usize, PathBuf)]) -> Vec<String> {
    // `--log` 只在调用方没给的时候才注入：给了就用他的（已沙箱进会话工作区），
    // 否则本地/远程的目录对不上，`tke report <同一个相对路径>` 就找不到东西
    let caller_gave_log = req.validated.argv.iter().any(|a| a == "--log");
    let mut out: Vec<String> = vec!["--json".into()];
    if !caller_gave_log {
        out.push("--log".into());
        out.push(req.dirs.logs.to_string_lossy().into_owned());
    }
    out.extend(vec![
        "--cache".into(),
        req.dirs.cache.to_string_lossy().into_owned(),
        "--current-dir".into(),
        req.dirs.workspace.to_string_lossy().into_owned(),
        // 等号形态：`--copilot false` 会不会吃掉后面的 token 取决于 clap 的心情，别赌
        "--copilot=false".into(),
    ]);
    if let Some(d) = &req.device {
        out.push("-d".into());
        out.push(d.clone());
    }
    let mut argv = req.validated.argv.clone();
    for (idx, path) in resolved_paths {
        argv[*idx] = path.to_string_lossy().into_owned();
    }
    out.extend(argv);
    out
}

/// 把被标记的宿主路径解析进会话工作区（越界的直接拒）
pub fn resolve_paths(req: &ExecRequest) -> Result<Vec<(usize, PathBuf)>, String> {
    req.validated
        .host_path_idx
        .iter()
        .map(|&i| {
            let raw = &req.validated.argv[i];
            crate::utils::resolve_in_workspace(&req.dirs.workspace, raw)
                .map(|p| (i, p))
                .map_err(|why| format!("参数 `{raw}` 越界：{why}"))
        })
        .collect()
}

/// 跑一条命令。**不返回 Err**——命令自己失败（非零退出）是正常结果，
/// 要如实回给调用方，不是服务端的错误
pub async fn run(bin: &std::path::Path, req: &ExecRequest) -> Result<ExecOutcome, String> {
    let t0 = Instant::now();
    let resolved = resolve_paths(req)?;
    let argv = build_argv(req, &resolved);

    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(&argv)
        .current_dir(&req.dirs.workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // 逃生口不经网络暴露：宿主机能力门禁（iOS 只在 macOS）不许被远程绕过
    cmd.env_remove("TKE_ALLOW_IOS");

    let (exit_code, timed_out, stdout_raw, stderr, spawn_ms, run_ms) =
        spawn_and_wait(cmd, req.timeout).await?;

    let (stdout_json, stdout_events) = parse_stdout(&stdout_raw);
    Ok(ExecOutcome {
        exit_code,
        timed_out,
        stdout_json,
        stdout_events,
        stdout_raw,
        stderr,
        timing: Timing { spawn_ms, run_ms, total_ms: t0.elapsed().as_millis() as u64 },
    })
}

/// 起进程 → 读干净两个管道 → 等退出（超时就杀）。
/// 单独一层是为了能被直接单测：`run()` 的 argv 是拼死的，测不出"挂住会不会被杀"
async fn spawn_and_wait(
    mut cmd: tokio::process::Command,
    timeout: Duration,
) -> Result<(Option<i32>, bool, String, String, u64, u64), String> {
    let spawn_at = Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("起 tke 子进程失败: {e}"))?;
    let spawn_ms = spawn_at.elapsed().as_millis() as u64;

    let mut so = child.stdout.take().expect("stdout 已接管");
    let mut se = child.stderr.take().expect("stderr 已接管");
    let mut stdout_raw = String::new();
    let mut stderr = String::new();

    let run_at = Instant::now();
    let waited = tokio::time::timeout(timeout, async {
        // 三件事一起等：读干净两个管道再收尸，否则大输出会把子进程堵死
        tokio::join!(
            so.read_to_string(&mut stdout_raw),
            se.read_to_string(&mut stderr),
            child.wait()
        )
    })
    .await;

    let (exit_code, timed_out) = match waited {
        Ok((_, _, status)) => (status.ok().and_then(|s| s.code()), false),
        Err(_) => {
            // 超时就杀——挂着的进程会一直占着设备（adb 那次无限挂的教训）
            let _ = child.start_kill();
            let _ = child.wait().await;
            (None, true)
        }
    };
    Ok((exit_code, timed_out, stdout_raw, stderr, spawn_ms, run_at.elapsed().as_millis() as u64))
}

/// stdout 可能是单个 JSON（多数命令），也可能是 NDJSON 事件流（run/steps）。
/// 两种都解析出来，原文照样带走
fn parse_stdout(raw: &str) -> (Option<serde_json::Value>, Vec<serde_json::Value>) {
    let lines: Vec<&str> = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    match lines.len() {
        0 => (None, Vec::new()),
        1 => (serde_json::from_str(lines[0]).ok(), Vec::new()),
        _ => {
            let events: Vec<serde_json::Value> =
                lines.iter().filter_map(|l| serde_json::from_str(l).ok()).collect();
            (None, events)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::allowlist::validate;

    fn req(args: &[&str], ws: &std::path::Path) -> ExecRequest {
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        ExecRequest {
            validated: validate(&argv).unwrap(),
            dirs: SessionDirs {
                root: ws.to_path_buf(),
                workspace: ws.to_path_buf(),
                logs: ws.join("logs"),
                cache: ws.join("cache"),
            },
            device: Some("web:1".into()),
            timeout: Duration::from_secs(5),
        }
    }

    #[test]
    fn 注入的全局参数在子命令前面() {
        let r = req(&["fetch", "--interactive"], std::path::Path::new("/w"));
        let argv = build_argv(&r, &[]);
        let cmd_at = argv.iter().position(|a| a == "fetch").unwrap();
        for injected in ["--json", "--log", "--cache", "--current-dir", "--copilot=false", "-d"] {
            let at = argv.iter().position(|a| a == injected).unwrap();
            assert!(at < cmd_at, "{injected} 必须在子命令前（P-44：放后面会被当成子命令的参数）");
        }
        assert_eq!(argv[argv.len() - 2..], ["fetch", "--interactive"]);
    }

    #[test]
    fn 调用方给了log就不再注入() {
        let r = req(&["steps", "等待 [1s]", "--log", "logs/scan"], std::path::Path::new("/w"));
        let resolved = resolve_paths(&r).unwrap();
        let argv = build_argv(&r, &resolved);
        assert_eq!(argv.iter().filter(|a| *a == "--log").count(), 1, "两个 --log 会让 clap 取到错的那个: {argv:?}");
        assert!(argv.contains(&"/w/logs/scan".to_string()), "{argv:?}");
    }

    #[test]
    fn copilot强制关掉() {
        // L1 是零 LLM 面：节点 config.toml 里配了 [ai] 也不许在命令层触发（INV-16 延伸条款）
        let r = req(&["run", "a.tks"], std::path::Path::new("/w"));
        assert!(build_argv(&r, &[]).contains(&"--copilot=false".to_string()));
    }

    #[test]
    fn 宿主路径解析进工作区() {
        let ws = std::path::Path::new("/w");
        let r = req(&["ocr", "--image", "shots/a.png"], ws);
        let resolved = resolve_paths(&r).unwrap();
        assert_eq!(resolved, vec![(2, PathBuf::from("/w/shots/a.png"))]);
        let argv = build_argv(&r, &resolved);
        assert!(argv.contains(&"/w/shots/a.png".to_string()));
    }

    #[test]
    fn 越界的路径参数被拒() {
        let ws = std::path::Path::new("/w");
        for bad in ["/etc/passwd", "../../etc/passwd"] {
            let r = req(&["ocr", "--image", bad], ws);
            let e = resolve_paths(&r).unwrap_err();
            assert!(e.contains("越界"), "{bad}: {e}");
        }
    }

    #[test]
    fn 单行json与事件流分开解析() {
        let (j, e) = parse_stdout("{\"success\":true}");
        assert!(j.is_some() && e.is_empty());
        let (j, e) = parse_stdout("{\"a\":1}\n{\"b\":2}\n");
        assert!(j.is_none());
        assert_eq!(e.len(), 2);
        // 解析不出来也不能吞：原文由调用方自己看（P-46）
        let (j, e) = parse_stdout("not json");
        assert!(j.is_none() && e.is_empty());
    }

    #[tokio::test]
    async fn 真的起一个子进程并收结果() {
        // 用 /bin/echo 代替 tke：这层测的是"起进程—收管道—记时"，与具体命令无关
        let ws = std::env::temp_dir();
        let mut r = req(&["fetch"], &ws);
        r.device = None;
        let out = run(std::path::Path::new("/bin/echo"), &r).await.unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(!out.timed_out);
        assert!(out.stdout_raw.contains("fetch"));
        assert!(out.timing.total_ms >= out.timing.run_ms);
    }

    #[tokio::test]
    async fn 超时会被杀掉而不是挂着() {
        let mut cmd = tokio::process::Command::new("/bin/sleep");
        cmd.arg("30").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let (code, timed_out, ..) = spawn_and_wait(cmd, Duration::from_millis(150)).await.unwrap();
        assert!(timed_out, "挂着的进程会一直占着设备，必须杀");
        assert_eq!(code, None);
    }
}
