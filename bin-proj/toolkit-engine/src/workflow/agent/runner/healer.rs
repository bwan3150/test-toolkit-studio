// 【定位自愈】ElementHealer 的 agent 层实现（Healenium 式，单次调用 agent 形状）：
// 回放某步元素连续定位失败 → 解析器回调这里 → 分层判断（每层一次 LLM 调用，命中即停）：
//
//   第一段 pick（同元素找回）：读库条目(当初的样子) + 当前实时页面元素 → 挑"哪个其实就是它"
//     （文字微调/换位置/换层级）→ 更新元素库条目 + 返回实时坐标当场救活本步。
//   第二段 triage（分诊，仅 tke run 辅助驾驶开启）：pick 没把握时，结合**脚本全文**再判——
//     replace     = 同页有功能等价的**别的元素**能进下一步 → 救活（记账标注替代，**不落库**：
//                   它不是"它"，把替代元素写进原元素名会污染库）
//     wrong_page  = 前面某步走偏，本步开始就不在该在的页 → 不救，出诊断
//     path_changed= 功能入口/路径被整体重构，原路径不存在 → 不救，出诊断
//     app_issue   = 页面对、路径对，元素消失/不可交互——脚本可能测出了 App 真实缺陷 → 出诊断
//     unknown     = 都对不上，未知路径错误 → 出诊断
//   诊断经 take_diagnosis() 由上层拼进该步报错，让人看到失败的真正原因。
//
// 这是"修复必须接地在真实页面"原则在**定位层**的落点；流程级修复见 tksops 断点续探。
// harness 的 replay/repair 只用第一段（那边有轨迹报告+编排官决策体系，分诊会与之打架）。

use std::path::PathBuf;
use std::sync::Mutex;

use crate::tools::element::{add_element_target, OcrChannel};
use crate::utils::Workarea;
use crate::workflow::tks::ElementHealer;
use crate::{AiConfig, Bounds, Fetcher, Point, UIElement};

use super::super::prompt::{render, PromptSet, PromptSpec};
use super::fmt::brief;

/// 【AI 辅助驾驶】纯回放（tke run / flow）的定位自愈装配入口。
/// ScriptRunner 解包 .tklib 后以解包出的元素库 json 路径调用（构造需要库路径，而解包发生在
/// 其内部，故经工厂延迟构造，见 ScriptRunner::with_healer_factory）。
/// 设备缺省不拦（adb 单设备默认，与 tke run 本身同一容忍度）——曾把它当硬前提，
/// 用户不带 -d 时自愈全程静默失效。提示词走 config [ai].prompts_dir（与 harness 同一套覆盖机制）。
pub fn copilot_healer(
    params: &crate::Params,
    lib_json: PathBuf,
    script_text: &str,
) -> Option<std::sync::Arc<dyn ElementHealer>> {
    let device = params.device().unwrap_or_default();
    let prompts = PromptSet::resolve(&PromptSpec {
        prompts_dir: params.ai.prompts_dir.clone().map(PathBuf::from),
        ..Default::default()
    })
    .ok()?;
    Some(std::sync::Arc::new(
        LlmElementHealer::new(params.ai.clone(), prompts, device, lib_json)
            .with_triage(script_text.to_string()),
    ))
}

pub(crate) struct LlmElementHealer {
    ai: AiConfig,
    prompts: PromptSet,
    device: String,
    lib_path: PathBuf,
    fetcher: Fetcher,
    /// 本次运行自愈成功的元素名（上层汇报 + 决定是否回包）
    healed: Mutex<Vec<String>>,
    /// 分诊上下文（Some(脚本全文) = 开启第二段 triage；harness 的 replay/repair 不开）
    script: Option<String>,
    /// 最近一次分诊的诊断结论（该步救不活时写入，上层 take 后拼进报错）
    diagnosis: Mutex<Option<String>>,
}

impl LlmElementHealer {
    pub(crate) fn new(ai: AiConfig, prompts: PromptSet, device: String, lib_path: PathBuf) -> Self {
        Self {
            ai,
            prompts,
            device,
            lib_path,
            fetcher: Fetcher::new(),
            healed: Mutex::new(Vec::new()),
            script: None,
            diagnosis: Mutex::new(None),
        }
    }

    /// 开启第二段分诊（builder 式）：带上脚本全文作为判断上下文（tke run 辅助驾驶装配时用）
    pub(crate) fn with_triage(mut self, script_text: String) -> Self {
        self.script = Some(script_text);
        self
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
    /// 本次运行自愈成功的元素名（供上层逐步提示 + 回包 .tklib）
    fn healed(&self) -> Vec<String> {
        self.healed_names()
    }

    /// 分诊诊断：取走即清空（只归属当前失败步）
    fn take_diagnosis(&self) -> Option<String> {
        self.diagnosis.lock().ok().and_then(|mut d| d.take())
    }

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

        // 3) 第一段 pick：同元素找回（没把握就 null，进第二段）
        if let Some(el) = self.pick_same(element_name, desc, clue, &page, &elements).await {
            // 持久化修正（force=true 覆盖旧通道）——harness 回包后以后的回放直接命中；
            // tke run 场景写的是解包临时副本，原 .tklib 不动
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
            return Some((el.center(), el.bounds.clone()));
        }

        // 4) 第二段 triage：分诊（仅 tke run 辅助驾驶开启；harness replay/repair 到此为止）
        let script = self.script.as_deref()?;
        self.triage(element_name, desc, clue, &page, &elements, script).await
    }
}

