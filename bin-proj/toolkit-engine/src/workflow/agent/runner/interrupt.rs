// 【统一中断】进程级 Ctrl+C 中断标志，跨所有阶段（探索/诊断/验证/医生/重探）统一查询。
//
// 设计：一次性安装一个 Ctrl+C 监听任务，首次按下即置中断标志；各阶段的循环在自己的检查点
// 查 `aborted()` 并优雅停止（出当前步骤后停，不硬杀）。这是把 CLI 键盘交互提到进程级统一管理，
// 也是后续「打断 Agent + 注入用户当前步骤指导」的地基（届时把"置标志"换成"置暂停+等用户输入"即可）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static ABORTED: AtomicBool = AtomicBool::new(false);
static INSTALL: Once = Once::new();

/// 安装一次性的 Ctrl+C 监听（重复调用只生效一次）。在一次 harness 运行开始时调用。
pub fn install() {
    INSTALL.call_once(|| {
        tokio::spawn(async {
            if tokio::signal::ctrl_c().await.is_ok() {
                ABORTED.store(true, Ordering::Relaxed);
                eprintln!("\n⏹ 收到中断（Ctrl+C）——将在当前步骤结束后安全停止…");
            }
        });
    });
}

/// 是否已请求中断（各阶段循环在检查点查询）
pub fn aborted() -> bool {
    ABORTED.load(Ordering::Relaxed)
}
