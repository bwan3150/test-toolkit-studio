// 【定位自愈】ElementHealer 的 agent 层实现（Healenium 式，单次调用 agent 形状）：
// 回放某步元素连续定位失败 → 解析器回调这里 → 读库条目(当初的样子) + 当前实时页面元素 →
// 一次 LLM 挑选"哪个其实就是它"（只在确有把握时选，发明/乱指由 null 兜住）→
// 更新元素库条目(持久化,回包后以后的回放直接命中) + 返回实时坐标当场救活本步。
// 这是"修复必须接地在真实页面"原则在**定位层**的落点；流程级修复见 tksops 断点续探。

use std::path::PathBuf;
use std::sync::Mutex;

use crate::tools::element::{add_element_target, OcrChannel};
use crate::utils::Workarea;
use crate::workflow::tks::ElementHealer;
use crate::{AiConfig, Bounds, Fetcher, LlmReply, LlmSession, Point, UIElement};

use super::super::prompt::{render, PromptSet};
use super::fmt::{brief, parse_desc_json};

pub(crate) struct LlmElementHealer {
    ai: AiConfig,
    prompts: PromptSet,
    device: String,
    lib_path: PathBuf,
    fetcher: Fetcher,
    /// 本次运行自愈成功的元素名（上层汇报 + 决定是否回包）
    healed: Mutex<Vec<String>>,
}

impl LlmElementHealer {
    pub(crate) fn new(ai: AiConfig, prompts: PromptSet, device: String, lib_path: PathBuf) -> Self {
        Self { ai, prompts, device, lib_path, fetcher: Fetcher::new(), healed: Mutex::new(Vec::new()) }
    }

    /// 本次运行自愈成功的元素名
    pub(crate) fn healed_names(&self) -> Vec<String> {
        self.healed.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// 元素的落库通道（与探索落库一致）：OcrText→结构空+ocr 文字；其它→结构+ocr。
    fn channels(el: &UIElement) -> (Option<&UIElement>, OcrChannel) {
        if el.class_name == "OcrText" {
            let ocr = el.text.clone().filter(|t| !t.trim().is_empty()).map(OcrChannel::Text).unwrap_or(OcrChannel::FromCrop);
            (None, ocr)
        } else {
            let ocr = el
                .text
                .clone()
                .or_else(|| el.content_desc.clone())
                .filter(|t| !t.trim().is_empty())
                .map(OcrChannel::Text)
                .unwrap_or(OcrChannel::FromCrop);
            (Some(el), ocr)
        }
    }
}

#[async_trait::async_trait]
impl ElementHealer for LlmElementHealer {
    async fn heal(&self, element_name: &str, workarea: &Workarea) -> Option<(Point, Bounds)> {
        // 1) 库条目（当初的样子）——线索给 LLM 对照
        let lib: serde_json::Value = std::fs::read_to_string(&self.lib_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())?;
        let entry = lib["elements"].get(element_name)?;
        let desc = entry["desc"].as_str().unwrap_or("（无）");
        let clue = entry["ocr"].as_str().unwrap_or("（无）");

        // 2) 当前实时页面元素（解析器每次重试都刚采集过，工作区里就是最新的）
        let elements = self.fetcher.fetch_elements_from_file(&workarea.ui_tree_path()).ok()?;
        if elements.is_empty() {
            return None;
        }
        const MAX_LIST: usize = 120;
        let mut page = elements
            .iter()
            .take(MAX_LIST)
            .enumerate()
            .map(|(i, e)| format!("[{}] {}", i, brief(&e.to_ai_text(), 100)))
            .collect::<Vec<_>>()
            .join("\n");
        if elements.len() > MAX_LIST {
            page.push_str(&format!("\n（其余 {} 个略）", elements.len() - MAX_LIST));
        }

        // 3) 单次挑选（稳定的 agent 形状：输入齐全、一次判断、没把握就 null）
        let system = "你帮助判断：回放脚本找不到的元素，在当前页面上哪个其实就是它（应用改版/文字微调）。只在确有把握时选。".to_string();
        let mut sess = LlmSession::new_for_role(&self.ai, "healer", system, Vec::new()).ok()?;
        let ask = render(
            &self.prompts.message("verify", "heal_pick"),
            &[("name", element_name), ("desc", desc), ("clue", clue), ("page", &page)],
        );
        sess.user(ask);
        let reply = match sess.next().await {
            Ok(LlmReply::Text(t)) => t,
            _ => return None,
        };
        let idx = parse_desc_json(&reply)?.get("index")?.as_u64()? as usize;
        let el = elements.get(idx)?;

        // 4) 持久化修正（force=true 覆盖旧通道）——回包后以后的回放直接命中，不再走自愈
        let (structure, ocr) = Self::channels(el);
        if add_element_target(self.device.clone(), &self.lib_path, element_name, None, el.bounds.clone(), structure, ocr, true)
            .await
            .is_err()
        {
            return None; // 落库失败：不冒充自愈成功（当场坐标仍可用，但不持久——宁可让上层走续探）
        }
        if let Ok(mut v) = self.healed.lock() {
            if !v.contains(&element_name.to_string()) {
                v.push(element_name.to_string());
            }
        }
        Some((el.center(), el.bounds.clone()))
    }
}
