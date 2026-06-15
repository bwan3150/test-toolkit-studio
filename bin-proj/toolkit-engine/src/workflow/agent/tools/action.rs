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
