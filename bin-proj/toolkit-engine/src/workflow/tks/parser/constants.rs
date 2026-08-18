// 解析器常量定义 - 命令和方向映射

use crate::TksCommand;
use std::collections::HashMap;

/// 创建命令映射表（中文命令 → TksCommand）
pub fn create_command_map() -> HashMap<String, TksCommand> {
    let mut map = HashMap::new();
    map.insert("启动".to_string(), TksCommand::Launch);
    map.insert("关闭".to_string(), TksCommand::Close);
    map.insert("点击".to_string(), TksCommand::Click);
    map.insert("悬停".to_string(), TksCommand::Hover);
    map.insert("选择".to_string(), TksCommand::Select);
    map.insert("按压".to_string(), TksCommand::Press);
    map.insert("滑动".to_string(), TksCommand::Swipe);
    map.insert("定向滑动".to_string(), TksCommand::DirectionalSwipe);
    map.insert("输入".to_string(), TksCommand::Input);
    map.insert("清理".to_string(), TksCommand::Clear);
    map.insert("隐藏键盘".to_string(), TksCommand::HideKeyboard);
    map.insert("返回".to_string(), TksCommand::Back);
    map.insert("等待".to_string(), TksCommand::Wait);
    map.insert("断言".to_string(), TksCommand::Assert);
    map.insert("切换".to_string(), TksCommand::Switch);
    map.insert("滚动查找".to_string(), TksCommand::ScrollFind);
    map.insert("断言页面".to_string(), TksCommand::AssertPage);
    map.insert("按键".to_string(), TksCommand::Key);
    map.insert("确认对话框".to_string(), TksCommand::DialogAccept);
    map.insert("取消对话框".to_string(), TksCommand::DialogDismiss);
    map.insert("对话框输入".to_string(), TksCommand::DialogInput);
    map
}

/// 所有可用指令名（报错时列给人看——认不出指令时最该知道的就是"那有哪些能用"）
pub fn command_names() -> Vec<String> {
    let mut v: Vec<String> = create_command_map().into_keys().collect();
    v.sort();
    v
}

/// 创建方向映射表（中文方向 → 英文方向）
pub fn create_direction_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("上".to_string(), "up".to_string());
    map.insert("下".to_string(), "down".to_string());
    map.insert("左".to_string(), "left".to_string());
    map.insert("右".to_string(), "right".to_string());
    map
}
