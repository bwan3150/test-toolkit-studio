// 参数层 - 全局参数的唯一解析入口与统一参数表
//
// 设计（重构决策 3-ii）：CLI + config 的参数在此**解析一次**，形成一张统一参数表 `Params`；
// 后续各模块持 `&Params` **查表取参**，不再把 device/element/log 顺着函数签名层层透传。
// 没有 project 概念，只有参数；优先级：CLI 显式 > config > 内置默认。
//
// 元素库默认查找（决策 4A）与在线 OCR 默认地址（决策 5）的"单一来源"也在此——
// recognizer / tools::element 不再各自硬编码。

use std::path::PathBuf;
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
/// 注意：这里只存纯 String；带类型的 OCR 来源注册表在 engines::ocr（set_ocr_source/ocr_source），
/// utils 不 import 上层 engines 的类型（分层：utils 在 engines 之下）。
static OCR_URL: OnceLock<String> = OnceLock::new();

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

/// 在线 OCR 服务默认地址
const DEFAULT_OCR_URL: &str = "https://ocr.test-toolkit.app/ocr";

/// 统一参数表（解析一次，后续查表取参）
#[derive(Debug, Clone)]
pub struct Params {
    /// 目标设备 ID
    pub device: Option<String>,
    /// 元素库路径（仅运行期内部注入：装配层解包 .tklib 后 with_element_lib 写入；无 CLI/config 来源）
    element: Option<PathBuf>,
    /// 产物输出根目录（不设则 run/steps 不保存产物）
    pub log: Option<PathBuf>,
    /// 脚本输出目录（harness 生成 .tks 落点）
    pub scripts: Option<PathBuf>,
    /// 缓存目录：运行中间文件（截图/页面/会话日志/临时元素库）落点；不设用系统临时目录。
    /// 这些只是运行中产物、不展示给用户，用 cache_root() 取最终落点。
    pub cache: Option<PathBuf>,
    /// 工作区目录：AI 文件操作（.tks/save_file 等）的范围根；不设用进程当前目录。用 workspace_root() 取。
    pub current_dir: Option<PathBuf>,
    /// 强制 NDJSON 输出
    pub json: bool,
    /// 在线 OCR 服务地址
    pub ocr_url: String,
    /// OCR 来源模式（online/offline/URL）；CLI --ocr 优先，否则用此。None=不跑 OCR
    pub ocr: Option<String>,
    /// harness 是否自检+自修复（config 默认；CLI --verify 出现则也为 true）
    pub verify: bool,
    /// AI 辅助驾驶（run/flow 回放的定位自愈）：CLI --copilot > config copilot > 默认 true。
    /// 开启且配置了 [ai] 时，回放中元素定位失败会让 AI 按当前页面修正并回写元素包。
    pub copilot: bool,
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
        cli_log: Option<PathBuf>,
        cli_scripts: Option<PathBuf>,
        cli_cache: Option<PathBuf>,
        cli_current_dir: Option<PathBuf>,
        json: bool,
        cli_copilot: Option<bool>,
        config: TkeConfig,
    ) -> Self {
        Self {
            device: cli_device.or(config.device),
            element: None, // 元素库无 CLI/config 来源——运行期由装配层解包 .tklib 后 with_element_lib 注入
            log: cli_log.or(config.log),
            scripts: cli_scripts.or(config.scripts),
            cache: cli_cache.or(config.cache),
            current_dir: cli_current_dir.or(config.current_dir),
            json,
            ocr_url: config.ocr_url.unwrap_or_else(|| DEFAULT_OCR_URL.to_string()),
            ocr: config.ocr,
            verify: config.verify.unwrap_or(false),
            copilot: cli_copilot.or(config.copilot).unwrap_or(true),
            harness: HarnessLimits::from_config(&config.harness),
            ai: config.ai,
            knowledge: config.knowledge,
        }
    }

    /// 目标设备（克隆，便于按值传给库层）
    pub fn device(&self) -> Option<String> {
        self.device.clone()
    }

    /// 元素库路径：**只认显式指定**（-e / config / 装配层解包 .tklib 后 with_element_lib 注入）。
    /// None = 无元素库（仅坐标步可用）。
    /// 「共享库默认查找」已彻底删除（方案定稿 2026-07-03）：每个脚本自持 `foo.tklib` 元素包，
    /// 不存在跨脚本共享的可变元素库——旧脚本的定位依据永远不会被新脚本的写入污染。
    pub fn element_lib(&self) -> Option<PathBuf> {
        self.element.clone()
    }

    /// 返回一个把元素库指向 `path` 的 Params 副本。
    /// harness 用它让诊断/验证的回放（ScriptRunner）也读**临时库**——否则探索把元素落到临时库、
    /// 回放却查正式库，会全部"元素未定义"。
    pub fn with_element_lib(&self, path: PathBuf) -> Self {
        let mut p = self.clone();
        p.element = Some(path);
        p
    }

    /// 返回一个把设备覆盖为 `device` 的 Params 副本。
    /// harness 交互向导选的设备（如 web）没经 -d，必须传播给诊断/验证回放（ScriptRunner），
    /// 否则回放取不到设备、会退回默认 adb（web 用例就会报「adb: no devices」）。
    pub fn with_device(&self, device: Option<String>) -> Self {
        let mut p = self.clone();
        p.device = device;
        p
    }

    /// 覆盖产物日志目录（准备阶段向导里用户填/跳过后回填；None=不输出产物）
    pub fn with_log(&self, log: Option<PathBuf>) -> Self {
        let mut p = self.clone();
        p.log = log;
        p
    }

    /// 运行中间文件的落点根目录：显式 --cache 优先，否则系统临时目录下的 `tke/cache`。
    /// 截图/页面结构/会话日志/临时元素库等都落这里——只是运行中产物，不展示给用户。
    pub fn cache_root(&self) -> PathBuf {
        self.cache
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("tke").join("cache"))
    }

    /// 工作区根目录：AI 文件操作（.tks/save_file/read/edit/delete）的范围根。
    /// 显式 --current-dir 优先（app spawn 用）；否则进程当前目录（CLI/TUI 直接用）。
    pub fn workspace_root(&self) -> PathBuf {
        self.current_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}
