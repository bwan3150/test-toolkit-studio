// TKS脚本相关数据结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use super::{Point, LocatorStrategy};

/// TKS脚本命令类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TksCommand {
    Launch,           // 启动
    Close,            // 关闭
    Click,            // 点击
    Press,            // 按压
    Swipe,            // 滑动
    DirectionalSwipe, // 定向滑动
    Input,            // 输入
    Clear,            // 清理
    HideKeyboard,     // 隐藏键盘
    Back,             // 返回
    Wait,             // 等待
    Assert,           // 断言
    Switch,           // 切换（web 标签 / App）
}

impl TksCommand {
    /// 从中文命令字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "启动" => Some(Self::Launch),
            "关闭" => Some(Self::Close),
            "点击" => Some(Self::Click),
            "按压" => Some(Self::Press),
            "滑动" => Some(Self::Swipe),
            "定向滑动" => Some(Self::DirectionalSwipe),
            "输入" => Some(Self::Input),
            "清理" => Some(Self::Clear),
            "隐藏键盘" => Some(Self::HideKeyboard),
            "返回" => Some(Self::Back),
            "等待" => Some(Self::Wait),
            "断言" => Some(Self::Assert),
            "切换" => Some(Self::Switch),
            _ => None,
        }
    }
}

/// TKS脚本参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TksParam {
    /// 纯文本
    Text(String),
    /// 数字
    Number(i32),
    /// 持续时间（毫秒）
    Duration(u32),
    /// 坐标 {x,y}
    Coordinate(Point),
    /// 统一元素引用: {元素名} 或 {元素名}&策略
    Element {
        name: String,
        strategy: LocatorStrategy,
    },
    /// 方向 up/down/left/right
    Direction(String),
    /// 布尔值
    Boolean(bool),
}

/// TKS脚本步骤
#[derive(Debug, Clone)]
pub struct TksStep {
    pub command: TksCommand,
    pub params: Vec<TksParam>,
    pub raw: String,
    pub line_number: usize,
}

/// TKS脚本
#[derive(Debug, Clone)]
pub struct TksScript {
    pub case_id: String,
    pub script_name: String,
    pub details: HashMap<String, String>,
    pub steps: Vec<TksStep>,
    pub file_path: Option<PathBuf>,
}
