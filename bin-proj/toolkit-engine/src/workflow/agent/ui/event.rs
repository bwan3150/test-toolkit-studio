// 【UI 事件】面向渲染的引擎事件层（与 Transcript 落盘解耦）。
//
// 引擎不再直接 eprintln，而是在每个原打印点 emit 一个 UiEvent；前端（TUI / JSON / Plain）
// 各自消费。Transcript 仍负责 conversation.jsonl 全量落盘、完全不动——UiEvent 只含渲染所需，
// 必须 Send + Clone + 'static，可跨线程发到 TUI 渲染线程 / 序列化成 NDJSON。
//
// 设计原则：事件携带「语义」而非「旧 eprintln 的精确格式」。各前端用自己的统一风格渲染，
// 不复刻历史行式输出的逐字噪声（用户诉求就是把又乱又丑的旧输出重做）。

use serde::Serialize;

/// 阶段（状态栏顶部高亮 / 分隔标题）
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Explore,   // 初探
    Reexplore, // 重探（带 n）
    Diagnose,  // 医生诊断 / 修复
    Optimize,  // 反思官优化
    Verify,    // 稳定性回放
    Finalize,  // 命名定稿 + 提交元素库
}

impl Phase {
    /// 分隔标题用的中文名
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Explore => "探索",
            Phase::Reexplore => "重探",
            Phase::Diagnose => "诊断修复",
            Phase::Optimize => "路径优化",
            Phase::Verify => "稳定性验证",
            Phase::Finalize => "定稿",
        }
    }
}

/// 子 agent 种类（决定颜色 / 标签）
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubAgent {
    Asserter,   // 断言官
    Supervisor, // 监督官
    Reflector,  // 反思官
    Optimizer,  // 优化官
}

impl SubAgent {
    /// 英文标签（Plain/TUI 显示用；统一用英文名）
    pub fn label(&self) -> &'static str {
        match self {
            SubAgent::Asserter => "Asserter",
            SubAgent::Supervisor => "Supervisor",
            SubAgent::Reflector => "Reflector",
            SubAgent::Optimizer => "Optimizer",
        }
    }
}

/// 步骤三态
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Running,
    Ok,
    Fail,
}

/// 页面未就绪原因（冷启动还没 launch / 刚主动收尾关闭）
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotReady {
    SessionClosed, // 刚主动关闭/收尾——不催 launch
    NeedLaunch,    // 冷启动从没 launch——提示先 launch
}

/// token 用量（状态栏累加 + 单条尾注）
#[derive(Debug, Clone, Copy, Serialize, Default)]
pub struct Tokens {
    pub prompt: i64,
    pub completion: i64,
}

impl Tokens {
    pub fn new(prompt: i64, completion: i64) -> Self {
        Self { prompt, completion }
    }
}

/// 语义色级（前端各自映射成颜色 / 图标）
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Info, // 中性信息
    Ok,   // 成功 / 放行
    Warn, // 警告 / 打回 / 卡住
    Err,  // 失败 / 错误
    Dim,  // 弱化辅助
}

/// 结果框里的一行状态
#[derive(Debug, Clone, Serialize)]
pub struct StatusLine {
    pub level: Level,
    pub text: String,
}

impl StatusLine {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self { level, text: text.into() }
    }
}

/// 元素库提交项
#[derive(Debug, Clone, Serialize)]
pub struct ElementItem {
    pub name: String,
    pub desc: Option<String>,
}

/// 编排官 todo 项状态
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,    // 待办
    InProgress, // 进行中
    Done,       // 已完成
}

/// 编排官计划里的一个 todo 项（主 AI 把"探索/验证/收尾"等列成可见可勾选的计划）
#[derive(Debug, Clone, Serialize)]
pub struct TodoItem {
    pub text: String,
    pub status: TodoStatus,
}

