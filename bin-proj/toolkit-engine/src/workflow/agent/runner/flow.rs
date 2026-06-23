// 主循环：perceive → decide → apply → record（只编排，具体能力委托各子模块）
// 产物经 RunArtifacts 统一保存：每个执行步存 screenshots/step_NNN + page/step_NNN，
// 与 tke run 同构；conversation.jsonl（AI 原始对话）由 transcript 写在同一运行目录。

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::engines::ocr::OcrSource;
use crate::{Fetcher, LlmReply, LlmSession, Result, RunArtifacts, StepResult, Workarea};

/// 终端着色：仅当 stderr 是 TTY 时输出 ANSI，管道/重定向为纯文本（与 tke run 一致）
pub(crate) fn paint(tty: bool, code: &str, s: &str) -> String {
    if tty {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

/// 页面签名哈希（用元素列表文本），检测"原地打转"用
fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// 从模型回复里抽出 JSON 对象（容忍前后多余文字 / ```json 围栏）
pub(crate) fn parse_desc_json(s: &str) -> Option<serde_json::Value> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    serde_json::from_str(s.get(start..=end)?).ok()
}

/// 动作行的友好显示：去掉 .tks 的 `[{ }]` 括号噪声，纯文本不上色（仅前面的 ✓/✗ 带色）。
/// 如 `点击 [{登录按钮}]` → `点击 登录按钮`、`定向滑动 [{640, 406}, 上, 406]` → `定向滑动 640, 406, 上, 406`
pub(crate) fn friendly(line: &str) -> String {
    line.replace("[{", "").replace("}]", "").replace(['[', ']', '{', '}'], "")
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 紧凑 token 显示：4443→4.4k、148693→149k、<1000 原样，避免大数字刷屏
pub(crate) fn fmt_tokens(n: i64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 100_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        format!("{}k", (n + 500) / 1000)
    }
}

/// 时长格式化（与 tke run 的 EventPrinter 对齐）：320ms / 3.7s / 1m12s
pub(crate) fn fmt_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m{:02}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

/// 取首个非空行并按字符数截断，避免模型长篇刷屏
pub(crate) fn brief(s: &str, max: usize) -> String {
    let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.chars().count() > max {
        let head: String = line.chars().take(max).collect();
        format!("{}…", head)
    } else {
        line.to_string()
    }
}

use crate::Platform;

/// 「即将执行」的人类可读预览：用 AI 选中元素的文字（执行前即有），让 CLI 先于设备显示，
/// 用户能对上 agent 这步要点啥。返回 None = 非设备动作、不预览。
fn preview_action(action: &AgentAction, elements: &[crate::UIElement]) -> Option<String> {
    // 用 auto_name（与 apply 落库同源）拼出和最终 .tks 行一致的展示，如「点击 Products@410_57」
    let name = |id: usize| -> String {
        elements.get(id).map(execution::auto_name).unwrap_or_else(|| format!("元素#{}", id))
    };
    Some(match action {
        AgentAction::Launch { target, .. } => format!("启动 {}", brief(target, 60)),
        AgentAction::Close { target } => format!("关闭 {}", brief(target, 60)),
        AgentAction::Click { element_id, .. } => format!("点击 {}", name(*element_id)),
        AgentAction::Input { element_id, text, .. } => format!("输入 {} \"{}\"", name(*element_id), brief(text, 30)),
        AgentAction::LongPress { element_id, duration_ms, .. } => format!("按压 {} {}ms", name(*element_id), duration_ms),
        AgentAction::Clear { element_id, .. } => format!("清空 {}", name(*element_id)),
        AgentAction::Assert { element_id, exist, .. } => {
            format!("断言 {} {}", name(*element_id), if *exist { "存在" } else { "不存在" })
        }
        AgentAction::ClickVisual { .. } => "视觉点击（看图框选）".to_string(),
        AgentAction::SwipeDir { direction, .. } => format!("定向滑动 {}", dir_cn(direction)),
        AgentAction::SwipeToFind { target, direction } => format!("滚动查找 \"{}\", {}", brief(target, 30), dir_cn(direction)),
        AgentAction::SwipeElement { element_id, direction, .. } => format!("在 {} 上滑 {}", name(*element_id), dir_cn(direction)),
        AgentAction::Drag { from_id, to_id } => format!("拖 {} → {}", name(*from_id), name(*to_id)),
        AgentAction::PressKey { key } => format!("按键 {}", key.trim().to_uppercase()),
        AgentAction::Switch { target } => format!("切换 {}", brief(target, 40)),
        AgentAction::Back => "返回".to_string(),
        AgentAction::HideKeyboard => "隐藏键盘".to_string(),
        AgentAction::Wait { ms: Some(ms), .. } => format!("等待 {}ms", ms),
        AgentAction::Wait { element: Some(e), .. } => format!("等待元素 {}", brief(e, 30)),
        AgentAction::Wait { .. } => "等待".to_string(),
        _ => return None, // Finish/RequestScreenshot/AskUser/Rename 非设备动作，不在此预览
    })
}

/// 方向英文 → 中文（预览用）
fn dir_cn(d: &str) -> &str {
    match d {
        "up" => "上",
        "down" => "下",
        "left" => "左",
        "right" => "右",
        other => other,
    }
}

use super::super::execution;
use super::super::interaction::read_user_line;
use super::super::perception::{capture, match_known, render_element_list, Perceived};
use super::super::prompt::{render, PromptSet};
use super::super::tools::{parse_tool_call, AgentAction};
use super::super::transcript::Transcript;

/// 循环所需的只读上下文
pub struct DriveCtx<'a> {
    pub device: &'a str,
    pub element_path: &'a Path,
    pub workarea: &'a Workarea,
    pub fetcher: &'a Fetcher,
    pub artifacts: &'a RunArtifacts,
    /// OCR 增强来源（None=不跑 OCR，行为同此前）
    pub ocr: Option<&'a OcrSource>,
    pub max_rounds: usize,
    /// 提示词集合：运行时消息模板（每轮页面/各类提示/截图等）从这里取，可外部覆盖
    pub prompts: &'a PromptSet,
    /// 用户原始测试用例（finish 终点校验：对照原话需求判断是否真到对的地方）
    pub case: &'a str,
}

