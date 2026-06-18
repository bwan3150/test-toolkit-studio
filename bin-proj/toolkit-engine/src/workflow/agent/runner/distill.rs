// 【脚本提炼】探索达成后，把"试错流水账"提炼成一条最短、干净、可复用的回放脚本。
//
// 探索阶段为了摸路会有重复点击、空滑、走错又绕回的步骤，直接当脚本回放既脏又易飘。
// 这里复用同一会话（AI 已知道每步对页面的实际影响），让它：
//   ① 从已执行步骤里挑出构成"最短成功路径"的编号（去掉冗余/无效/绕路）；
//   ② 选一个"只有真正达成目标后才出现"的已知元素，在结尾加 `断言 [{元素}, 存在]`，
//      让脚本能自证目标达成——验证回放时这一步过不了就说明没真完成（而非"无报错即通过"）。
//
// 产出干净脚本后再交给 verify 回放校验。AI 只挑编号、不手写 .tks，稳妥不易出格式错。

use std::io::IsTerminal;
use std::path::Path;

use crate::{LlmReply, LlmSession, LocatorStrategy, TksCommand, TksParam, TksStep};

use super::super::transcript::Transcript;
use super::flow::{friendly, paint, parse_desc_json};

/// 把录制的步骤提炼成干净脚本（含可选目标断言）。失败/无效则原样返回。
pub async fn distill_script(
    sess: &mut LlmSession,
    tx: &mut Transcript,
    recorded: &[String],
    case: &str,
    element_path: &Path,
) -> Vec<String> {
    let tty = std::io::stderr().is_terminal();
    if recorded.len() < 2 {
        return recorded.to_vec();
    }

    let listing = recorded
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, friendly(l)))
        .collect::<Vec<_>>()
        .join("\n");
    sess.user(format!(
        "探索已达成目标。现在请把它**提炼成一条最短、干净、可稳定复用的回放脚本**。\n\
         你执行过的步骤如下（编号）：\n{}\n\n\
         请从中挑出真正构成**最短成功路径**的步骤编号（按执行顺序），去掉：重复点击、\
         没让页面前进的空步、走错又绕回来的弯路。再选一个**只有真正达成目标后才会出现**\
         的已知元素名，用于在脚本结尾断言“目标确实达成”（实在没有合适的就填 null）。\n\
         只返回一个 JSON：{{\"keep\":[步号,...], \"assert_element\":\"元素名\"或null}}；\
         不要调用工具、不要 ``` 围栏或多余文字。整体测试目标：{}",
        listing, case
    ));

    let reply = match sess.next().await {
        Ok(LlmReply::Text(t)) => t,
        _ => return recorded.to_vec(),
    };
    tx.log("distill", serde_json::json!({ "content": reply.clone() }));

    let obj = match parse_desc_json(&reply) {
        Some(o) => o,
        None => return recorded.to_vec(),
    };

    // 挑出的步骤编号（按 AI 给的顺序，越界/无效的丢弃）
    let keep: Vec<usize> = obj["keep"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).map(|n| n as usize).collect())
        .unwrap_or_default();
    let mut clean: Vec<String> = keep
        .iter()
        .filter(|&&i| i >= 1 && i <= recorded.len())
        .map(|&i| recorded[i - 1].clone())
        .collect();
    if clean.is_empty() {
        // AI 没给有效编号 → 退回原脚本，别把脚本提炼没了
        return recorded.to_vec();
    }
    let kept = clean.len();

    // 目标断言：仅当 assert_element 是库里**已存在**的元素才追加（否则回放断言无从解析）
    let mut asserted = false;
    if let Some(name) = obj["assert_element"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
    {
        if element_exists(element_path, name) {
            clean.push(assert_line(name));
            asserted = true;
        }
    }

    eprintln!();
    eprintln!(
        "{}",
        paint(
            tty,
            "1;36",
            &format!(
                "▶ 脚本提炼：{} 步 → {} 步{}",
                recorded.len(),
                kept,
                if asserted { " + 目标断言" } else { "（无目标断言）" }
            )
        )
    );
    tx.log(
        "distill_result",
        serde_json::json!({ "from": recorded.len(), "kept": kept, "asserted": asserted }),
    );
    clean
}

/// 构造 `断言 [{name}, "存在"]` 行（经序列化器，保证可被 run 回放）
fn assert_line(name: &str) -> String {
    crate::workflow::step_to_source(&TksStep {
        command: TksCommand::Assert,
        params: vec![
            TksParam::Element { name: name.to_string(), strategy: LocatorStrategy::Auto },
            TksParam::Text("存在".to_string()),
        ],
        raw: String::new(),
        line_number: 0,
    })
}

/// 元素库里是否已存在该名字的元素
fn element_exists(lib_path: &Path, name: &str) -> bool {
    std::fs::read_to_string(lib_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|lib| lib["elements"][name].is_object())
        .unwrap_or(false)
}
