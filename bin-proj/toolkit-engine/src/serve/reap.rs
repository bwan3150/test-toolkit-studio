// 【收尸】上一代节点留下的任务子进程。
//
// 节点被 `kill -9` 时没有任何机会做收尾，它起的 `tke harness` 子进程会活下来 ——
// 而那些进程**还握着设备**：继续 adb 点点点、继续写会话目录。表现就是节点重启后
// 设备行为诡异，而新节点对此一无所知（实测一次清掉过 3 个）。
//
// 两条防线，缺一不可：
//
//   1. `PR_SET_PDEATHSIG`（本模块的 `arm_parent_death`，仅 Linux）—— **即时**：
//      父进程一死内核就给子进程发 SIGTERM，`kill -9` 也拦不住这条。
//      macOS/Windows 没有等价物。
//   2. **启动时收尸**（本模块的 `reap_previous`，三平台通用）—— 补上第 1 条的缺口，
//      也补上"父进程还活着但会话已经失联"这类情况。
//
// 只有 1 会漏掉非 Linux；只有 2 会留下一个"节点已经死了、孤儿还在操作设备"的窗口。

use std::path::Path;

/// 子进程 pid 落在会话目录下的这个文件里
pub const PID_FILE: &str = "task.pid";

/// 给即将 spawn 的子进程挂上「父死我也死」（Linux）。
///
/// 其它平台是空实现 —— 那边靠 `reap_previous` 兜底。
#[cfg(target_os = "linux")]
pub fn arm_parent_death(cmd: &mut tokio::process::Command) {
    // tokio::process::Command 自带 pre_exec（unix），不需要 std 的扩展 trait
    // SAFETY: pre_exec 在 fork 之后、exec 之前跑，此时只允许 async-signal-safe 调用。
    // prctl 是系统调用，满足这个要求；这里也不碰任何堆分配或锁。
    unsafe {
        cmd.pre_exec(|| {
            // 父进程死时给自己发 SIGTERM。子进程是 tke，收到会正常收尾。
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // 竞态：如果父进程在 fork 之后、prctl 之前就已经死了，PDEATHSIG 永远不会触发。
            // 再确认一次自己还有父亲 —— getppid()==1 说明已经被 init 收养了。
            if libc::getppid() == 1 {
                libc::_exit(0);
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub fn arm_parent_death(_cmd: &mut tokio::process::Command) {}

/// 记下这个会话的任务子进程 pid。写失败只记日志 —— 收尸是兜底能力，
/// 不该因为写不了一个文件就让任务起不来。
pub fn note_pid(session_root: &Path, pid: u32) {
    let f = session_root.join(PID_FILE);
    if let Err(e) = std::fs::write(&f, pid.to_string()) {
        tracing::warn!(target: "tke::serve", "记录任务 pid 失败（{:?}）：{}", f, e);
    }
}

/// 任务正常结束，把 pid 文件删掉 —— 留着会让下次启动去查一个早就没了的 pid。
pub fn clear_pid(session_root: &Path) {
    let _ = std::fs::remove_file(session_root.join(PID_FILE));
}

/// 扫 `<root>/sessions/*/task.pid`，把上一代还活着的任务子进程杀掉。
///
/// 返回杀掉的个数（给启动日志用）。
pub fn reap_previous(root: &Path) -> usize {
    let dir = root.join("sessions");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0; // 第一次启动，没有这个目录，正常
    };
    let mut killed = 0usize;
    for e in entries.flatten() {
        let session_root = e.path();
        let f = session_root.join(PID_FILE);
        let Ok(txt) = std::fs::read_to_string(&f) else {
            continue;
        };
        let Ok(pid) = txt.trim().parse::<u32>() else {
            let _ = std::fs::remove_file(&f);
            continue;
        };
        let sid = session_root.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if is_our_task(pid, sid) {
            kill(pid);
            killed += 1;
            tracing::info!(target: "tke::serve", "清掉上一代遗留的任务进程 pid={} session={}", pid, sid);
        }
        let _ = std::fs::remove_file(&f);
    }
    killed
}

/// 这个 pid 现在**确实还是那个任务**吗。
///
/// **不能只看"pid 还在"就杀** —— pid 会被复用，那样会误杀一个毫不相干的进程。
/// 判据是命令行里同时含 `tke` 与这个 **session id**：session id 出现在子进程的
/// `--log` / `--cache` 路径里，是这一次任务独有的指纹。
fn is_our_task(pid: u32, sid: &str) -> bool {
    if sid.is_empty() {
        return false;
    }
    let cmdline = read_cmdline(pid);
    match cmdline {
        Some(c) => c.contains(sid) && c.contains("tke"),
        None => false, // 读不到就是没了（或不是我们能确认的），一律不杀
    }
}

#[cfg(target_os = "linux")]
fn read_cmdline(pid: u32) -> Option<String> {
    let raw = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    // /proc 里参数以 NUL 分隔
    Some(String::from_utf8_lossy(&raw).replace('\0', " "))
}

#[cfg(not(target_os = "linux"))]
fn read_cmdline(pid: u32) -> Option<String> {
    let out = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

#[cfg(unix)]
fn kill(pid: u32) {
    // 先礼后兵：SIGTERM 让它自己收尾（它是 tke，会关浏览器/停 App），
    // 给一点时间再 SIGKILL 兜底
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tke-reap-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sessions")).unwrap();
        d
    }

    #[test]
    fn 没有sessions目录时不炸() {
        let d = std::env::temp_dir().join(format!("tke-reap-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(reap_previous(&d), 0);
    }

    /// **pid 复用是这段代码唯一会造成伤害的方式**：pid 号还在，但那已经是别人的进程了。
    /// 判据必须是"命令行里有这个 session id"，不能是"pid 还在"。
    #[test]
    fn pid被复用时不误杀() {
        // 拿本进程当"被复用的 pid"：它活着，但命令行里不会有这个 session id
        assert!(!is_our_task(std::process::id(), "s6a91050d001"));
    }

    #[test]
    fn 空session_id一律不杀() {
        assert!(!is_our_task(std::process::id(), ""));
    }

    #[test]
    fn 认不出的pid不杀且清掉记录() {
        let d = tmp("stale");
        let s = d.join("sessions").join("s_dead");
        std::fs::create_dir_all(&s).unwrap();
        // 一个几乎不可能存在的 pid
        std::fs::write(s.join(PID_FILE), "4194303").unwrap();
        assert_eq!(reap_previous(&d), 0);
        // 记录要清掉 —— 留着的话每次启动都去查一个早就没了的 pid
        assert!(!s.join(PID_FILE).exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn pid文件写了能读回来也能清掉() {
        let d = tmp("io");
        let s = d.join("sessions").join("s1");
        std::fs::create_dir_all(&s).unwrap();
        note_pid(&s, 12345);
        assert_eq!(std::fs::read_to_string(s.join(PID_FILE)).unwrap(), "12345");
        clear_pid(&s);
        assert!(!s.join(PID_FILE).exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
