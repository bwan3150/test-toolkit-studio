// 执行结果和应用信息相关数据结构

use serde::{Deserialize, Serialize};

/// 一个**任务**的累积日志（`tke steps` 的 Task 布局）：
/// 一次检查要调很多次 steps，每次是一「批」。批次全部落在同一份 log.json 里，
/// 报告据此接成一条连续时间线（步骤编号也跨批次连续）。
/// 每批自带 `device`，所以跨设备检查**不需要按设备分目录**——顺序才是要还原的东西。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TaskLog {
    pub batches: Vec<ExecutionResult>,
    /// 这次检查**要验的是什么**（用户的原话/需求）。报告开头显示它——
    /// 没有它，人打开报告只看到一串点击，根本不知道当初想验证什么。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// 调用方 AI 给出的**结论**。**tke 自己判断不了**：
    /// 某一步定位没命中只说明"这次尝试无效"，换个方式点中了就没事；
    /// 而"功能真的坏了"只有走完全程的人/AI 才知道。见 `Verdict`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// 结论的一句话说明（"3 台 player 全部在线，指标正常" / "保存后列表不刷新，是 bug"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// 任务级结论。**只有这三种**，且都由调用方 AI 判断——
/// 步骤级的"没点中"不在此列（那是过程中的无效尝试，不是任务失败）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// 要验的功能确实可用
    Pass,
    /// **被测对象有问题**：功能坏了 / 复现了 bug / 用户说的问题属实
    Fail,
    /// 没验成：跑不下去（前提不满足、环境问题、被挡住了），不是被测对象的错
    Blocked,
}

impl Verdict {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pass" | "ok" | "通过" => Some(Self::Pass),
            "fail" | "bug" | "失败" => Some(Self::Fail),
            "blocked" | "block" | "受阻" => Some(Self::Blocked),
            _ => None,
        }
    }
    /// (徽章样式, 徽章文字)
    pub fn badge(self) -> (&'static str, &'static str) {
        match self {
            Self::Pass => ("b-ok", "通过"),
            Self::Fail => ("b-ng", "有问题"),
            Self::Blocked => ("b-wa", "未验成"),
        }
    }
}

impl TaskLog {
    /// 读已有 log.json；**兼容**旧的单批格式（整个文件就是一个 ExecutionResult）
    pub fn load(path: &std::path::Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        if let Ok(t) = serde_json::from_str::<Self>(&text) {
            return t;
        }
        serde_json::from_str::<ExecutionResult>(&text)
            .map(|r| Self { batches: vec![r], ..Default::default() })
            .unwrap_or_default()
    }
}

/// 脚本执行结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub case_id: String,
    pub script_name: String,
    pub start_time: String,
    pub end_time: String,
    pub steps: Vec<StepResult>,
    pub error: Option<String>,
    /// 脚本文件路径（工作流运行时记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    /// 本次运行的产物目录（工作流运行时记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_dir: Option<String>,
    /// 本次运行中启动过的 Android 包名（flow 收尾统一关闭用）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launched_packages: Vec<String>,
    /// 目标设备（报告里要说清"这次是在哪台设备上跑的"——同一份脚本在 web 和真机上
    /// 结果可能完全不同，光看截图未必分得出）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

/// 单步执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub index: usize,
    pub command: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    /// 脚本中的行号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// 本步标注后截图路径（相对 run_dir）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// 本步 UI 结构文件路径（相对 run_dir）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml: Option<String>,
    /// 这一步在干什么——来自 .tks 的行内注释（`点击 [...] # 点开详情看是否跳转`），
    /// 由写指令的人/AI 留下。报告直接展示它：光看命令看不出意图，这句话就是意图。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// AI 辅助驾驶：本步元素按原定位找不到、由 AI 依当前页面找回才通过（值 = 元素名）。
    /// 只影响本次执行，不改 .tks / .tklib——报告用，提示脚本定位可能需要更新。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub healed: Option<String>,
    /// 这一步**弹出了原生对话框**（alert/confirm/prompt），值 = 对话框上的文字。
    /// 它是浏览器画的、不在 DOM 里，截图和页面结构里都找不到——不在这儿说一句，
    /// 它就等于不存在，而它正挡着后面所有操作（P-37）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog: Option<String>,
}

/// 应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// 包名
    pub package_name: String,
    /// 版本名称 (如 8.8.76.667)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    /// 版本号 (如 105910845)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i64>,
    /// APK 路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apk_path: Option<String>,
    /// 启动 Activity (用于启动应用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_activity: Option<String>,
}

/// 当前聚焦的应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFocus {
    /// 包名
    pub package_name: String,
    /// Activity 名称
    pub activity_name: String,
    /// 完整的窗口信息 (原始输出)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_info: Option<String>,
}
