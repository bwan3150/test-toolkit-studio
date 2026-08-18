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
    AssertPage,       // 断言页面（页面级校验：当前页与元素包里存的「页面」特征集命中率匹配——起始/终点校验的规范形式）
    Switch,           // 切换（web 标签 / App）
    ScrollFind,       // 滚动查找（朝某方向滚动直到目标文字出现——可复现地"滚到目标可见"，替代固定距离盲滑）
    Key,              // 按键（enter/tab/escape/backspace 等硬键/特殊键）
    Hover,            // 悬停（web 独有：鼠标移到元素上触发 hover，展开悬停下拉/菜单，不点击）
    Select,           // 选择（web 独有：选中 <select> 的某一项。原生下拉展开后选项由浏览器绘制、
                      //       DOM 里不可见，点击路线走不通，只能走 DOM 设值 + 派发事件）
    DialogAccept,     // 确认对话框（web 独有：原生 alert/confirm 的「确定」。这三种是浏览器画的、
                      //             不在 DOM 里，fetch 采不到，只能走这条专门的路）
    DialogDismiss,    // 取消对话框（web 独有：原生 confirm 的「取消」）
    DialogInput,      // 对话框输入（web 独有：往 prompt 里填字**并确定**——填完不确定等于没填）
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
            "选择" => Some(Self::Select),
            "清理" => Some(Self::Clear),
            "隐藏键盘" => Some(Self::HideKeyboard),
            "返回" => Some(Self::Back),
            "等待" => Some(Self::Wait),
            "断言" => Some(Self::Assert),
            "断言页面" => Some(Self::AssertPage),
            "切换" => Some(Self::Switch),
            "滚动查找" => Some(Self::ScrollFind),
            "确认对话框" => Some(Self::DialogAccept),
            "取消对话框" => Some(Self::DialogDismiss),
            "对话框输入" => Some(Self::DialogInput),
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
    /// 行内注释（`点击 [{1,2}] # 点开详情看是否跳转`）——写指令的人/AI 留下的**这一步在
    /// 干什么**。原样带进执行结果与 HTML 报告：复核的人光看命令看不出意图，这句话就是意图。
    pub note: Option<String>,
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
