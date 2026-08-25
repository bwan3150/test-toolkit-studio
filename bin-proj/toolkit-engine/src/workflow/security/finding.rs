//! 安全域跨角色共用的 Finding 模型（prober 产出、analyst 复核、reporter 渲染都用它）。
//!
//! 与 recon 的轻量 `ReconFinding` 区别：Finding 是「进报告候选」，带类别、软/硬证据标记、
//! 复现命令、关联证据路径。**软硬分字段**是 INV-13/ADR-0009 的硬要求——
//! `confirmed=false`（疑似·待复现）不进独立漏洞报告，只在全局清单标疑似。

use serde::Serialize;

pub use super::recon::Severity;
use super::evidence::EvidenceRef;

/// 漏洞类别（与报告 spec 的维度一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Auth,
    DataExposure,
    Injection,
    Transport,
    Config,
    Info,
}

impl Category {
    /// 从字符串宽松解析（LLM 传进来的 category）。
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "auth" | "authn" | "authz" => Category::Auth,
            "data-exposure" | "data" | "exposure" | "leak" => Category::DataExposure,
            "injection" | "inject" | "sqli" | "xss" | "ssrf" => Category::Injection,
            "transport" | "tls" | "ssl" => Category::Transport,
            "config" | "configuration" | "misconfig" => Category::Config,
            _ => Category::Info,
        }
    }
}

/// 一条候选发现。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub category: Category,
    pub title: String,
    pub detail: String,
    /// 硬证据（已黑盒复现）= true；疑似·待复现 = false。
    #[serde(default)]
    pub confirmed: bool,
    /// 可直接执行的复现命令（curl 等），可空。
    #[serde(default)]
    pub repro: Option<String>,
    /// 关联证据（evidence/step_NNN_*）的相对路径序号。
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

impl Finding {
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        category: Category,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            category,
            title: title.into(),
            detail: detail.into(),
            confirmed: false,
            repro: None,
            evidence: Vec::new(),
        }
    }
}

/// prober 一次运行的产出。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ProbeReport {
    pub target: String,
    pub mode: String,
    pub findings: Vec<Finding>,
    /// prober 自己的收尾总结（一句话）。
    pub summary: String,
    /// 实际用掉的推理轮数（工具调用步数）。
    pub steps: usize,
    /// 这一段烧了多少 token（平台按它计费，ADR-0023 D3）
    pub usage: super::usage::Usage,
}