impl LlmElementHealer {
    /// 第一段：单次挑选"当前页面上哪个其实就是它"（强制工具调用；没把握 = None）
    async fn pick_same<'a>(
        &self,
        element_name: &str,
        desc: &str,
        clue: &str,
        page: &str,
        elements: &'a [UIElement],
    ) -> Option<&'a UIElement> {
        let system = "你帮助判断：回放脚本找不到的元素，在当前页面上哪个其实就是它（应用改版/文字微调）。只在确有把握时选。".to_string();
        let ask = render(
            &self.prompts.message("verify", "heal_pick"),
            &[("name", element_name), ("desc", desc), ("clue", clue), ("page", page)],
        );
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "index": { "type": ["integer", "null"], "description": "当前页面上就是它的元素序号；页面上没有对应物就 null（宁可不修也别乱指）" },
                "reason": { "type": "string", "description": "一句话依据" }
            },
            "required": ["reason"]
        });
        let (obj, _pt, _ct) = super::oneshot::one_shot(&self.ai, "healer", system, "提交自愈挑选结果", schema, ask).await;
        let idx = obj?.get("index")?.as_u64()? as usize;
        elements.get(idx)
    }

    /// 第二段：分诊——结合脚本全文判断失败真因。
    /// replace = 同页功能等价的替代元素 → 救活（记账标注替代，**不落库**：把替代元素写进
    /// 原元素名会污染库）；其余 verdict → 写诊断（上层 take 后拼进该步报错），不救。
    async fn triage(
        &self,
        element_name: &str,
        desc: &str,
        clue: &str,
        page: &str,
        elements: &[UIElement],
        script: &str,
    ) -> Option<(Point, Bounds)> {
        let system = "你帮助分诊：回放脚本某步找不到元素且页面上没有'就是它'的对应物。结合脚本上下文判断失败真因：同页替代/前面走偏/路径重构/应用问题/未知。".to_string();
        let ask = render(
            &self.prompts.message("verify", "heal_triage"),
            &[("name", element_name), ("desc", desc), ("clue", clue), ("page", page), ("script", script)],
        );
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "verdict": { "type": "string", "enum": ["replace", "wrong_page", "path_changed", "app_issue", "unknown"], "description": "分诊结论（五选一，按提示中的顺序判断）" },
                "index": { "type": ["integer", "null"], "description": "verdict=replace 时：当前页面上可替代的元素序号；其余 verdict 为 null" },
                "diagnosis": { "type": "string", "description": "给测试人员的诊断：依据 + 建议，一两句话" }
            },
            "required": ["verdict", "diagnosis"]
        });
        let (obj, _pt, _ct) = super::oneshot::one_shot(&self.ai, "healer", system, "提交分诊结果", schema, ask).await;
        let obj = obj?;
        let verdict = obj.get("verdict")?.as_str().unwrap_or("unknown").to_string();
        let diagnosis = obj.get("diagnosis").and_then(|d| d.as_str()).unwrap_or("").to_string();

        if verdict == "replace" {
            if let Some(el) = obj.get("index").and_then(|i| i.as_u64()).and_then(|i| elements.get(i as usize)) {
                // 替代救活：记账标注"走了替代路径"（报告可见），不写元素库
                let label = el
                    .text
                    .clone()
                    .or_else(|| el.content_desc.clone())
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| "无文字元素".into());
                if let Ok(mut v) = self.healed.lock() {
                    v.push(format!("{}→替代「{}」", element_name, brief(&label, 40)));
                }
                return Some((el.center(), el.bounds.clone()));
            }
            // verdict=replace 但 index 无效：按 unknown 落诊断
        }

        // 不救：写诊断（带结论标签），上层在该步最终失败时取走拼进报错
        let tag = match verdict.as_str() {
            "wrong_page" => "疑前面步骤走偏（本步开始时已不在预期页面）",
            "path_changed" => "疑路径已整体重构（原入口/流程不存在了）",
            "app_issue" => "疑 App 问题（脚本可能测出了真实缺陷）",
            _ => "未知路径错误",
        };
        if let Ok(mut d) = self.diagnosis.lock() {
            *d = Some(if diagnosis.is_empty() { tag.to_string() } else { format!("{}：{}", tag, diagnosis) });
        }
        None
    }
}
