//! tke security —— 探索式黑盒安全测试（ADR-0019）。
//!
//! 这是 tke 的**第二个 agent 领域**：harness 操作屏幕，security 打 HTTP、看响应、验漏洞。
//! 本模块是安全域的**业务逻辑层**（CLI 参数翻译在 `cli/security/`）。
//!
//! 分层（照 device 那套，见 ADR-0019 决策 2）：
//!   primitive（`http`/`recon`，确定性、可脚本、落证据、无 AI）
//!     ⇄ AI 工具（薄封装，P2）⇄ `tke security` 编排（P2）
//!
//! 铁律：
//!   - INV-14 每个探测都过 `evidence`，无无证据的第二条路。
//!   - INV-13 漏洞判定必须黑盒复现（P2 的 analyst 闸门；本层只提供可复现的探测原语）。
//!
//! P1 只落**侦察底座**：HTTP 引擎（trait + 真实 ureq + fake 可测）+ 证据落盘 + 首批 recon 检查。

pub mod analyst;
pub mod evidence;
pub mod finding;
pub mod http;
pub mod prompt;
pub mod orchestrator;
pub mod prober;
pub mod recon;
pub mod report;
pub mod usage;
