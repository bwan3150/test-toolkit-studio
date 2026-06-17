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
    /// 定向滑动（up/down/left/right），从屏幕中心滑动
    SwipeDir { direction: String, distance: Option<i32> },
    /// 返回
    Back,
    /// 隐藏键盘
    HideKeyboard,
    /// 等待：固定毫秒，或等某元素出现
    Wait { ms: Option<u64>, element: Option<String> },
    /// 信息不足，主动请求当前页面截图
    RequestScreenshot { reason: String },
    /// 向用户反问
    AskUser { question: String },
    /// 结束探索（给出成功与否及依据）
    Finish { success: bool, reason: String },
}

impl AgentAction {
    /// AI 为本步动作填写的意图说明（desc/reason/question），用于 CLI 实时展示。
    /// 当模型未单独给出思考文字时，退而用此说明告诉用户"AI 想干啥"。
    pub fn intent(&self) -> Option<&str> {
        match self {
            AgentAction::Click { desc, .. }
            | AgentAction::Input { desc, .. }
            | AgentAction::LongPress { desc, .. }
            | AgentAction::Clear { desc, .. }
            | AgentAction::ClickVisual { desc, .. } => desc.as_deref(),
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
        )
    }
}
