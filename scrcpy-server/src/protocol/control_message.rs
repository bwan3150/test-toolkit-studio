// 控制消息类 - 从 ws-scrcpy ControlMessage.ts 转换
// 用于向设备发送控制指令

use byteorder::{BigEndian, WriteBytesExt};

use super::video_settings::VideoSettings;

// 控制消息类型常量
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlMessageType {
    ExpandNotificationPanel = 0,
    ExpandSettingsPanel = 1,
    Touch = 2,  // 注意：原 JS 代码中 TYPE_TOUCH = 2 和 TYPE_COLLAPSE_PANELS = 2，这里使用 Touch
    GetClipboard = 3,
    SetClipboard = 4,
    SetScreenPowerMode = 5,
    RotateDevice = 6,
    ChangeStreamParameters = 101, // 修改视频流参数
}

// 触摸动作常量 (对应 Android MotionEvent)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TouchAction {
    Down = 0,  // 按下
    Up = 1,    // 抬起
    Move = 2,  // 移动
}

#[derive(Debug, Clone)]
pub struct ControlMessage {
    pub msg_type: ControlMessageType,
}

impl ControlMessage {
    pub fn new(msg_type: ControlMessageType) -> Self {
        Self { msg_type }
    }

    /**
     * 创建修改视频设置的命令
     * @param video_settings - 视频设置对象
     * @returns 控制消息的二进制数据
     */
    pub fn create_set_video_settings_command(video_settings: &VideoSettings) -> Vec<u8> {
        let settings_buffer = video_settings.to_buffer();
        let total_length = 1 + settings_buffer.len(); // 1字节类型 + 设置数据
        let mut buffer = Vec::with_capacity(total_length);

        // 第一个字节: 消息类型
        buffer.push(ControlMessageType::ChangeStreamParameters as u8);

        // 剩余字节: VideoSettings 数据
        buffer.extend_from_slice(&settings_buffer);

        buffer
    }

    /**
     * 创建触摸事件控制消息
     * @param action - 触摸动作 (ACTION_DOWN/ACTION_UP/ACTION_MOVE)
     * @param pointer_id - 触摸点 ID (通常为 0)
     * @param x - X 坐标
     * @param y - Y 坐标
     * @param screen_width - 屏幕宽度
     * @param screen_height - 屏幕高度
     * @param pressure - 压力值 (0xFFFF 表示全压力, 默认)
     * @param buttons - 按钮状态 (默认 0)
     * @returns 触摸控制消息的二进制数据
     */
    #[allow(clippy::too_many_arguments)]
    pub fn create_touch_event(
        action: TouchAction,
        pointer_id: u32,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
        pressure: u16,
        buttons: i32,
    ) -> Vec<u8> {
        // TouchControlMessage 格式 (28 字节):
        // - 1 byte: type (2)
        // - 1 byte: action
        // - 8 bytes: pointerId (long, Big Endian)
        // - 4 bytes: x (int, Big Endian)
        // - 4 bytes: y (int, Big Endian)
        // - 2 bytes: screenWidth (short, Big Endian)
        // - 2 bytes: screenHeight (short, Big Endian)
        // - 2 bytes: pressure (short, Big Endian)
        // - 4 bytes: buttons (int, Big Endian)
        let mut buffer = Vec::with_capacity(28);

        // Type
        buffer.push(ControlMessageType::Touch as u8);

        // Action
        buffer.push(action as u8);

        // PointerId (8 字节 long, Big Endian)
        // JavaScript 只支持 53 位整数, 所以高 32 位为 0
        buffer.write_u32::<BigEndian>(0).unwrap(); // 高 32 位
        buffer.write_u32::<BigEndian>(pointer_id).unwrap(); // 低 32 位

        // X 坐标 (4 字节 int, Big Endian)
        buffer.write_i32::<BigEndian>(x).unwrap();

        // Y 坐标 (4 字节 int, Big Endian)
        buffer.write_i32::<BigEndian>(y).unwrap();

        // 屏幕宽度 (2 字节 short, Big Endian)
        buffer.write_u16::<BigEndian>(screen_width).unwrap();

        // 屏幕高度 (2 字节 short, Big Endian)
        buffer.write_u16::<BigEndian>(screen_height).unwrap();

        // 压力值 (2 字节 short, Big Endian)
        buffer.write_u16::<BigEndian>(pressure).unwrap();

        // 按钮状态 (4 字节 int, Big Endian)
        buffer.write_i32::<BigEndian>(buttons).unwrap();

        buffer
    }
}

// 辅助函数：创建触摸事件，使用默认参数
impl ControlMessage {
    /// 创建触摸按下事件
    pub fn create_touch_down(
        pointer_id: u32,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
    ) -> Vec<u8> {
        Self::create_touch_event(
            TouchAction::Down,
            pointer_id,
            x,
            y,
            screen_width,
            screen_height,
            0xFFFF,
            0,
        )
    }

    /// 创建触摸抬起事件
    pub fn create_touch_up(
        pointer_id: u32,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
    ) -> Vec<u8> {
        Self::create_touch_event(
            TouchAction::Up,
            pointer_id,
            x,
            y,
            screen_width,
            screen_height,
            0xFFFF,
            0,
        )
    }

    /// 创建触摸移动事件
    pub fn create_touch_move(
        pointer_id: u32,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
    ) -> Vec<u8> {
        Self::create_touch_event(
            TouchAction::Move,
            pointer_id,
            x,
            y,
            screen_width,
            screen_height,
            0xFFFF,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::video_settings::VideoSettings;

    #[test]
    fn test_create_touch_event() {
        let buffer = ControlMessage::create_touch_event(
            TouchAction::Down,
            0,
            100,
            200,
            1080,
            1920,
            0xFFFF,
            0,
        );

        // 验证长度 (28 字节)
        assert_eq!(buffer.len(), 28);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::Touch as u8);

        // 验证动作
        assert_eq!(buffer[1], TouchAction::Down as u8);
    }

    #[test]
    fn test_create_touch_down() {
        let buffer = ControlMessage::create_touch_down(0, 100, 200, 1080, 1920);
        assert_eq!(buffer.len(), 28);
        assert_eq!(buffer[1], TouchAction::Down as u8);
    }

    #[test]
    fn test_create_touch_up() {
        let buffer = ControlMessage::create_touch_up(0, 100, 200, 1080, 1920);
        assert_eq!(buffer.len(), 28);
        assert_eq!(buffer[1], TouchAction::Up as u8);
    }

    #[test]
    fn test_create_touch_move() {
        let buffer = ControlMessage::create_touch_move(0, 100, 200, 1080, 1920);
        assert_eq!(buffer.len(), 28);
        assert_eq!(buffer[1], TouchAction::Move as u8);
    }

    #[test]
    fn test_create_set_video_settings_command() {
        let settings = VideoSettings::new()
            .with_bitrate(2097152)
            .with_max_fps(30);

        let buffer = ControlMessage::create_set_video_settings_command(&settings);

        // 验证第一个字节是消息类型
        assert_eq!(buffer[0], ControlMessageType::ChangeStreamParameters as u8);

        // 验证总长度 = 1 字节类型 + VideoSettings 长度
        assert!(buffer.len() > 1);
    }
}