/// 面向渲染的引擎事件。serde 外部标签 → NDJSON `{"type":"phase", ...}`。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiEvent {
    /// 阶段切换（前端打分隔标题 / 切状态栏高亮）。reexplore 带次数；其它 n=None
    Phase { phase: Phase, n: Option<u32> },

    /// 每轮页面观察头
    Page {
        round: usize,
        struct_elements: usize,   // 结构元素数（非 OCR）
        ocr_added: Option<usize>, // None=未开 OCR；Some(n)=本轮 OCR 新增伪元素数
        ocr_failed: bool,         // OCR 接口报错
        tabs: usize,              // web 标签页数（0=无）
        location: Option<String>, // 当前所处页/应用（web=活动标签标题或 URL；移动端暂 None）
        known: usize,             // 命中库的元素数（供 TUI 详情）
        unknown: usize,
        revisits: usize,
        not_ready: Option<NotReady>,
    },

    /// OCR 调用报错单列
    OcrError { round: usize, error: String },

    /// 卡住提示（连续无变化 / 重复操作 / 多页打转）
    Stuck { round: usize, message: String },

    /// 主探索 agent 一句思考 + 本次 token
    AgentThought { round: usize, text: String, tokens: Tokens },

    /// 主 AI（编排官）对用户说的一句话 —— 它是助手本体，渲染成纯文本：无 ● 标记、无名字、无专属色。
    Assistant { text: String, tokens: Tokens },

    /// 主 AI 的计划清单（探索/验证/收尾等步骤，可见可勾选）。每次替换整张清单。
    Todo { items: Vec<TodoItem> },

    /// 子 agent 一句话（断言官 / 监督官 / 反思官 / 医生 / 优化官）。
    /// text 已是要展示的核心句（如监督官「打回（第2次）：…」由引擎拼好），level 决定色调。
    SubAgent {
        kind: SubAgent,
        level: Level,
        text: String,
        tokens: Tokens,
    },

    /// 步骤执行：Running 先发(带 step/preview，供 TUI 显示进行中)，随后 Ok/Fail 发同 step。
    Step {
        step: usize,
        state: StepState,
        preview: String,        // "点击 登录按钮" 友好预览
        line: Option<String>,   // 落库 .tks 行（Ok 时）
        duration_ms: Option<u64>,
        error: Option<String>,  // Fail 时
    },

    /// 元素改名（自动名 → 语义名）
    Rename { old: String, new: String },

    /// 自动停止 / 强制结束 / 中断
    AutoStop { reason: String },

    /// 退出中（为新建元素生成 desc）
    ExitingNote { message: String },

    /// 通用信息行（诊断/验证/优化等阶段的杂项输出，按 level 上色）
    Notice { level: Level, text: String },

    /// 引擎等用户输入（ask_user / setup 触发；前端必须回 UiCommand::Answer）。
    /// options 非空=候选项（设备选择等），前端可渲染成方向键可选列表；空=纯文本输入。
    /// free_input=true 时列表末尾带一个**可直接打字**的输入行（选项 or 自由输入二选一）。
    AwaitingInput { round: usize, question: String, options: Vec<String>, #[serde(default)] free_input: bool },

    /// 指导已采纳回显（收到 Guidance 后引擎确认）
    GuidanceAccepted { text: String },

    /// setup 完成后的会话参数（发一次）：设备/平台一行展示；模型/供应商/推理不灌消息流，
    /// TUI 存起来供 `/model` 查询（JSON 前端全量透出给 app）。不携带用例文本——用户的话
    /// 已有输入回显，重复展示只添乱。
    SessionInfo { device: String, platform: String, model: String, provider: String, reasoning: String },

    /// 脚本生成完毕
    ScriptGenerated { name: String, steps: usize, success: bool },

    /// 最终结果框（三层状态）
    Summary {
        explore: StatusLine,
        diagnose: StatusLine,
        verify: StatusLine,
        reason: String,
        model: String,
        tokens: Tokens, // 全程总量
    },

    /// 元素库更新框
    Elements {
        committed: usize,
        items: Vec<ElementItem>,
        committed_to_lib: bool, // false=未稳定通过,未提交
    },

    /// 引擎整体收束（前端据此退出 alt-screen / 关 NDJSON 流）
    Done {
        success: bool,
        script: Option<String>,
        conversation: String,
    },
}
