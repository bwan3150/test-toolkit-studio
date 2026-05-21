// 控制消息类 - 从 ws-scrcpy ControlMessage.ts 转换
// 用于向设备发送控制指令

use byteorder::{BigEndian, WriteBytesExt};

use super::video_settings::VideoSettings;

// 控制消息类型常量 - 严格对应 ControlMessage.ts 的定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlMessageType {
    Keycode = 0,                    // TYPE_KEYCODE
    Text = 1,                       // TYPE_TEXT
    Touch = 2,                      // TYPE_TOUCH
    Scroll = 3,                     // TYPE_SCROLL
    BackOrScreenOn = 4,             // TYPE_BACK_OR_SCREEN_ON
    ExpandNotificationPanel = 5,    // TYPE_EXPAND_NOTIFICATION_PANEL
    ExpandSettingsPanel = 6,        // TYPE_EXPAND_SETTINGS_PANEL
    CollapsePanels = 7,             // TYPE_COLLAPSE_PANELS
    GetClipboard = 8,               // TYPE_GET_CLIPBOARD
    SetClipboard = 9,               // TYPE_SET_CLIPBOARD
    SetScreenPowerMode = 10,        // TYPE_SET_SCREEN_POWER_MODE
    RotateDevice = 11,              // TYPE_ROTATE_DEVICE
    ChangeStreamParameters = 101,   // TYPE_CHANGE_STREAM_PARAMETERS
    PushFile = 102,                 // TYPE_PUSH_FILE
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

    /**
     * 创建按键码控制消息
     * 对应 KeyCodeControlMessage.ts 的 toBuffer() 方法
     * @param action - 动作 (按下/抬起)
     * @param keycode - 按键码
     * @param repeat - 重复次数
     * @param meta_state - Meta 状态
     * @returns 按键控制消息的二进制数据
     */
    pub fn create_keycode_event(
        action: i8,
        keycode: i32,
        repeat: i32,
        meta_state: i32,
    ) -> Vec<u8> {
        // KeyCodeControlMessage 格式 (14 字节):
        // - 1 byte: type (0)
        // - 1 byte: action
        // - 4 bytes: keycode (int, Big Endian)
        // - 4 bytes: repeat (int, Big Endian)
        // - 4 bytes: metaState (int, Big Endian)
        const PAYLOAD_LENGTH: usize = 13;
        let mut buffer = Vec::with_capacity(PAYLOAD_LENGTH + 1);

        // Type - 对应 TS: offset = buffer.writeInt8(this.type, offset);
        buffer.push(ControlMessageType::Keycode as u8);

        // Action - 对应 TS: offset = buffer.writeInt8(this.action, offset);
        buffer.push(action as u8);

        // Keycode - 对应 TS: offset = buffer.writeInt32BE(this.keycode, offset);
        buffer.write_i32::<BigEndian>(keycode).unwrap();

        // Repeat - 对应 TS: offset = buffer.writeInt32BE(this.repeat, offset);
        buffer.write_i32::<BigEndian>(repeat).unwrap();

        // MetaState - 对应 TS: buffer.writeInt32BE(this.metaState, offset);
        buffer.write_i32::<BigEndian>(meta_state).unwrap();

        buffer
    }

    /**
     * 创建文本输入控制消息
     * 对应 TextControlMessage.ts 的 toBuffer() 方法
     * @param text - 要输入的文本
     * @returns 文本控制消息的二进制数据
     */
    pub fn create_text_event(text: &str) -> Vec<u8> {
        // TextControlMessage 格式:
        // - 1 byte: type (1)
        // - 4 bytes: length (int, Big Endian)
        // - N bytes: text (UTF-8 string)

        // 对应 TS: const length = this.text.length;
        let length = text.len();

        // 对应 TS: const TEXT_SIZE_FIELD_LENGTH = 4;
        const TEXT_SIZE_FIELD_LENGTH: usize = 4;

        // 对应 TS: const buffer = Buffer.alloc(length + 1 + TextControlMessage.TEXT_SIZE_FIELD_LENGTH);
        let mut buffer = Vec::with_capacity(length + 1 + TEXT_SIZE_FIELD_LENGTH);

        // Type - 对应 TS: offset = buffer.writeUInt8(this.type, offset);
        buffer.push(ControlMessageType::Text as u8);

        // Length - 对应 TS: offset = buffer.writeUInt32BE(length, offset);
        buffer.write_u32::<BigEndian>(length as u32).unwrap();

        // Text - 对应 TS: buffer.write(this.text, offset);
        buffer.extend_from_slice(text.as_bytes());

        buffer
    }

    /**
     * 创建设置剪切板命令
     * 对应 CommandControlMessage.ts 的 createSetClipboardCommand() 方法
     * @param text - 剪切板文本内容
     * @param paste - 是否粘贴 (默认 false)
     * @returns 设置剪切板控制消息的二进制数据
     */
    pub fn create_set_clipboard_command(text: &str, paste: bool) -> Vec<u8> {
        // 对应 TS: const textBytes: Uint8Array | null = text ? Util.stringToUtf8ByteArray(text) : null;
        let text_bytes: Option<&[u8]> = if !text.is_empty() {
            Some(text.as_bytes())
        } else {
            None
        };

        // 对应 TS: const textLength = textBytes ? textBytes.length : 0;
        let text_length = text_bytes.map_or(0, |b| b.len());

        // 对应 TS: const buffer = Buffer.alloc(1 + 1 + 4 + textLength);
        let mut buffer = Vec::with_capacity(1 + 1 + 4 + text_length);

        // Type - 对应 TS: offset = buffer.writeInt8(event.type, offset);
        buffer.push(ControlMessageType::SetClipboard as u8);

        // Paste flag - 对应 TS: offset = buffer.writeInt8(paste ? 1 : 0, offset);
        buffer.push(if paste { 1 } else { 0 });

        // Text length - 对应 TS: offset = buffer.writeInt32BE(textLength, offset);
        buffer.write_i32::<BigEndian>(text_length as i32).unwrap();

        // Text bytes - 对应 TS: if (textBytes) { textBytes.forEach(...) }
        if let Some(bytes) = text_bytes {
            buffer.extend_from_slice(bytes);
        }

        buffer
    }

    /**
     * 创建获取剪切板命令
     * 对应 CommandControlMessage.ts 中 TYPE_GET_CLIPBOARD 的处理
     * @returns 获取剪切板控制消息的二进制数据
     */
    pub fn create_get_clipboard_command() -> Vec<u8> {
        // GetClipboard 只需要发送类型字节
        // 对应 TS 中 CommandControlMessage 的默认 toBuffer() 实现
        vec![ControlMessageType::GetClipboard as u8]
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

    #[test]
    fn test_create_keycode_event() {
        // 测试按键码事件创建
        let buffer = ControlMessage::create_keycode_event(0, 4, 0, 0);

        // 验证长度 (14 字节 = 1 type + 1 action + 4 keycode + 4 repeat + 4 metaState)
        assert_eq!(buffer.len(), 14);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::Keycode as u8);
        assert_eq!(buffer[0], 0); // TYPE_KEYCODE = 0

        // 验证 action
        assert_eq!(buffer[1], 0);

        // 验证 keycode (Big Endian, 值为 4)
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(buffer[4], 0);
        assert_eq!(buffer[5], 4);
    }

    #[test]
    fn test_create_text_event() {
        // 测试文本输入事件
        let text = "Hello";
        let buffer = ControlMessage::create_text_event(text);

        // 验证长度 (1 type + 4 length + 5 text = 10 字节)
        assert_eq!(buffer.len(), 10);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::Text as u8);
        assert_eq!(buffer[0], 1); // TYPE_TEXT = 1

        // 验证文本长度 (Big Endian, 值为 5)
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(buffer[4], 5);

        // 验证文本内容
        assert_eq!(&buffer[5..10], b"Hello");
    }

    #[test]
    fn test_create_text_event_empty() {
        // 测试空文本
        let buffer = ControlMessage::create_text_event("");

        // 验证长度 (1 type + 4 length = 5 字节)
        assert_eq!(buffer.len(), 5);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::Text as u8);

        // 验证文本长度为 0
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(buffer[4], 0);
    }

    #[test]
    fn test_create_set_clipboard_command() {
        // 测试设置剪切板命令
        let text = "clipboard text";
        let buffer = ControlMessage::create_set_clipboard_command(text, false);

        // 验证长度 (1 type + 1 paste + 4 length + 14 text = 20 字节)
        assert_eq!(buffer.len(), 20);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::SetClipboard as u8);
        assert_eq!(buffer[0], 9); // TYPE_SET_CLIPBOARD = 9

        // 验证 paste 标志为 0 (false)
        assert_eq!(buffer[1], 0);

        // 验证文本长度 (Big Endian, 值为 14)
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(buffer[4], 0);
        assert_eq!(buffer[5], 14);

        // 验证文本内容
        assert_eq!(&buffer[6..20], b"clipboard text");
    }

    #[test]
    fn test_create_set_clipboard_command_with_paste() {
        // 测试设置剪切板命令 (带粘贴)
        let buffer = ControlMessage::create_set_clipboard_command("test", true);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::SetClipboard as u8);

        // 验证 paste 标志为 1 (true)
        assert_eq!(buffer[1], 1);
    }

    #[test]
    fn test_create_set_clipboard_command_empty() {
        // 测试空文本剪切板
        let buffer = ControlMessage::create_set_clipboard_command("", false);

        // 验证长度 (1 type + 1 paste + 4 length = 6 字节)
        assert_eq!(buffer.len(), 6);

        // 验证文本长度为 0
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(buffer[4], 0);
        assert_eq!(buffer[5], 0);
    }

    #[test]
    fn test_create_get_clipboard_command() {
        // 测试获取剪切板命令
        let buffer = ControlMessage::create_get_clipboard_command();

        // 验证长度 (仅 1 字节类型)
        assert_eq!(buffer.len(), 1);

        // 验证消息类型
        assert_eq!(buffer[0], ControlMessageType::GetClipboard as u8);
        assert_eq!(buffer[0], 8); // TYPE_GET_CLIPBOARD = 8
    }
}
