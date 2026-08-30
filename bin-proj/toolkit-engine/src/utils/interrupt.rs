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

/// 安装 Ctrl+C 监听（重复调用只生效一次）。在一次运行开始时调用。
///
/// ⚠️ 「优雅停止」只对**正在跑步骤**成立：置个标志，循环走到检查点自己停。
/// 但有两种情况下它就是个坑，都得能立刻退出——
///   1. **在等用户输入时**（`继续？[y/N]`）：主线程阻塞在 `read_line`，
///      没有任何循环会去查标志。用户按十次 Ctrl+C 都"没反应",直到他敲回车
///      才轮到检查点——按取消键还得再按回车,这不叫中断
///   2. **按第二次**：第一次没停下来，说明当前步骤要么很长要么卡住了。
///      这时用户要的是"马上给我停"，通用惯例也是第二次硬退
pub fn install() {
    INSTALL.call_once(|| {
        tokio::spawn(async {
            loop {
                if tokio::signal::ctrl_c().await.is_err() {
                    break;
                }
                // swap：第一次拿到 false（转入优雅停），第二次拿到 true（硬退）
                if AT_PROMPT.load(Ordering::Relaxed) || ABORTED.swap(true, Ordering::Relaxed) {
                    eprintln!("\n已取消");
                    // 130 = 128 + SIGINT，shell 的通用约定
                    std::process::exit(130);
                }
                // 两句话说完：在干什么、想更快怎么办。**不解释为什么要等**
                // （INV-18：这一行会不会改变他下一步做什么）
                eprintln!("\n安全中断中…  再按一次立即中断");
            }
        });
    });
}

// —— 等用户输入期间：Ctrl+C 立即退出（见 install 的说明）——
static AT_PROMPT: AtomicBool = AtomicBool::new(false);

/// 包住**阻塞读 stdin** 的那一段。在它的生命周期内 Ctrl+C 直接退出进程：
/// 这时没有任何"当前步骤"需要收尾，等标志被查到反而要用户再敲一次回车。
///
/// ```ignore
/// let _g = interrupt::prompting();   // 离开作用域自动恢复
/// std::io::stdin().read_line(&mut line)
/// ```
pub fn prompting() -> PromptGuard {
    AT_PROMPT.store(true, Ordering::Relaxed);
    PromptGuard
}

/// 见 [`prompting`]。用 Drop 而不是手工配对的 `end()`——中间那段常有 `return`，
/// 漏掉一次就会让后续的 Ctrl+C 变成硬退，那是另一种难查的怪事
pub struct PromptGuard;

impl Drop for PromptGuard {
    fn drop(&mut self) {
        AT_PROMPT.store(false, Ordering::Relaxed);
    }
}

/// 是否已请求中断（各处循环在检查点查询）
pub fn aborted() -> bool {
    ABORTED.load(Ordering::Relaxed)
}

// —— 软停（Esc）：请求中断「当前正在执行的动作」（滚动查找/回放等长动作轮询此标志立即停下）——
// 与 aborted 的区别：这是「停下当前动作、转入暂停等用户指导」，不是终止整个流程。
static PAUSE: AtomicBool = AtomicBool::new(false);

/// 请求软停当前动作（TUI 按 Esc 时调）；长动作内部轮询 `pause_requested()` 立即停。
pub fn request_pause() {
    PAUSE.store(true, Ordering::Relaxed);
}

/// 当前动作是否被请求软停（滚动查找/回放等长循环每次迭代查询）
pub fn pause_requested() -> bool {
    PAUSE.load(Ordering::Relaxed)
}

/// 清除软停标志（引擎进入暂停处理、或恢复运行时调）
pub fn clear_pause() {
    PAUSE.store(false, Ordering::Relaxed);
}
