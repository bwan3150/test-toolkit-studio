// 参数层 - 全局参数的唯一解析入口与统一参数表
//
// 设计（重构决策 3-ii）：CLI + config 的参数在此**解析一次**，形成一张统一参数表 `Params`；
// 后续各模块持 `&Params` **查表取参**，不再把 device/element/log 顺着函数签名层层透传。
// 没有 project 概念，只有参数；优先级：CLI 显式 > config > 内置默认。
//
// 元素库默认查找（决策 4A）与在线 OCR 默认地址（决策 5）的"单一来源"也在此——
// recognizer / tools::element 不再各自硬编码。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::config::{AiConfig, HarnessConfig, KnowledgeConfig, TkeConfig};

/// harness 各环节次数上限（解析后的具体值，含默认）。配置见 [harness] 段。
#[derive(Debug, Clone)]
pub struct HarnessLimits {
    /// 探索失败后「反思+从头重探」上限
    pub reexplore: usize,
    /// 验证/修复阶段「活体重探(修复)」上限
    pub repairs: usize,
    /// 稳定性测试需连续通过的次数
    pub stability: usize,
    /// 脚本医生单次诊断轮数上限
    pub doctor_iters: usize,
}

impl Default for HarnessLimits {
    fn default() -> Self {
        Self { reexplore: 1, repairs: 6, stability: 2, doctor_iters: 10 }
    }
}

impl HarnessLimits {
    fn from_config(c: &HarnessConfig) -> Self {
        let d = Self::default();
        Self {
            reexplore: c.reexplore.map(|v| v as usize).unwrap_or(d.reexplore),
            repairs: c.repairs.map(|v| v as usize).unwrap_or(d.repairs),
            // 稳定性至少 1 次、诊断至少 1 轮，避免配 0 导致退化
            stability: c.stability.map(|v| (v as usize).max(1)).unwrap_or(d.stability),
            doctor_iters: c.doctor_iters.map(|v| (v as usize).max(1)).unwrap_or(d.doctor_iters),
        }
    }
}

/// 进程级在线 OCR 地址表：main 启动时由 Params 设置一次，识别引擎深处查询，
/// 避免为单个 URL 穿透多层构造器（统一参数表"按需查询"的体现）。
static OCR_URL: OnceLock<String> = OnceLock::new();

/// 进程级 OCR 来源（在线 URL / 离线语言）：由 `tke run --ocr` / `tke harness --ocr` 设置一次，
/// 识别引擎（recognizer::ocr）查询以决定回放时元素/断言走 online 还是 offline。
/// 未设置则回退「在线 + OCR_URL」（保持旧行为）。
static OCR_SOURCE: OnceLock<crate::engines::ocr::OcrSource> = OnceLock::new();

/// 设置在线 OCR 地址（仅 main 启动时调用一次）
pub fn set_ocr_url(url: String) {
    let _ = OCR_URL.set(url);
}

/// 查询在线 OCR 地址（未设置则用内置默认）
pub fn ocr_url() -> String {
    OCR_URL
        .get()
        .cloned()
        .unwrap_or_else(|| DEFAULT_OCR_URL.to_string())
}

/// 设置进程级 OCR 来源（run/harness 处理器在跑脚本前调用一次）
pub fn set_ocr_source(src: crate::engines::ocr::OcrSource) {
    let _ = OCR_SOURCE.set(src);
}

/// 查询进程级 OCR 来源；未显式设置返回 None（调用方回退「在线 + ocr_url」）
pub fn ocr_source() -> Option<crate::engines::ocr::OcrSource> {
    OCR_SOURCE.get().cloned()
}

/// 元素库默认查找路径（相对当前目录）——全项目唯一一份
const DEFAULT_ELEMENT_PATHS: &[&str] = &["element.json", "locator/element.json"];

/// 在线 OCR 服务默认地址
const DEFAULT_OCR_URL: &str = "https://ocr.test-toolkit.app/ocr";

/// 统一参数表（解析一次，后续查表取参）
#[derive(Debug, Clone)]
pub struct Params {
    /// 目标设备 ID
    pub device: Option<String>,
    /// 元素库路径（显式来源；默认查找经 element_lib* 方法）
    element: Option<PathBuf>,
    /// 产物输出根目录（不设则 run/steps 不保存产物）
    pub log: Option<PathBuf>,
    /// 脚本输出目录（harness 生成 .tks 落点）
    pub scripts: Option<PathBuf>,
    /// 强制 NDJSON 输出
    pub json: bool,
    /// 在线 OCR 服务地址
    pub ocr_url: String,
    /// OCR 来源模式（online/offline/URL）；CLI --ocr 优先，否则用此。None=不跑 OCR
    pub ocr: Option<String>,
    /// harness 是否自检+自修复（config 默认；CLI --verify 出现则也为 true）
    pub verify: bool,
    /// harness 各环节次数上限
    pub harness: HarnessLimits,
    /// AI 配置
    pub ai: AiConfig,
    /// 记忆/知识库配置
    pub knowledge: KnowledgeConfig,
}

impl Params {
    /// 合并 CLI 显式参数与配置文件，解析出统一参数表
    /// 优先级：CLI 显式 > config > 内置默认
    pub fn resolve(
        cli_device: Option<String>,
        cli_element: Option<PathBuf>,
        cli_log: Option<PathBuf>,
        cli_scripts: Option<PathBuf>,
        json: bool,
        config: TkeConfig,
    ) -> Self {
        Self {
            device: cli_device.or(config.device),
            element: cli_element.or(config.element),
            log: cli_log.or(config.log),
            scripts: cli_scripts.or(config.scripts),
            json,
            ocr_url: config.ocr_url.unwrap_or_else(|| DEFAULT_OCR_URL.to_string()),
            ocr: config.ocr,
            verify: config.verify.unwrap_or(false),
            harness: HarnessLimits::from_config(&config.harness),
            ai: config.ai,
            knowledge: config.knowledge,
        }
    }

    /// 目标设备（克隆，便于按值传给库层）
    pub fn device(&self) -> Option<String> {
        self.device.clone()
    }

    /// 元素库路径（**读取语义**）：显式 > 默认查找已存在的 > None
    /// 用于 recognize / run / harness 等"读元素库"的场景；None 表示无元素库（仅坐标可用）
    pub fn element_lib(&self) -> Option<PathBuf> {
        if let Some(p) = &self.element {
            return Some(p.clone());
        }
        DEFAULT_ELEMENT_PATHS
            .iter()
            .map(PathBuf::from)
            .find(|p| p.exists())
    }

    /// 元素库路径（**写入语义**）：同上，但没有任何已存在文件时回退第一个默认路径以便创建
    /// 用于 element add 落库
    pub fn element_lib_for_write(&self) -> PathBuf {
        self.element_lib()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_ELEMENT_PATHS[0]))
    }

    /// 返回一个把元素库指向 `path` 的 Params 副本。
    /// harness 用它让诊断/验证的回放（ScriptRunner）也读**临时库**——否则探索把元素落到临时库、
    /// 回放却查正式库，会全部"元素未定义"。
    pub fn with_element_lib(&self, path: PathBuf) -> Self {
        let mut p = self.clone();
        p.element = Some(path);
        p
    }
}

/// 供库层在缺省时复用的默认查找（与 Params 同一份常量），用于尚未持有 Params 的入口
pub fn find_default_element_lib() -> Option<PathBuf> {
    DEFAULT_ELEMENT_PATHS
        .iter()
        .map(PathBuf::from)
        .find(|p: &PathBuf| Path::new(p).exists())
}
