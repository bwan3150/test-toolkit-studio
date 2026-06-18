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
