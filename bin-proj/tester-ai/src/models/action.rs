// 操作模型

use serde::{Deserialize, Serialize};

/// Worker Agent 返回的操作决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDecision {
    /// 操作类型
    pub action_type: ActionType,

    /// 目标元素 ID (对应 MergedElement 的 id)
    pub target_element_id: Option<usize>,

    /// 额外参数
    pub params: ActionParams,

    /// 决策理由
    pub reasoning: String,

    /// 是否认为测试完成
    pub test_completed: bool,
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// 点击
    Click,
    /// 按压
    Press,
    /// 滑动
    Swipe,
    /// 拖动
    Drag,
    /// 定向拖动
    DirectionalDrag,
    /// 输入文字
    Input,
    /// 清理输入框
    Clear,
    /// 隐藏键盘
    HideKeyboard,
    /// 等待
    Wait,
    /// 返回
    Back,
    /// 启动 App
    Launch,
    /// 关闭 App
    Stop,
    /// 断言
    Assert,
    /// 读取文本
    ReadText,
    /// 无操作（测试完成时使用）
    None,
}

/// 操作参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ActionParams {
    /// 输入的文本 (用于 Input)
    pub text: Option<String>,

    /// 按压时长 (ms) (用于 Press)
    pub duration: Option<u32>,

    /// 终点坐标 (用于 Swipe, Drag)
    pub to: Option<(i32, i32)>,

    /// 方向 (用于 DirectionalDrag)
    pub direction: Option<Direction>,

    /// 距离 (用于 DirectionalDrag)
    pub distance: Option<i32>,

    /// 断言条件 (用于 Assert)
    pub assert_condition: Option<AssertCondition>,

    /// 包名 (用于 Launch, Stop)
    pub package: Option<String>,

    /// Activity (用于 Launch)
    pub activity: Option<String>,
}

/// 方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// 断言条件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertCondition {
    /// 存在
    Exists,
    /// 不存在
    NotExists,
    /// 可见
    Visible,
    /// 不可见
    NotVisible,
}