/// 循环结果
pub struct DriveOutcome {
    pub success: bool,
    pub reason: String,
    pub lines: Vec<String>,
    pub steps: Vec<StepResult>,
    pub rounds: usize,
    /// 本轮新增的元素名（人工审核用）
    pub created: Vec<String>,
    /// 本轮更新了描述的元素名（人工审核用）
    pub updated: Vec<String>,
    /// 是否被用户中断（Ctrl+C）
    pub aborted: bool,
    /// AI 在 finish 时给生成脚本起的简短文件名（不含扩展名）；None 则由上层兜底
    pub script_name: Option<String>,
    /// 本次发生的元素改名 (旧名, 新名)，供上层把前缀脚本里的引用同步改掉
    pub renames: Vec<(String, String)>,
    /// 每个落库步骤当时 AI 给的 comment（理由），与 lines 一一对应；供反思 agent 复盘"怎么走成这样"
    pub step_comments: Vec<String>,
}

/// 驱动探索循环
///
/// `spinner=true`：初次探索——开头转个「正在启动…」动画（首屏采集前的空窗）。
/// `spinner=false`：修复续接——复用同一循环从当前实时页面继续探索生成修正步骤，不转动画。
/// 注意：本函数**不再打印结果/元素库框**——汇总在 verify 全部跑完后由上层统一渲染，
///   以便把验证/修复阶段的 token 也算进总量。修复模式调用前请先 `sess.user(开场白)`。
pub async fn drive(
    sess: &mut LlmSession,
    txp: &mut Transcript,
    ctx: &DriveCtx<'_>,
    spinner: bool,
    round_prefix: &str,
) -> Result<DriveOutcome> {
    // 进入「探索 agent」作用域：本函数（首次探索 与 reexplore 活体重探都走它）产出的所有事件
    // 都归属 explorer。Drop 守卫保证任何退出路径都弹栈（被 doctor 调用时弹回 doctor 作用域）。
    let mut _ascope = txp.scoped("explorer");
    let tx = &mut *_ascope;
    let tty = std::io::stderr().is_terminal();

    // 用户中断（Ctrl+C）：查进程级统一中断标志（由 interrupt::install 在运行开始时安装监听），
    // 监听到则在下一个决策点优雅停止、照常出总结。

    // 启动加载动画：采集首屏前有数秒空窗，转个 spinner 让用户知道在跑（仅 TTY、且初次探索）
    let spin_stop = Arc::new(AtomicBool::new(false));
    let mut spin_handle = if spinner && tty {
        let stop = spin_stop.clone();
        Some(tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0usize;
            while !stop.load(Ordering::Relaxed) {
                eprint!("\r{} 正在启动…", frames[i % frames.len()]);
                let _ = std::io::stderr().flush();
                i += 1;
                tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            }
            eprint!("\r\x1b[K"); // 清除 spinner 行
            let _ = std::io::stderr().flush();
        }))
    } else {
        None
    };

    let mut aborted = false;
    let mut page_sigs: Vec<u64> = Vec::new(); // 各轮结构签名，检测原地打转
    let mut no_progress = 0usize; // 连续多少轮页面无变化（上一步操作没生效）
    let max_llm_calls = ctx.max_rounds * 4 + 10; // 含要图/反问等不前进的轮次
    let mut lines: Vec<String> = Vec::new();
    let mut step_comments: Vec<String> = Vec::new(); // 与 lines 对应：每步 AI 的理由(comment)
    let mut steps: Vec<StepResult> = Vec::new();
    let mut round = 0usize;
    let mut llm_calls = 0usize;
    let mut finish_rechecks = 0usize; // finish 终点校验被打回的次数（防无限）
    let mut finish: Option<(bool, String)> = None;
    let mut script_name: Option<String> = None; // AI 在 finish 时起的脚本名
    let mut renames: Vec<(String, String)> = Vec::new(); // 本次元素改名记录
    let mut loop_streak = 0usize; // 连续"同一操作且页面不变"的次数（真打转信号）
    let mut last_was_swipe = false; // 上一步是否是滑动（用于剔除"空滑"——回放会滑过头）
    let mut last_was_close = false; // 上一步是否是主动关闭/收尾（关 app/销毁会话）——下一轮空页面不催 launch
    // 本轮测试对元素库的变更（供结束时人工审核）
    let mut created: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new(); // 格式化的差异行
    let mut updated_names: Vec<String> = Vec::new(); // 去重用

    'outer: while round < ctx.max_rounds {
        if super::interrupt::aborted() {
            aborted = true;
            finish = Some((false, "已终止（用户中断 Ctrl+C）".to_string()));
            break 'outer;
        }
        round += 1;

        // 1) 采集页面
        //    web/iOS 冷启动时尚无会话，采集会失败——此时降级为"空页面 + 提示先 launch"，
        //    不中断循环；AI 调 launch 建会话后，下一轮采集即正常。
        let (p, perceive_err) = match capture(ctx.device, ctx.workarea, ctx.fetcher, ctx.ocr).await {
            Ok(p) => (p, None),
            Err(e) => {
                tx.log("perceive_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                (
                    Perceived {
                        elements: Vec::new(),
                        shot_path: ctx.workarea.screenshot_path(),
                        xml_path: ctx.workarea.ui_tree_path(),
                        ocr_filled: 0,
                        ocr_added: 0,
                        ocr_recognized: 0,
                        ocr_error: None,
                        tabs: Vec::new(),
                    },
                    Some(e.to_string()),
                )
            }
        };
        // 首屏采集完毕，停掉加载动画
        if let Some(h) = spin_handle.take() {
            spin_stop.store(true, Ordering::Relaxed);
            let _ = h.await;
        }

        // 元素库对照：**只在内部用于落库时复用库名**（避免重复造名/库膨胀），**不再展示给 AI**。
        // —— AI 每轮看到的是干净的纯页面元素，选择完全基于当前页面真实情况，不受历史库诱导（之前
        //    "已收录"标注会诱导 AI 偏向点对口名字的脏库元素、反复跳错页）。命名复用挪到落库时机械去重。
        let platform = Platform::from_device(Some(ctx.device));
        let known = match_known(&p.elements, platform, ctx.element_path);
        let n_known = known.iter().filter(|k| k.is_some()).count();
        let n_unknown = p.elements.len() - n_known;
        // 仅 name 的视图，供 apply 落库时复用库名（rename 后会就地更新）；AI 看不到
        let mut known_names: Vec<Option<String>> =
            known.iter().map(|k| k.as_ref().map(|h| h.name.clone())).collect();

        // 干净页面：渲染时传全 None，不标任何"已收录"——AI 只看纯元素列表
        let no_known = vec![None; p.elements.len()];
        let list_text = render_element_list(&p.elements, &no_known, &ctx.prompts.message("explorer", "element_tag"));

        // 结构签名：只用真实元素的结构文本（排除易变的 OCR 伪元素如时钟、坐标），
        // 用于判断"上一步操作有没有让页面变化"以及是否在多页间打转。
        let struct_sig = hash_str(
            &p.elements
                .iter()
                .filter(|e| e.class_name != "OcrText")
                .map(|e| e.to_ai_text())
                .collect::<Vec<_>>()
                .join("|"),
        );
        let unchanged = page_sigs.last() == Some(&struct_sig); // 与上一轮完全相同 → 上一步没生效
        let revisits = page_sigs.iter().rev().take(8).filter(|&&h| h == struct_sig).count();
        page_sigs.push(struct_sig);
        if unchanged {
            no_progress += 1;
        } else {
            no_progress = 0;
        }
        // 剔除"空滑"：上一步是滑动但页面纹丝未动（已到边界/没滚起来）——这种步无用，
        // 且回放时它往往会真的滚动，导致整段脚本滑过头到底部、错过目标元素。从脚本里删掉。
        if unchanged && last_was_swipe {
            if let Some(dropped) = lines.pop() {
                tx.log("prune_noop_swipe", serde_json::json!({ "round": round, "dropped": dropped }));
            }
        }

        // 真打转 = "反复做同一操作" 且 "页面始终不变" **两者都满足**。
        // 只看其一会误杀：同一滑动但页面在翻新内容=在前进；试不同元素但页面没变=在尝试。都不算打转。
        let repeated_same = lines.len() >= 2 && lines[lines.len() - 1] == lines[lines.len() - 2];
        if unchanged && repeated_same {
            loop_streak += 1;
        } else {
            loop_streak = 0;
        }
        const LOOP_ABORT: usize = 3;
        if loop_streak >= LOOP_ABORT && !p.elements.is_empty() {
            let act = lines.last().map(|l| friendly(l)).unwrap_or_default();
            tx.log("stuck_abort", serde_json::json!({ "round": round, "loop_streak": loop_streak, "kind": "same_action_no_change", "action": act }));
            eprintln!("{}", paint(tty, "31", &format!("  反复执行同一无效操作「{}」且页面始终不变，自动停止", act)));
            finish = Some((false, format!("反复执行同一无效操作「{}」、自动停止", act)));
            break 'outer;
        }
        // 兜底止损：页面极长时间完全不前进（哪怕一直在试不同元素），最终也停（措辞不叫"重复"）。
        // 阈值放宽，给"逐步在正确道路上"的尝试留足空间。
        const NO_PROGRESS_BACKSTOP: usize = 12;
        if no_progress >= NO_PROGRESS_BACKSTOP && !p.elements.is_empty() {
            tx.log("stuck_abort", serde_json::json!({ "round": round, "no_progress": no_progress, "kind": "no_progress" }));
            eprintln!(
                "{}",
                paint(tty, "31", &format!("  连续 {} 轮页面始终无前进，自动停止（避免空烧 token）", no_progress))
            );
            finish = Some((false, format!("连续 {} 轮页面无前进、自动停止", no_progress)));
            break 'outer;
        }
        let stuck = (no_progress >= 1 || revisits >= 2) && !p.elements.is_empty();

        tx.log(
            "page_snapshot",
            serde_json::json!({
                "round": round,
                "element_count": p.elements.len(),
                "known": n_known,
                "unknown": n_unknown,
                "revisits": revisits,
                "tab_count": p.tabs.len(),
                "tabs": p.tabs.iter().map(|t| serde_json::json!({
                    "index": t.index, "title": t.title, "url": t.url, "active": t.active
                })).collect::<Vec<_>>(),
                "elements": list_text.clone(),
                "xml": p.xml_path.to_string_lossy(),
                "perceive_error": perceive_err.clone(),
            }),
        );
        // 提示（卡住/无变化/打转）：模板无前导换行，这里统一加 \n 作分隔
        let hint = if perceive_err.is_some() {
            // 区分"冷启动从没 launch"和"刚主动收尾关闭"——后者不催 launch，提示可直接 finish，
            // 否则会逼着 AI 在已完成任务后重新打开网址，把干净的 finish(success=true) 搅黄。
            let key = if last_was_close { "hint_after_close" } else { "hint_perceive_error" };
            format!("\n{}", ctx.prompts.message("explorer", key))
        } else if no_progress >= 1 {
            format!("\n{}", render(&ctx.prompts.message("explorer", "hint_no_progress"), &[("n", &no_progress.to_string())]))
        } else if revisits >= 2 {
            format!("\n{}", ctx.prompts.message("explorer", "hint_revisits"))
        } else {
            String::new()
        };
        // 标签页信息（web 多标签时），人和 AI 对称可见
        let tabs_text = crate::format_tabs(&p.tabs);
        let tabs_block = if tabs_text.is_empty() { String::new() } else { format!("{}\n", tabs_text) };
        let round_s = round.to_string();
        // 卡得更死（连续 2 次没变 / 多页打转）→ 进入**纯视觉**：元素列表已经帮不上忙了，
        // 只发当前截图、不再带元素/OCR，让 AI 直接看图用 click_visual 决策（避免被无效元素继续误导）。
        let go_visual = (no_progress >= 2 || revisits >= 3) && perceive_err.is_none();
        // 渲染每轮要发给 AI 的页面消息（含框架/hint）——同时记进 conversation.json，供提示词调优复盘
        let page_msg = render(
            &ctx.prompts.message("explorer", "page_round"),
            &[("tabs", &tabs_block), ("round", &round_s), ("elements", &list_text), ("hint", &hint)],
        );
        if go_visual {
            let prompt = render(
                &ctx.prompts.message("explorer", "page_round_visual"),
                &[("tabs", &tabs_block), ("round", &round_s), ("hint", &hint)],
            );
            tx.log("llm_message", serde_json::json!({ "round": round, "content": prompt, "image": p.shot_path.to_string_lossy() }));
            if let Err(e) = sess.user_with_image(&prompt, &p.shot_path) {
                // 截图失败兜底：退回发元素列表
                tx.log("auto_screenshot_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                tx.log("llm_message", serde_json::json!({ "round": round, "content": page_msg }));
                sess.user_page(page_msg);
            } else {
                tx.log("auto_screenshot", serde_json::json!({ "round": round }));
            }
        } else {
            tx.log("llm_message", serde_json::json!({ "round": round, "content": page_msg }));
            sess.user_page(page_msg);
        }
        // 轮与轮之间空一行，分隔更清楚
        eprintln!();
        // 轮次头：左侧"第 N 轮"做锚点，右侧次要统计(已知/未知/OCR/标签页)整体淡色，
        // 不与下面的「思考 + 动作」主内容抢眼。OCR/标签页有值才显示。
        // 展示 AI 实际看到的页面构成——不再显示"已知/未知"（每轮都把未标注元素给 AI 看，
        // 已知/未知对 AI 判断没意义、只会误导）。结构页面元素 + OCR 元素 +（web）标签页数，
        // 让人清楚 agent 看到了啥、据什么判断。
        let ocr_n = p.ocr_added;
        let page_n = p.elements.len().saturating_sub(ocr_n);
        let mut stat = vec![format!("{} 页面元素", page_n)];
        // OCR 状态**如实**展示，绝不用一个静默的 0 掩盖问题——区分四种：
        //   没开(不显示) / 接口报错(下面红字单列) / 识别N并入M新增K / 真识别 0。
        if ctx.ocr.is_some() {
            if p.ocr_error.is_some() {
                stat.push("OCR✗报错".to_string());
            } else {
                // recognized=识别到的文字总数(>0 即 OCR 在工作)；并入=回填进已有元素；新增=独立伪元素。
                stat.push(format!("OCR识别{}（并入{}·新增{}）", p.ocr_recognized, p.ocr_filled, p.ocr_added));
            }
        }
        if !p.tabs.is_empty() {
            stat.push(format!("{} 标签页", p.tabs.len()));
        }
        let notready = if perceive_err.is_some() {
            if last_was_close {
                paint(tty, "2", "  · 会话已关闭（收尾）")
            } else {
                paint(tty, "33", "  · 页面未就绪，待 launch")
            }
        } else {
            String::new()
        };
        eprintln!(
            "{}  {}{}",
            paint(tty, "2", &format!("{}第 {} 轮", round_prefix, round)),
            paint(tty, "2", &stat.join(" · ")),
            notready
        );
        // OCR 接口报错——单独红字一行 + 记日志，绝不让它被"0"掩盖。
        if let Some(err) = &p.ocr_error {
            eprintln!("  {}", paint(tty, "31", &format!("OCR 接口报错：{}", brief(err, 120))));
            tx.log("ocr_error", serde_json::json!({ "round": round, "error": err }));
        }
        // 卡住提示：淡黄一行，不用 emoji（与 ✓/✗ 状态色区分开）
        if stuck {
            let msg = if no_progress >= 2 || revisits >= 3 {
                "  上一步没生效/原地打转，已附截图让 AI 看图"
            } else if no_progress >= 1 {
                "  上一步操作后页面无变化，已提示 AI 换做法"
            } else {
                "  在多页间打转，已提示 AI 换路径"
            };
            eprintln!("{}", paint(tty, "33", msg));
        }

        // 2) 内层：持续问 AI，直到产生一个"改变页面的动作"或结束
        loop {
            if super::interrupt::aborted() {
                aborted = true;
                finish = Some((false, "已终止（用户中断 Ctrl+C）".to_string()));
                break 'outer;
            }
            if llm_calls >= max_llm_calls {
                finish = Some((false, format!("达到 LLM 调用上限({})，强制结束", max_llm_calls)));
                break 'outer;
            }
            llm_calls += 1;
            tx.log("llm_request", serde_json::json!({ "round": round, "seq": llm_calls }));

            let reply = match sess.next().await {
                Ok(r) => r,
                Err(e) => {
                    tx.log("llm_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                    return Err(e);
                }
            };

            // 本次 LLM 调用的 token 用量（上行/输入、下行/输出），着色、显示在 comment 之后
            let (pt, ct) = sess.last_usage();
            let toks = paint(tty, "2", &format!("↑{} ↓{}", fmt_tokens(pt), fmt_tokens(ct)));
            tx.log("llm_usage", serde_json::json!({ "round": round, "seq": llm_calls, "prompt_tokens": pt, "completion_tokens": ct }));

            let (thinking, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 200);
                    let say = if b.is_empty() { "（未调用工具，已提示其用工具或 finish）".to_string() } else { b };
                    eprintln!("  {}  {}", say, toks);
                    tx.log("llm_text", serde_json::json!({ "round": round, "content": t }));
                    let m = ctx.prompts.message("explorer", "nudge_use_tool");
                    tx.log("llm_message", serde_json::json!({ "round": round, "content": m.clone() }));
                    sess.user(m);
                    continue;
                }
                LlmReply::ToolCalls { text, calls } => (text, calls),
            };

            // 协议要求：本轮所有 tool_call 都要有 tool_result。仅执行第一个，其余回执"已忽略"
            let primary = calls[0].clone();
            for extra in &calls[1..] {
                sess.tool_result(extra.call_id.as_str(), "已忽略：每轮仅处理第一个工具调用");
            }

            let (action, comment) = match parse_tool_call(&primary) {
                Ok(v) => v,
                Err(e) => {
                    tx.log("tool_parse_error", serde_json::json!({ "round": round, "tool": primary.name.clone(), "error": e.to_string() }));
                    sess.tool_result(primary.call_id.as_str(), format!("参数错误: {}", e));
                    continue;
                }
            };
            tx.log(
                "llm_decision",
                serde_json::json!({ "round": round, "tool": primary.name.clone(), "comment": comment.clone(), "arguments": primary.arguments.clone() }),
            );

            // 每次 LLM 调用打印一行思考（comment 优先，其次模型文字，再次 reason，最后工具名）
            // + token 着色在后；不显示"AI"标签
            let say = comment
                .as_deref()
                .map(|s| brief(s, 200))
                .filter(|s| !s.is_empty())
                .or_else(|| thinking.as_deref().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .or_else(|| action.intent().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| format!("调用 {}", primary.name));
            eprintln!("  {}  {}", say, toks);

            match action {
                AgentAction::Finish { success, reason, script_name: sn } => {
                    // 回执 finish 的 tool_result，避免后续 desc 生成轮因悬空 tool_call 被 API 拒
                    sess.tool_result(primary.call_id.as_str(), "结束前先做终点校验");
                    // —— 终点校验：把当前页面 + 用户原话需求发回模型，判断是否真到对的地方 ——
                    //   只对"声称成功"的 finish 校验；被打回有上限，避免无限循环。
                    if success && finish_rechecks < 2 && perceive_err.is_none() {
                        finish_rechecks += 1;
                        let check = render(
                            &ctx.prompts.message("explorer", "finish_check"),
                            &[("case", ctx.case), ("page", &list_text)],
                        );
                        tx.log("llm_message", serde_json::json!({ "round": round, "content": check.clone() }));
                        sess.user(check);
                        if let Ok(LlmReply::Text(t)) = sess.next().await {
                            let (pt, ct) = sess.last_usage();
                            tx.log("finish_check", serde_json::json!({ "round": round, "content": t.clone(), "prompt_tokens": pt, "completion_tokens": ct }));
                            if let Some(obj) = parse_desc_json(&t) {
                                if obj["done"].as_bool() == Some(false) {
                                    let why = obj["reason"].as_str().unwrap_or("尚未真正达成").to_string();
                                    eprintln!("  {}", paint(tty, "33", &format!("终点校验未通过：{}", brief(&why, 80))));
                                    sess.user(render(&ctx.prompts.message("explorer", "finish_recheck_fail"), &[("why", &why)]));
                                    continue; // 不结束，回到内层循环继续修
                                }
                            }
                        }
                    }
                    // 结果在末尾统一 section 展示，这里只记录与收尾
                    tx.log("finish", serde_json::json!({ "success": success, "reason": reason.clone(), "script_name": sn.clone() }));
                    script_name = sn;
                    finish = Some((success, reason));
                    break 'outer;
                }
                AgentAction::RequestScreenshot { reason } => {
                    tx.log("screenshot_requested", serde_json::json!({ "round": round, "reason": reason }));
                    sess.tool_result(primary.call_id.as_str(), "已附上当前页面截图（见下一条消息）");
                    let shot_msg = ctx.prompts.message("explorer", "screenshot_provided");
                    tx.log("llm_message", serde_json::json!({ "round": round, "content": shot_msg.clone(), "image": p.shot_path.to_string_lossy() }));
                    match sess.user_with_image(&shot_msg, &p.shot_path) {
                        Ok(()) => tx.log(
                            "screenshot_sent",
                            serde_json::json!({ "round": round, "screenshot": p.shot_path.to_string_lossy() }),
                        ),
                        Err(e) => {
                            tx.log("screenshot_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                            let m = ctx.prompts.message("explorer", "screenshot_failed");
                            tx.log("llm_message", serde_json::json!({ "round": round, "content": m.clone() }));
                            sess.user(m);
                        }
                    }
                    continue; // 不前进，重新询问
                }
                AgentAction::AskUser { question } => {
                    tx.log("ask_user", serde_json::json!({ "round": round, "question": question.clone() }));
                    let answer = read_user_line(&question).await;
                    tx.log("user_reply", serde_json::json!({ "round": round, "answer": answer.clone() }));
                    sess.tool_result(primary.call_id.as_str(), format!("用户答复：{}", answer));
                    continue; // 不前进，重新询问
                }
                AgentAction::Rename { old_name, new_name } => {
                    // 纠正起错的已知元素名：改库 + 同步已生成的 .tks 引用 + 当前轮已知名映射
                    let ok = crate::tools::element::rename_element(ctx.element_path, &old_name, &new_name)
                        .unwrap_or(false);
                    if ok {
                        let from = format!("{{{}}}", old_name);
                        let to = format!("{{{}}}", new_name);
                        for l in lines.iter_mut() {
                            if l.contains(&from) {
                                *l = l.replace(&from, &to);
                            }
                        }
                        for k in known_names.iter_mut() {
                            if k.as_deref() == Some(old_name.as_str()) {
                                *k = Some(new_name.clone());
                            }
                        }
                        for c in created.iter_mut() {
                            if *c == old_name {
                                *c = new_name.clone();
                            }
                        }
                        renames.push((old_name.clone(), new_name.clone()));
                        tx.log("rename_element", serde_json::json!({ "round": round, "old": old_name.clone(), "new": new_name.clone() }));
                        eprintln!("  {}", paint(tty, "35", &format!("✎ 改名：{} → {}", old_name, new_name)));
                        sess.tool_result(primary.call_id.as_str(), format!("已改名：{} → {}（库与脚本引用已同步）", old_name, new_name));
                    } else {
                        sess.tool_result(
                            primary.call_id.as_str(),
                            format!("改名未生效：{} 不存在或 {} 已被占用，请换个新名或确认旧名", old_name, new_name),
                        );
                    }
                    continue; // 不前进，重新询问
                }
                device_action => {
                    // 单行格式（对齐回放/tke run）：先显示「[N] 指令 ...」再操作设备，执行后接上
                    // `✓ 耗时`/`✗ 耗时`——CLI 与设备同步，且 agent 这步点啥、花多久一目了然。
                    let step_no = steps.len() + 1;
                    let preview = preview_action(&device_action, &p.elements);
                    if let Some(pv) = &preview {
                        eprint!("  {} {} {} ", paint(tty, "2", &format!("[{:>2}]", step_no)), pv, paint(tty, "2", "..."));
                        let _ = std::io::stderr().flush();
                    }
                    let act_t0 = std::time::Instant::now();
                    let is_swipe = matches!(&device_action, AgentAction::SwipeDir { .. });
                    match execution::apply(
                        ctx.device,
                        ctx.element_path,
                        &device_action,
                        &p.elements,
                        &known_names,
                        &p.shot_path,
                        tx,
                        round,
                    )
                    .await
                    {
                        Ok((line, detail, trace, saved)) => {
                            let dur = fmt_duration(act_t0.elapsed().as_millis() as u64);
                            if preview.is_some() {
                                // 接上之前那行「[N] 指令 ...」
                                eprintln!("{} {}", paint(tty, "32", "✓"), paint(tty, "2", &dur));
                            } else {
                                eprintln!("  {} {} {}", paint(tty, "32", "✓"), friendly(&line), paint(tty, "2", &dur));
                            }
                            // 记录元素库变更（去重），供结束时人工审核
                            if let Some(s) = saved {
                                if s.created {
                                    if !created.contains(&s.name) {
                                        created.push(s.name.clone());
                                    }
                                } else if s.desc_updated
                                    && !created.contains(&s.name)
                                    && !updated_names.contains(&s.name)
                                {
                                    // 写出"哪个元素的 desc 从什么改成什么"
                                    updated.push(format!(
                                        "{} · desc：「{}」→「{}」",
                                        s.name,
                                        s.old_desc.as_deref().unwrap_or("（空）"),
                                        s.new_desc.as_deref().unwrap_or("（空）"),
                                    ));
                                    updated_names.push(s.name.clone());
                                }
                            }
                            // 存本步产物（screenshots/step_NNN + page/step_NNN），与 run 同构
                            // trace 带点击点+元素 bounds → 截图标注红框+蓝点，可核对 AI 实际点到哪
                            let step_index = steps.len();
                            let (screenshot, xml) =
                                ctx.artifacts.save_step(ctx.workarea, step_index, &trace, &line, true);
                            tx.log(
                                "tks_step",
                                serde_json::json!({ "round": round, "step": step_index + 1, "line": line.clone(), "exec": detail.clone(), "screenshot": screenshot.clone(), "xml": xml.clone() }),
                            );
                            steps.push(StepResult {
                                index: step_index,
                                command: line.clone(),
                                success: true,
                                error: None,
                                duration_ms: 0,
                                line: None,
                                screenshot,
                                xml,
                            });
                            lines.push(line.clone());
                            step_comments.push(comment.clone().unwrap_or_default());
                            last_was_swipe = is_swipe; // 记下这步是不是滑动，供下一轮判定空滑
                            last_was_close = matches!(&device_action, AgentAction::Close { .. }); // 记下是不是主动收尾关闭
                            sess.tool_result(primary.call_id.as_str(), format!("已执行：{}（.tks: {}）", detail, line));
                        }
                        Err(e) => {
                            let dur = fmt_duration(act_t0.elapsed().as_millis() as u64);
                            if preview.is_some() {
                                eprintln!("{} {}", paint(tty, "31", "✗"), paint(tty, "2", &dur));
                            }
                            eprintln!("  {} {}", paint(tty, "31", "✗ 执行失败:"), e);
                            tx.log("exec_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                            sess.tool_result(primary.call_id.as_str(), format!("执行失败: {}", e));
                            continue; // 同一页面重试
                        }
                    }
                    // settle：等页面切换动画/加载稳定再重新采集，避免抓到上一页或半截动画
                    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    break; // 页面已变 → 重新采集
                }
            }
        }
    }

    // 收尾：确保加载动画已停（如中断发生在首屏采集之前）
    if let Some(h) = spin_handle.take() {
        spin_stop.store(true, Ordering::Relaxed);
        let _ = h.await;
    }

    let (success, reason) = finish.unwrap_or((false, format!("达到最大轮数({})未结束", ctx.max_rounds)));

    // approach A：探索结束后，据本轮各新建元素的实际作用统一生成 desc（创建时不写、不想当然）。
    // 复用同一会话——它已知道每个元素被点击后发生了什么。中断时也生成（元素已被点过，丢了可惜），
    // 给"退出中"提示；再按一次 Ctrl+C 会命中默认信号、强制退出。
    // desc 写进 element.json（持久化）；最终「元素库更新」框由上层据库中 desc 渲染。
    if !created.is_empty() {
        if aborted {
            eprintln!("{}", paint(tty, "33", "退出中：正在为新建元素生成描述…（再次 Ctrl+C 强制退出）"));
        }
        let desc_msg = render(&ctx.prompts.message("explorer", "desc_pass"), &[("elements", &created.join("、"))]);
        tx.log("llm_message", serde_json::json!({ "content": desc_msg.clone() }));
        sess.user(desc_msg);
        if let Ok(LlmReply::Text(t)) = sess.next().await {
            tx.log("desc_pass", serde_json::json!({ "content": t.clone() }));
            if let Some(obj) = parse_desc_json(&t) {
                for name in &created {
                    if let Some(d) = obj.get(name).and_then(|v| v.as_str()).map(str::trim) {
                        if !d.is_empty() {
                            let _ = crate::tools::element::set_element_desc(ctx.element_path, name, d);
                        }
                    }
                }
            }
        }
    }

    Ok(DriveOutcome { success, reason, lines, steps, rounds: round, created, updated, aborted, script_name, renames, step_comments })
}
