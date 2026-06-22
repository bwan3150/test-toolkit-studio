// 【统一中断】进程级 Ctrl+C 中断标志，跨所有层（CLI/工作流/ScriptRunner/agent 各阶段）统一查询。
//
// 放在 utils 层（最底层共享），让 ScriptRunner 的逐步回放也能在每步前查中断、及时停下——
// 否则一段 16 步的回放要全跑完才轮到上层检查点，Ctrl+C 看起来"没反应"。
//
// 一次性安装 Ctrl+C 监听，首次按下置标志；各处循环在检查点查 `aborted()` 优雅停（出当前步骤后停，不硬杀）。
// 这也是后续「打断 Agent + 注入用户当前步骤指导」的地基（届时把"置标志"换成"置暂停+等用户输入"即可）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static ABORTED: AtomicBool = AtomicBool::new(false);
static INSTALL: Once = Once::new();

/// 安装一次性 Ctrl+C 监听（重复调用只生效一次）。在一次运行开始时调用。
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

/// 是否已请求中断（各处循环在检查点查询）
pub fn aborted() -> bool {
    ABORTED.load(Ordering::Relaxed)
}
