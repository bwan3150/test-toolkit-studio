//! 证据落盘：安全域每个探测的 请求+响应 都写进任务目录（INV-14，无无证据的第二条路）。
//!
//! 组织方式沿用 report.rs 的「一个任务一份目录、步骤连续编号」思路：
//!   evidence/step_001_req.txt  ← 请求原文
//!   evidence/step_001_resp.txt ← 响应原文（状态行 + 头 + 体，UTF-8 有损）
//!
//! 这是 analyst 判定漏洞（INV-13）和报告「复现/证据」段的原料。返回的相对路径直接进 findings.json。

use std::path::{Path, PathBuf};

use crate::{Result, TkeError};
use super::http::{HttpRequest, HttpResponse};

/// 一次探测落盘后的引用（相对任务目录），进报告与 findings.json。
#[derive(Debug, Clone)]
pub struct EvidenceRef {
    pub seq: usize,
    pub request: PathBuf,
    pub response: PathBuf,
}

/// 任务级证据目录：反复调用、步骤连续编号。
pub struct EvidenceDir {
    dir: PathBuf,
    next: usize,
}

impl EvidenceDir {
    /// 在 `task_dir/evidence/` 下开一个证据目录。
    ///
    /// **续写而非覆盖**：编号从目录里已有的最大 `step_NNN` 之后接着排——
    /// 一次评估往往是多个进程调用（http + 各 recon verb）共用同一个 `--log`，
    /// 每次都从 001 重来会互相覆盖（承「一个任务一份、反复调用续写、连续编号」原则）。
    pub fn new(task_dir: &Path) -> Result<Self> {
        let dir = task_dir.join("evidence");
        std::fs::create_dir_all(&dir).map_err(TkeError::IoError)?;
        let next = highest_seq(&dir) + 1;
        Ok(Self { dir, next })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 记录一对 请求/响应，返回相对 `task_dir` 的路径引用。
    pub fn record(&mut self, req: &HttpRequest, resp: &HttpResponse) -> Result<EvidenceRef> {
        let seq = self.next;
        self.next += 1;

        let req_name = format!("step_{seq:03}_req.txt");
        let resp_name = format!("step_{seq:03}_resp.txt");

        std::fs::write(self.dir.join(&req_name), render_request(req)).map_err(TkeError::IoError)?;
        std::fs::write(self.dir.join(&resp_name), render_response(resp)).map_err(TkeError::IoError)?;

        Ok(EvidenceRef {
            seq,
            request: PathBuf::from("evidence").join(&req_name),
            response: PathBuf::from("evidence").join(&resp_name),
        })
    }
}

/// 扫目录里已有的 `step_NNN_*.txt`，返回最大的 NNN（没有则 0）。
fn highest_seq(dir: &Path) -> usize {
    let mut max = 0;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // 形如 step_007_req.txt / step_007_resp.txt
        if let Some(rest) = name.strip_prefix("step_") {
            if let Some(num) = rest.split('_').next() {
                if let Ok(n) = num.parse::<usize>() {
                    max = max.max(n);
                }
            }
        }
    }
    max
}

/// 请求原文：`METHOD URL` + 头 + 空行 + 体。
fn render_request(req: &HttpRequest) -> String {
    let mut s = format!("{} {}\n", req.method, req.url);
    for (k, v) in &req.headers {
        s.push_str(&format!("{k}: {v}\n"));
    }
    if let Some(b) = &req.body {
        s.push('\n');
        s.push_str(&String::from_utf8_lossy(b));
    }
    s
}

/// 响应原文：`HTTP <status>` + 头 + 空行 + 体（截断则标注）。
fn render_response(resp: &HttpResponse) -> String {
    let mut s = format!("HTTP {}\n", resp.status);
    for (k, v) in &resp.headers {
        s.push_str(&format!("{k}: {v}\n"));
    }
    s.push('\n');
    s.push_str(&resp.text());
    if resp.truncated {
        s.push_str(&format!("\n\n[响应体超过 {} 字节，已截断]", super::http::MAX_BODY));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_pair_with_sequential_numbering() {
        let tmp = std::env::temp_dir().join(format!("tke-sec-evi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut evi = EvidenceDir::new(&tmp).unwrap();

        let req = HttpRequest::new("GET", "https://t.example/a");
        let resp = HttpResponse {
            status: 200,
            headers: vec![("Server".into(), "nginx".into())],
            body: b"hello".to_vec(),
            truncated: false,
            elapsed_ms: 3,
        };
        let r1 = evi.record(&req, &resp).unwrap();
        let r2 = evi.record(&req, &resp).unwrap();

        assert_eq!(r1.seq, 1);
        assert_eq!(r2.seq, 2);
        assert!(evi.dir().join("step_001_req.txt").exists());
        let resp_txt = std::fs::read_to_string(evi.dir().join("step_001_resp.txt")).unwrap();
        assert!(resp_txt.starts_with("HTTP 200"));
        assert!(resp_txt.contains("Server: nginx"));
        assert!(resp_txt.contains("hello"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resumes_numbering_across_reopen() {
        // 模拟「多个进程调用共用同一 --log」：第二次开 EvidenceDir 应从 step_002 续，而非覆盖 step_001
        let tmp = std::env::temp_dir().join(format!("tke-sec-evi-resume-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let req = HttpRequest::new("GET", "https://t.example/");
        let resp = HttpResponse { status: 200, headers: vec![], body: b"x".to_vec(), truncated: false, elapsed_ms: 1 };

        let r1 = EvidenceDir::new(&tmp).unwrap().record(&req, &resp).unwrap();
        assert_eq!(r1.seq, 1);
        let r2 = EvidenceDir::new(&tmp).unwrap().record(&req, &resp).unwrap();
        assert_eq!(r2.seq, 2, "重开应续写而非从 1 覆盖");
        assert!(tmp.join("evidence/step_001_req.txt").exists());
        assert!(tmp.join("evidence/step_002_req.txt").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
