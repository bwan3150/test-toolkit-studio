// 主循环：perceive → decide → apply → record（只编排，具体能力委托各子模块）
// 产物经 RunArtifacts 统一保存：每个执行步存 screenshots/step_NNN + page/step_NNN，
// 与 tke run 同构；conversation.jsonl（AI 原始对话）由 transcript 写在同一运行目录。

use crate::Platform;
use crate::{LlmReply, LlmSession, Result, StepResult};

/// 页面签名哈希（用元素列表文本），检测"原地打转"用
fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

use super::super::execution;
use super::super::transcript::Transcript;
use super::ctx::{DriveCtx, DriveOutcome};
use super::fmt::{brief, friendly, parse_desc_json, preview_action};
use super::super::perception::{capture, match_known, render_element_list, Perceived};
use super::super::ui::{ask_with_options, Level, NotReady, StepState, SubAgent, Tokens, UiCommand, UiEvent};
use super::super::prompt::render;
use super::super::tools::{parse_tool_call, AgentAction};
use super::asserter;
use super::supervisor;

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
    _spinner: bool,
    // 重探批次前缀（如「重探1·」）。Page 事件不再携带它（前缀语义改由 Phase::Reexplore 表达），
    // 故本函数内部已不消费；保留参数以兼容调用方，待后续阶段如需可并入 Page。
    _round_prefix: &str,
) -> Result<DriveOutcome> {
    // 进入「探索 agent」作用域：本函数（首次探索 与 reexplore 活体重探都走它）产出的所有事件
    // 都归属 explorer。Drop 守卫保证任何退出路径都弹栈（被 doctor 调用时弹回 doctor 作用域）。
    let mut _ascope = txp.scoped("explorer");
    let tx = &mut *_ascope;

    // 用户中断（Ctrl+C）：查进程级统一中断标志（由 interrupt::install 在运行开始时安装监听），
    // 监听到则在下一个决策点优雅停止、照常出总结。

    let mut aborted = false;
    let mut page_sigs: Vec<u64> = Vec::new(); // 各轮结构签名，检测原地打转
    let mut no_progress = 0usize; // 连续多少轮页面无变化（上一步操作没生效）
    let max_llm_calls = ctx.max_rounds * 4 + 10; // 含要图/反问等不前进的轮次
    let mut lines: Vec<String> = Vec::new();
    let mut step_comments: Vec<String> = Vec::new(); // 与 lines 对应：每步 AI 的理由(comment)
    let mut steps: Vec<StepResult> = Vec::new();
    let mut round = 0usize;
    let mut llm_calls = 0usize;
    let mut supervisor_rejects = 0usize; // 监督官打回 finish 的次数（防无限：到上限后以失败收场）
    const MAX_SUPERVISOR_REJECTS: usize = 3;
    let mut subagent_pt = 0i64; // 子 agent(踩实官 + 监督官)独立会话 token 合计，累进 DriveOutcome
    let mut subagent_ct = 0i64;
    let mut prev_action_elements: Vec<crate::UIElement> = Vec::new(); // 上一步执行前所见的页面元素，用于算"新出现的元素"增量
    let mut finish: Option<(bool, String)> = None;
    let mut script_name: Option<String> = None; // AI 在 finish 时起的脚本名
    let mut renames: Vec<(String, String)> = Vec::new(); // 本次元素改名记录
    let mut loop_streak = 0usize; // 连续"同一操作且页面不变"的次数（真打转信号）
    let mut pingpong_streak = 0usize; // 连续"页面变了但回到旧页、且没探出新页面"的次数（A↔B 横跳信号）
    let mut escalated = false; // 是否已注入过"升级求助"硬指令（每次探索只强制一次）
    let mut last_was_swipe = false; // 上一步是否是滑动（用于剔除"空滑"——回放会滑过头）
    let mut last_was_close = false; // 上一步是否是主动关闭/收尾（关 app/销毁会话）——下一轮空页面不催 launch
    let mut last_was_nav = false; // 上一步是否是「导航」动作（点击/视觉点击/切换/拖拽）——用于「踩实」判定
    let mut last_no_pagecheck = false; // 上一步是否是「天生不改变页面」的指令（断言/等待/隐藏键盘）——这类指令执行后不做前后页面对比
    // 本轮测试对元素库的变更（供结束时人工审核）
    let mut created: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new(); // 格式化的差异行
    let mut updated_names: Vec<String> = Vec::new(); // 去重用
    let mut pending_guidance: Vec<String> = Vec::new(); // 安全点收到的用户中途指导，下一轮注入会话
    let mut guidance_log: Vec<String> = Vec::new(); // 全程用户指导——历史压缩时保留在 summary 里持续生效
    const COMPACT_KEEP_PAGES: usize = 6; // 历史压缩：保留最近几轮页面消息以来的完整对话

    'outer: while round < ctx.max_rounds {
        // 安全点：取前端命令（本阶段只处理 Abort，优雅停在此处）
        for c in ctx.ui.drain_commands() {
            match c {
                UiCommand::Abort => {
                    aborted = true;
                    finish = Some((false, "已终止（用户中断）".to_string()));
                    break 'outer;
                }
                UiCommand::Guidance { text } => pending_guidance.push(text),
                UiCommand::Pause => {
                    super::interrupt::clear_pause(); // 已响应软停，清标志（当前动作已停下）
                    // 软停：暂停当前探索，等用户给指导再带着继续；放弃（再按 Ctrl+C）则终止
                    match ctx
                        .ui
                        .await_answer(round, "已暂停。输入指导让 AI 带着继续，或再按 Ctrl+C 退出".to_string())
                        .await
                    {
                        Some(g) if !g.trim().is_empty() => pending_guidance.push(g),
                        Some(_) => {}
                        None => {
                            aborted = true;
                            finish = Some((false, "已终止（用户中断）".to_string()));
                            break 'outer;
                        }
                    }
                }
                _ => {}
            }
        }
        if super::interrupt::aborted() {
            aborted = true;
            finish = Some((false, "已终止（用户中断 Ctrl+C）".to_string()));
            break 'outer;
        }
        round += 1;

        // 首轮给一句可见反馈：`━━ 探索 ━━` 之后如果直接进采集，adb 慢时 TUI 会静默十几秒像卡死
        if round == 1 {
            ctx.ui.emit(UiEvent::Notice { level: Level::Dim, text: "▶ 采集首屏页面…".to_string() });
        }

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
        // 上一步是「天生不改变页面」的指令(断言/等待/隐藏键盘)时，页面不变是**预期**、不携带任何"有没有
        // 前进"的信息——不记签名(免污染打转统计)、不累计 no_progress、本轮一律不做"无变化/打转"判定，
        // 免得把正常的不变误报成"上一步没生效/原地打转"。
        if last_no_pagecheck {
            no_progress = 0;
        } else {
            page_sigs.push(struct_sig);
            if unchanged {
                no_progress += 1;
            } else {
                no_progress = 0;
            }
        }
        // 踩实(非阻塞·基于页面 diff)：上一步是「导航」(点击/视觉点击/切换/拖拽)且确实让页面变了 → 本轮提示
        // AI「操作有反应，但要确认方向」，并把**因上一步操作新出现的元素**单列出来，引导它从中挑一个 assert
        // 踩实(或发现进错页就 back)。不阻塞、不打断——只引导。空元素页(PDF/新标签)无法断言则不提示。
        let changed_after_nav = last_was_nav && !unchanged && !p.elements.is_empty();
        // 新出现的元素索引(当前页有、上一步执行前没有)：按 to_ai_text 去重比对
        let delta_indices: Vec<usize> = if changed_after_nav {
            let prev_keys: std::collections::HashSet<String> = prev_action_elements.iter().map(|e| e.to_ai_text()).collect();
            p.elements.iter().enumerate().filter(|(_, e)| !prev_keys.contains(&e.to_ai_text())).map(|(i, _)| i).collect()
        } else {
            Vec::new()
        };
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
        if !last_no_pagecheck && unchanged && repeated_same {
            loop_streak += 1;
        } else {
            loop_streak = 0;
        }
        const LOOP_ABORT: usize = 3;
        if loop_streak >= LOOP_ABORT && !p.elements.is_empty() {
            let act = lines.last().map(|l| friendly(l)).unwrap_or_default();
            tx.log("stuck_abort", serde_json::json!({ "round": round, "loop_streak": loop_streak, "kind": "same_action_no_change", "action": act }));
            ctx.ui.emit(UiEvent::AutoStop { reason: format!("反复执行同一无效操作「{}」且页面始终不变，自动停止", act) });
            finish = Some((false, format!("反复执行同一无效操作「{}」、自动停止", act)));
            break 'outer;
        }
        // 兜底止损：页面极长时间完全不前进（哪怕一直在试不同元素），最终也停（措辞不叫"重复"）。
        // 阈值放宽，给"逐步在正确道路上"的尝试留足空间。
        const NO_PROGRESS_BACKSTOP: usize = 12;
        if no_progress >= NO_PROGRESS_BACKSTOP && !p.elements.is_empty() {
            tx.log("stuck_abort", serde_json::json!({ "round": round, "no_progress": no_progress, "kind": "no_progress" }));
            ctx.ui.emit(UiEvent::AutoStop { reason: format!("连续 {} 轮页面始终无前进，自动停止（避免空烧 token）", no_progress) });
            finish = Some((false, format!("连续 {} 轮页面无前进、自动停止", no_progress)));
            break 'outer;
        }
        // 横跳止损（填盲区）：页面**变了**但回到了近期出现过的旧页（revisits>=2），且一直没探出任何
        // 从未见过的新页面——典型是反复点一个"点了会跳回首页/旧页"的元素（点歪到 logo、顶栏悬停同名项、
        // 纯图标点不中跳走）。上面 loop_streak/no_progress 两道闸门都要求"页面不变"，对这种 A↔B 横跳是
        // **结构性盲区**（每步 unchanged=false，二者全程归零），故单独计数，连续横跳到上限就止损。
        if !last_no_pagecheck {
            if revisits == 0 {
                pingpong_streak = 0; // 探出了从未见过的新页面 → 不是横跳，真有进展
            } else if !unchanged && revisits >= 2 {
                pingpong_streak += 1; // 页面变了却又回到旧页、还没探出新页 → 横跳一次
            }
        }
        const PINGPONG_ABORT: usize = 6;
        if pingpong_streak >= PINGPONG_ABORT && !p.elements.is_empty() {
            tx.log("stuck_abort", serde_json::json!({ "round": round, "pingpong": pingpong_streak, "kind": "pingpong" }));
            ctx.ui.emit(UiEvent::AutoStop { reason: format!("在少数几个页面间反复横跳 {} 次、始终没探出新页面，自动停止（避免空烧 token）", pingpong_streak) });
            finish = Some((false, format!("在少数几个页面间反复横跳 {} 次、自动停止", pingpong_streak)));
            break 'outer;
        }
        let stuck = !last_no_pagecheck && (no_progress >= 1 || revisits >= 2) && !p.elements.is_empty();
        // 升级求助（nudge 之上、自动停止之下的中间层）：温和提示后仍多轮无进展/横跳，
        // 注入硬指令——必须 ask_user 求助（交互→转用户；托管→参谋代答）或 finish(false) 交卡点报告。
        // 只触发一次：求助后要么方向被纠正，要么继续恶化由上面的自动停止兜底。
        let escalate = !escalated
            && !p.elements.is_empty()
            && !last_no_pagecheck
            && (no_progress >= 4 || pingpong_streak >= 3);
        if escalate {
            escalated = true;
            tx.log("stuck_escalate", serde_json::json!({ "round": round, "no_progress": no_progress, "pingpong": pingpong_streak }));
            ctx.ui.emit(UiEvent::Stuck { round, message: "多轮无进展，已要求 AI 求助或交卡点报告".to_string() });
            let m = ctx.prompts.message("explorer", "stuck_escalate");
            tx.log("llm_message", serde_json::json!({ "round": round, "content": m.clone() }));
            sess.user(m);
        }

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
        } else if last_no_pagecheck {
            // 上一步是断言/等待/隐藏键盘——页面没变是预期，不发"无变化/打转"提示
            String::new()
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
        let go_visual = !last_no_pagecheck && (no_progress >= 2 || revisits >= 3) && perceive_err.is_none();
        // 中途指导注入：把安全点收到的用户指导，在本轮采集完页面、发 page 消息给 LLM 之前注入会话，
        // 让 AI 据此调整接下来的方向（与 page 消息一并下发，下一轮即生效）。
        if !pending_guidance.is_empty() {
            for g in pending_guidance.drain(..) {
                let m = format!("【用户中途指导】{}\n请据此调整接下来的操作方向。", g);
                tx.log("user_guidance", serde_json::json!({ "round": round, "content": m.clone() }));
                sess.user(m);
                guidance_log.push(g.clone());
                ctx.ui.emit(UiEvent::GuidanceAccepted { text: g });
            }
        }

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
        // 历史压缩：只保留最近 COMPACT_KEEP_PAGES 轮页面以来的完整对话，更早轮次折叠成一条
        // 确定性回顾（已执行步骤 + 用户全程指导）。页面虽逐轮 elide，tool_result/AI 思考仍随
        // 轮数线性累积、且每次 next 全量重发——不压缩则 prompt token 平方级膨胀（40 轮 58 万的根因）。
        {
            let steps_recap = if lines.is_empty() {
                "（尚无）".to_string()
            } else {
                lines.iter().enumerate().map(|(i, l)| format!("{}. {}", i + 1, friendly(l))).collect::<Vec<_>>().join("\n")
            };
            let mut summary = format!(
                "【历史已压缩】更早轮次的对话细节已省略。到目前为止你已执行并写入脚本的步骤：\n{}\n请基于当前页面继续推进任务。",
                steps_recap
            );
            if !guidance_log.is_empty() {
                summary.push_str(&format!("\n\n【用户此前的指导，务必持续遵守】\n- {}", guidance_log.join("\n- ")));
            }
            if sess.compact_history(COMPACT_KEEP_PAGES, summary) {
                tx.log("history_compacted", serde_json::json!({ "round": round, "keep_pages": COMPACT_KEEP_PAGES }));
            }
        }
        // 页面观察头：emit 一个 Page 事件，前端各自渲染（结构元素 / OCR 新增 / 标签页 / 未就绪原因）。
        // 不再显示"第 N 轮"——下面每个执行步前的 `[N]` tks 行号才是锚点；不显示"已知/未知"对 AI 判断无意义。
        ctx.ui.emit(UiEvent::Page {
            round,
            struct_elements: p.elements.len().saturating_sub(p.ocr_added),
            ocr_added: if ctx.ocr.is_some() && p.ocr_error.is_none() { Some(p.ocr_added) } else { None },
            ocr_failed: ctx.ocr.is_some() && p.ocr_error.is_some(),
            tabs: p.tabs.len(),
            location: p.tabs.iter().find(|t| t.active).map(|t| {
                if t.title.is_empty() { t.url.clone() } else { t.title.clone() }
            }),
            known: n_known,
            unknown: n_unknown,
            revisits,
            not_ready: if perceive_err.is_some() {
                Some(if last_was_close { NotReady::SessionClosed } else { NotReady::NeedLaunch })
            } else {
                None
            },
        });
        // OCR 接口报错：单列具体报错 + 记日志，绝不让它被"0"掩盖。
        if let Some(err) = &p.ocr_error {
            tx.log("ocr_error", serde_json::json!({ "round": round, "error": err }));
            ctx.ui.emit(UiEvent::OcrError { round, error: err.clone() });
        }
        // 卡住提示
        if stuck {
            let msg = if no_progress >= 2 || revisits >= 3 {
                "上一步没生效/原地打转，已附截图让 AI 看图"
            } else if no_progress >= 1 {
                "上一步操作后页面无变化，已提示 AI 换做法"
            } else {
                "在多页间打转，已提示 AI 换路径"
            };
            ctx.ui.emit(UiEvent::Stuck { round, message: msg.to_string() });
        }

        // 1b) 自动断言（踩实）：上一步导航让页面变了 → 断言官从**本次新出现的元素**里挑标志、
        //     系统自动插断言（不依赖主探索 agent 记得断言，它总漏）。全流程在 asserter::auto_checkpoint：
        //     无新增元素直接跳过、挑不出=疑似点错只记日志（交监督官在 finish 处兜底）。
        if changed_after_nav && !ctx.task_mode {
            let (apt, act) = asserter::auto_checkpoint(
                ctx,
                tx,
                &p,
                &delta_indices,
                &known_names,
                round,
                asserter::CheckpointScript {
                    lines: &mut lines,
                    step_comments: &mut step_comments,
                    steps: &mut steps,
                    created: &mut created,
                },
            )
            .await;
            subagent_pt += apt;
            subagent_ct += act;
        }

        // 2) 内层：持续问 AI，直到产生一个"改变页面的动作"或结束
        loop {
            // 安全点：取前端命令（Abort 终止；Guidance 即时注入会话；Pause 软停等指导再继续）
            for c in ctx.ui.drain_commands() {
                match c {
                    UiCommand::Abort => {
                        aborted = true;
                        finish = Some((false, "已终止（用户中断）".to_string()));
                        break 'outer;
                    }
                    UiCommand::Guidance { text } => {
                        let m = format!("【用户中途指导】{}\n请据此调整接下来的操作方向。", text);
                        tx.log("user_guidance", serde_json::json!({ "round": round, "content": m.clone() }));
                        sess.user(m);
                        guidance_log.push(text.clone());
                        ctx.ui.emit(UiEvent::GuidanceAccepted { text });
                    }
                    UiCommand::Pause => {
                        super::interrupt::clear_pause(); // 已响应软停，清标志（当前动作已停下）
                        match ctx
                            .ui
                            .await_answer(round, "已暂停。输入指导让 AI 带着继续，或再按 Ctrl+C 退出".to_string())
                            .await
                        {
                            Some(g) if !g.trim().is_empty() => {
                                let m = format!("【用户中途指导】{}\n请据此调整接下来的操作方向。", g);
                                tx.log("user_guidance", serde_json::json!({ "round": round, "content": m.clone() }));
                                sess.user(m);
                                guidance_log.push(g.clone());
                                ctx.ui.emit(UiEvent::GuidanceAccepted { text: g });
                            }
                            Some(_) => {}
                            None => {
                                aborted = true;
                                finish = Some((false, "已终止（用户中断）".to_string()));
                                break 'outer;
                            }
                        }
                    }
                    _ => {}
                }
            }
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

            // 本次 LLM 调用的 token 用量（上行/输入、下行/输出），随 emit 一并发出
            let (pt, ct) = sess.last_usage();
            tx.log("llm_usage", serde_json::json!({ "round": round, "seq": llm_calls, "prompt_tokens": pt, "completion_tokens": ct }));

            let (thinking, calls) = match reply {
                LlmReply::Text(t) => {
                    let b = brief(&t, 200);
                    let say = if b.is_empty() { "（未调用工具，已提示其用工具或 finish）".to_string() } else { b };
                    ctx.ui.emit(UiEvent::AgentThought { round, text: say.clone(), tokens: Tokens::new(pt, ct) });
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

            // 每次 LLM 调用一句思考（comment 优先，其次模型文字，再次 reason，最后工具名）
            let say = comment
                .as_deref()
                .map(|s| brief(s, 200))
                .filter(|s| !s.is_empty())
                .or_else(|| thinking.as_deref().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .or_else(|| action.intent().map(|s| brief(s, 200)).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| format!("调用 {}", primary.name));
            ctx.ui.emit(UiEvent::AgentThought { round, text: say.clone(), tokens: Tokens::new(pt, ct) });

            match action {
                AgentAction::Finish { success, reason, script_name: sn } => {
                    // 回执 finish 的 tool_result，避免后续 desc 生成轮因悬空 tool_call 被 API 拒
                    sess.tool_result(primary.call_id.as_str(), "结束前先交监督官审查");
                    // —— 监督官把关：独立 agent 审查「用户需求 + 全部步骤 + 最后一页元素」，放行才结束、
                    //    否则打回继续探索（治"过早 finish/放弃"）。打回有上限，到顶后判定卡死、以失败收场。
                    // 通用任务模式(operate)跳过把关——它不是"完成测试用例"，agent 说做完就做完。
                    if !ctx.task_mode && supervisor_rejects < MAX_SUPERVISOR_REJECTS && perceive_err.is_none() {
                        // 步骤清单：步号 · 动作 · 当时理由
                        let steps_listing = lines
                            .iter()
                            .enumerate()
                            .map(|(i, l)| {
                                let why = step_comments.get(i).map(|c| c.trim()).filter(|c| !c.is_empty()).unwrap_or("");
                                if why.is_empty() {
                                    format!("{}. {}", i + 1, friendly(l))
                                } else {
                                    format!("{}. {}  ← {}", i + 1, friendly(l), brief(why, 60))
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let verdict =
                            supervisor::supervise(ctx.ai, ctx.prompts, tx, ctx.device, ctx.case, &steps_listing, &list_text, success, &reason).await;
                        // 把关失败（会话/解析出错）绝不静默：兜底虽按"放行"处理（不卡死探索），
                        // 但必须让用户看见"这次 finish 没经过监督"——否则供应商抖动时整套把关悄悄下线。
                        if verdict.is_none() {
                            tx.log("supervisor_error", serde_json::json!({ "round": round, "note": "会话或解析失败，按放行兜底" }));
                            ctx.ui.emit(UiEvent::SubAgent {
                                kind: SubAgent::Supervisor,
                                level: Level::Warn,
                                text: "监督官调用失败——本次 finish 未经把关，按放行处理".to_string(),
                                tokens: Tokens::new(0, 0),
                            });
                        }
                        if let Some(v) = verdict {
                            subagent_pt += v.pt;
                            subagent_ct += v.ct;
                            if !v.approved {
                                supervisor_rejects += 1;
                                ctx.ui.emit(UiEvent::SubAgent {
                                    kind: SubAgent::Supervisor,
                                    level: Level::Warn,
                                    text: format!("打回（第 {} 次）：{}", supervisor_rejects, v.reason),
                                    tokens: Tokens::new(v.pt, v.ct),
                                });
                                let m = render(&ctx.prompts.message("supervisor", "pushback"), &[("reason", &v.reason)]);
                                tx.log("llm_message", serde_json::json!({ "round": round, "content": m.clone() }));
                                sess.user(m);
                                continue; // 不结束，回到内层循环按监督官指引继续探索
                            }
                            ctx.ui.emit(UiEvent::SubAgent {
                                kind: SubAgent::Supervisor,
                                level: Level::Ok,
                                text: format!("放行：{}", v.reason),
                                tokens: Tokens::new(v.pt, v.ct),
                            });
                        }
                    }
                    // 监督官已多次打回仍走到这 → 判定卡死，以探索失败收场（防死循环）
                    let (eff_success, eff_reason) = if supervisor_rejects >= MAX_SUPERVISOR_REJECTS {
                        (false, format!("监督官连续 {} 次打回仍未达成，判定探索失败：{}", supervisor_rejects, reason))
                    } else {
                        (success, reason)
                    };
                    tx.log("finish", serde_json::json!({ "success": eff_success, "reason": eff_reason.clone(), "script_name": sn.clone(), "supervisor_rejects": supervisor_rejects }));
                    script_name = sn;
                    finish = Some((eff_success, eff_reason));
                    break 'outer;
                }
                AgentAction::RequestScreenshot { reason } => {
                    // 工具调用可见化：设备动作有 [N] 步骤行、改名有 ✎ 行，这个此前是隐形的——
                    // 用户只看到"AI 说要看图"然后凭空继续，不知道它真调了工具
                    ctx.ui.emit(UiEvent::Notice { level: Level::Dim, text: "⚒ request_screenshot：查看当前页面截图".to_string() });
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
                AgentAction::AskUser { question, options } => {
                    tx.log("ask_user", serde_json::json!({ "round": round, "question": question.clone(), "options": options.clone() }));
                    // 托管（用户点名 auto）或没人可问（Plain/CI）→ **参谋代答**：
                    // 单次调用站在全局目标+当前页面视角给行动指示，绝不阻塞、绝不打扰用户
                    let auto = ctx.ask_mode == super::ctx::AskMode::Auto || !ctx.ui.supports_prompts();
                    if auto {
                        ctx.ui.emit(UiEvent::Notice { level: Level::Dim, text: format!("? Explorer 求助：{}", brief(&question, 80)) });
                        let adv = super::advisor::consult(ctx.ai, ctx.prompts, tx, ctx.case, &question, &options, &lines, &list_text, true).await;
                        subagent_pt += adv.pt;
                        subagent_ct += adv.ct;
                        if adv.answer.is_empty() {
                            sess.tool_result(primary.call_id.as_str(), "（托管模式，参谋暂无答复——请基于已有信息选最稳妥的方向继续）");
                        } else {
                            ctx.ui.emit(UiEvent::Notice { level: Level::Dim, text: format!("◇ 参谋代答：{}", brief(&adv.answer, 100)) });
                            sess.tool_result(primary.call_id.as_str(), format!("（托管模式，参谋代答）{}", adv.answer));
                        }
                        continue;
                    }
                    // 交互：探索 agent 没自带候选时，请参谋把问题提炼成候选选项（用户可选可输入,少打字）
                    let opts_final = if options.is_empty() {
                        let adv = super::advisor::consult(ctx.ai, ctx.prompts, tx, ctx.case, &question, &options, &lines, &list_text, false).await;
                        subagent_pt += adv.pt;
                        subagent_ct += adv.ct;
                        adv.options
                    } else {
                        options
                    };
                    let answer = if opts_final.is_empty() {
                        ctx.ui.await_answer(round, question.clone()).await
                    } else {
                        ask_with_options(ctx.ui, round, &question, &opts_final).await
                    };
                    let answer = match answer {
                        Some(a) => a,
                        None => { aborted = true; finish = Some((false, "已终止（用户中断）".to_string())); break 'outer; }
                    };
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
                        ctx.ui.emit(UiEvent::Rename { old: old_name.clone(), new: new_name.clone() });
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
                    // 踩实改为「非阻塞引导」(见上方 changed_after_nav/confirm_direction)：不再拦截动作、不打断模型，
                    // 避免弱模型被拦后误判执行状态而放弃。是否真踩实由模型自觉 + finish 处监督官兜底把关。
                    // 记录本步执行前所见的页面元素，供下一轮算"新出现的元素"增量(delta) 给 AI 看。
                    prev_action_elements = p.elements.clone();
                    // 单行格式（对齐回放/tke run）：先显示「[N] 指令 ...」再操作设备，执行后接上
                    // `✓ 耗时`/`✗ 耗时`——CLI 与设备同步，且 agent 这步点啥、花多久一目了然。
                    let step_no = steps.len() + 1;
                    let preview = preview_action(&device_action, &p.elements);
                    ctx.ui.emit(UiEvent::Step {
                        step: step_no,
                        state: StepState::Running,
                        preview: preview.clone().unwrap_or_default(),
                        line: None,
                        duration_ms: None,
                        error: None,
                    });
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
                            ctx.ui.emit(UiEvent::Step {
                                step: step_no,
                                state: StepState::Ok,
                                preview: preview.clone().unwrap_or_else(|| friendly(&line)),
                                line: Some(line.clone()),
                                duration_ms: Some(act_t0.elapsed().as_millis() as u64),
                                error: None,
                            });
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
                                healed: None,
                                note: None,
                                dialog: None,
                            });
                            lines.push(line.clone());
                            step_comments.push(comment.clone().unwrap_or_default());
                            last_was_swipe = is_swipe; // 记下这步是不是滑动，供下一轮判定空滑
                            last_was_close = matches!(&device_action, AgentAction::Close { .. }); // 记下是不是主动收尾关闭
                            // 记下这步是不是「导航」(点击/视觉点击/切换/拖拽)——下一轮据此强制踩实(若它进了新页面)
                            last_was_nav = matches!(
                                &device_action,
                                AgentAction::Click { .. } | AgentAction::ClickVisual { .. } | AgentAction::Switch { .. } | AgentAction::Drag { .. }
                            );
                            // 记下这步是不是「天生不改变页面」的指令(断言/等待/隐藏键盘)——下一轮不做前后页面对比，
                            // 免得把"正常的页面没变"误报成"上一步没生效/原地打转"。
                            last_no_pagecheck = matches!(
                                &device_action,
                                AgentAction::Assert { .. } | AgentAction::AssertVisual { .. } | AgentAction::Wait { .. } | AgentAction::HideKeyboard
                            );
                            sess.tool_result(primary.call_id.as_str(), format!("已执行：{}（.tks: {}）", detail, line));
                        }
                        Err(e) => {
                            ctx.ui.emit(UiEvent::Step {
                                step: step_no,
                                state: StepState::Fail,
                                preview: preview.clone().unwrap_or_default(),
                                line: None,
                                duration_ms: Some(act_t0.elapsed().as_millis() as u64),
                                error: Some(e.to_string()),
                            });
                            tx.log("exec_error", serde_json::json!({ "round": round, "error": e.to_string() }));
                            sess.tool_result(primary.call_id.as_str(), format!("执行失败: {}", e));
                            continue; // 同一页面重试
                        }
                    }
                    // settle：等页面切换动画/加载稳定再重新采集，避免抓到上一页或半截动画。
                    // 断言/等待/隐藏键盘天生不改变页面——跳过等待（此前固定 700ms，每条断言步纯浪费）。
                    if !last_no_pagecheck {
                        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                    }
                    break; // 页面已变 → 重新采集
                }
            }
        }
    }

    let (success, reason) = finish.unwrap_or((false, format!("达到最大轮数({})未结束", ctx.max_rounds)));

    // approach A：探索结束后，据本轮各新建元素的实际作用统一生成 desc（创建时不写、不想当然）。
    // 复用同一会话——它已知道每个元素被点击后发生了什么。中断时也生成（元素已被点过，丢了可惜），
    // 给"退出中"提示；再按一次 Ctrl+C 会命中默认信号、强制退出。
    // desc 写进 element.json（持久化）；最终「元素库更新」框由上层据库中 desc 渲染。
    if !created.is_empty() {
        if aborted {
            ctx.ui.emit(UiEvent::ExitingNote { message: "退出中：正在为新建元素生成描述…（再次 Ctrl+C 强制退出）".to_string() });
        }
        let desc_msg = render(&ctx.prompts.message("explorer", "desc_pass"), &[("elements", &created.join("、"))]);
        tx.log("llm_message", serde_json::json!({ "content": desc_msg.clone() }));
        sess.user(desc_msg);
        // 失败不静默：desc 丢了元素仍可用（保留特征名），但要让用户知道这批元素没有描述
        let desc_warn = |why: &str| {
            ctx.ui.emit(UiEvent::Notice {
                level: Level::Warn,
                text: format!("元素描述生成失败（{}），本次新建元素暂无 desc", why),
            });
        };
        match sess.next().await {
            Ok(LlmReply::Text(t)) => {
                tx.log("desc_pass", serde_json::json!({ "content": t.clone() }));
                match parse_desc_json(&t) {
                    Some(obj) => {
                        for name in &created {
                            if let Some(d) = obj.get(name).and_then(|v| v.as_str()).map(str::trim) {
                                if !d.is_empty() {
                                    let _ = crate::tools::element::set_element_desc(ctx.element_path, name, d);
                                }
                            }
                        }
                    }
                    None => desc_warn("回复不是 JSON"),
                }
            }
            Ok(_) => desc_warn("模型没给文本回复"),
            Err(e) => desc_warn(&brief(&e.to_string(), 60)),
        }
    }

    Ok(DriveOutcome { success, reason, lines, steps, rounds: round, created, updated, aborted, script_name, renames, step_comments, subagent_pt, subagent_ct })
}
