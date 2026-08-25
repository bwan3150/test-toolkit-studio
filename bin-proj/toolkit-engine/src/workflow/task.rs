//! 测试任务标记（跨轨共享）：`<taskdir>/task.json` 记这个任务是哪条测试轨（ui / security）+ 目标/强度。
//!
//! 「领域是数据，不是命令」（ADR-0021）：task/steps/report 是**领域无关**的生命周期层；
//! 一个任务是 UI 测试还是安全测试，记在这个标记里，`tke report <dir>` 据此分派到不同报告生成，
//! 而不是拆成 `tke ui report` / `tke security report` 两条命令（那是 ADR-0020，已被本设计取代）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Result, TkeError};

/// 标记文件名（落在任务目录根）。
pub const MARKER: &str = "task.json";

/// 任务元信息。`kind` 是唯一必需字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    /// 测试轨：`"ui"`（设备/UI）/ `"security"`（安全）。未来新轨照加。
    pub kind: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

impl TaskMeta {
    pub fn new(kind: impl Into<String>) -> Self {
        Self { kind: kind.into(), target: None, mode: None }
    }
    pub fn is_security(&self) -> bool {
        self.kind.eq_ignore_ascii_case("security")
    }
}

/// 写 `<dir>/task.json`（目录不存在则建）。
pub fn write_marker(dir: &Path, meta: &TaskMeta) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(TkeError::IoError)?;
    let json = serde_json::to_string_pretty(meta).map_err(TkeError::JsonError)?;
    std::fs::write(dir.join(MARKER), json).map_err(TkeError::IoError)?;
    Ok(())
}

/// 读 `<dir>/task.json`；没有 / 坏了都返回 None（调用方自行兜底）。
pub fn read_marker(dir: &Path) -> Option<TaskMeta> {
    let s = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    serde_json::from_str(&s).ok()
}

/// 分辨任务是不是安全测试：优先看标记；没标记则看有没有 `findings.json`（安全报告的产物特征）。
/// —— 让 `tke report <dir>` 在没显式标记时也能正确分派（skill 只丢 findings 也认得）。
pub fn is_security_task(dir: &Path) -> bool {
    if let Some(m) = read_marker(dir) {
        return m.is_security();
    }
    dir.join("findings.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_detect() {
        let tmp = std::env::temp_dir().join(format!("tke-task-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut m = TaskMeta::new("security");
        m.target = Some("https://t.example/".into());
        m.mode = Some("safe".into());
        write_marker(&tmp, &m).unwrap();

        let got = read_marker(&tmp).unwrap();
        assert_eq!(got.kind, "security");
        assert!(got.is_security());
        assert!(is_security_task(&tmp));

        // 无标记但有 findings.json → 也判定为 security
        let tmp2 = std::env::temp_dir().join(format!("tke-task2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp2);
        std::fs::create_dir_all(&tmp2).unwrap();
        std::fs::write(tmp2.join("findings.json"), "{}").unwrap();
        assert!(is_security_task(&tmp2));

        // 啥也没有 → 非 security（当 UI）
        let tmp3 = std::env::temp_dir().join(format!("tke-task3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp3);
        std::fs::create_dir_all(&tmp3).unwrap();
        assert!(!is_security_task(&tmp3));

        for d in [tmp, tmp2, tmp3] { let _ = std::fs::remove_dir_all(&d); }
    }
}
