// AI 决策出的强类型动作

/// AI 决策出的一个动作
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// 启动应用/打开网址：android=包名[,activity]，web=URL，ios=bundleId
    Launch { target: String, activity: Option<String> },
    /// 关闭应用/销毁会话
    Close { target: String },
    /// 点击元素
    Click { element_id: usize, name: String, desc: Option<String> },
    /// 在元素处输入文本
    Input { element_id: usize, name: String, desc: Option<String>, text: String },
    /// 长按元素
    LongPress { element_id: usize, name: String, desc: Option<String>, duration_ms: u64 },
    /// 清空输入框
    Clear { element_id: usize, name: String, desc: Option<String> },
    /// 断言某元素在当前页面存在/不存在——用于给脚本加**闭环校验**：到达关键页/目标页后，
    /// 断言该页独有元素存在，回放时若没真正到达就会在此失败（而不是悄悄跑偏）。
    /// exist=true 断言存在（默认），false 断言不存在。落库该元素，产出 `断言 [{name}, 存在]`。
    Assert { element_id: usize, name: String, desc: Option<String>, exist: bool },
    /// 看图后视觉点击：当结构元素与 OCR 都定位不到目标时，多模态 AI 看截图后
    /// 给出目标框 region=[x1,y1,x2,y2]（优先）或点击点 (x,y)。
    /// 落库为纯 img 元素（结构/ocr 通道空），换设备靠图像模板匹配回放。
    ClickVisual {
        region: Option<[i32; 4]>,
        x: Option<i32>,
        y: Option<i32>,
        name: String,
        desc: Option<String>,
    },
    /// 定向滑动（up/down/left/right），从屏幕中心滑动。
    /// amount=屏幕比例 full/half/quarter（推荐，免算像素）；distance=像素（兜底）
    SwipeDir { direction: String, distance: Option<i32>, amount: Option<String> },
    /// 滚动查找：朝某方向滚动直到目标文字出现（可复现地"滚到目标可见"，回放与探索滚动位置一致）。
    /// 替代"盲滑 N 次找东西"——目标在更下方就用 direction=up 滚动找它。落 .tks `滚动查找 ["文字", 方向]`。
    SwipeToFind { target: String, direction: String },
    /// 在某元素上朝某方向滑动（列表项左滑删除、拖滑块、轮播翻页等）。
    /// 与 swipe_direction 不同：从该**元素**起滑、回放时实时解析其位置，落 `定向滑动 [{元素}, 方向, 距离]`。
    SwipeElement { element_id: usize, direction: String, distance: Option<i32>, amount: Option<String> },
    /// 从元素A拖到元素B（拖拽排序/拖放）。落 `滑动 [{元素A}, {元素B}]`，回放实时解析两端位置。
    Drag { from_id: usize, to_id: usize },
    /// 按下一个按键（enter/tab/escape/backspace 等）。常用于输入后回车提交。落 `按键 ["enter"]`。
    PressKey { key: String },
    /// 返回
    Back,
    /// 切换标签/App：web=标签序号 或 用新标签打开 URL；移动端=切到目标 App 包名
    Switch { target: String },
    /// 隐藏键盘
    HideKeyboard,
    /// 等待：固定毫秒，或等某元素出现
    Wait { ms: Option<u64>, element: Option<String> },
    /// 信息不足，主动请求当前页面截图
    RequestScreenshot { reason: String },
    /// 向用户反问
    AskUser { question: String },
    /// 结束探索（给出成功与否及依据）；script_name=给生成脚本起的简短文件名（不含扩展名）
    Finish { success: bool, reason: String, script_name: Option<String> },
    /// 纠正已知元素的名字（当初起错了名，如把 logo 当成导航选项）。不改变页面、不产生 .tks 步骤。
    Rename { old_name: String, new_name: String },
}

impl AgentAction {
    /// 控制流动作自带的说明（finish/截图/反问的 reason/question），用于 CLI 展示兜底。
    /// 设备动作的"这步为什么"由 comment 表达（横切字段，不在此），desc 是元素描述、不算意图。
    pub fn intent(&self) -> Option<&str> {
        match self {
            AgentAction::RequestScreenshot { reason }
            | AgentAction::Finish { reason, .. } => Some(reason.as_str()),
            AgentAction::AskUser { question } => Some(question.as_str()),
            _ => None,
        }
    }

    /// 是否需要再次采集页面（执行了改变页面的设备动作后为 true）
    pub fn changes_page(&self) -> bool {
        !matches!(
            self,
            AgentAction::RequestScreenshot { .. }
                | AgentAction::AskUser { .. }
                | AgentAction::Finish { .. }
                | AgentAction::Rename { .. }
        )
    }
}
