// 控制消息类 - 从 ws-scrcpy ControlMessage.ts 转换
// 用于向设备发送控制指令

class ControlMessage {
  // 控制消息类型常量 - 严格对应 ControlMessage.ts 的定义
  static TYPE_KEYCODE = 0;                    // 按键码输入
  static TYPE_TEXT = 1;                       // 文本输入
  static TYPE_TOUCH = 2;                      // 触摸事件
  static TYPE_SCROLL = 3;                     // 滚动事件
  static TYPE_BACK_OR_SCREEN_ON = 4;          // 返回键或唤醒屏幕
  static TYPE_EXPAND_NOTIFICATION_PANEL = 5;  // 展开通知面板
  static TYPE_EXPAND_SETTINGS_PANEL = 6;      // 展开设置面板
  static TYPE_COLLAPSE_PANELS = 7;            // 折叠面板
  static TYPE_GET_CLIPBOARD = 8;              // 获取剪切板
  static TYPE_SET_CLIPBOARD = 9;              // 设置剪切板
  static TYPE_SET_SCREEN_POWER_MODE = 10;     // 设置屏幕电源模式
  static TYPE_ROTATE_DEVICE = 11;             // 旋转设备
  static TYPE_CHANGE_STREAM_PARAMETERS = 101; // 修改视频流参数
  static TYPE_PUSH_FILE = 102;                // 推送文件

  // 触摸动作常量 (对应 Android MotionEvent)
  static ACTION_DOWN = 0;  // 按下
  static ACTION_UP = 1;    // 抬起
  static ACTION_MOVE = 2;  // 移动

  constructor(type) {
    this.type = type;
  }

  /**
   * 创建修改视频设置的命令
   * @param {VideoSettings} videoSettings - 视频设置对象
   * @returns {Uint8Array} 控制消息的二进制数据
   */
  static createSetVideoSettingsCommand(videoSettings) {
    const settingsBuffer = videoSettings.toBuffer();
    const totalLength = 1 + settingsBuffer.length; // 1字节类型 + 设置数据
    const buffer = new Uint8Array(totalLength);

    // 第一个字节: 消息类型
    buffer[0] = ControlMessage.TYPE_CHANGE_STREAM_PARAMETERS;

    // 剩余字节: VideoSettings 数据
    buffer.set(settingsBuffer, 1);

    return buffer;
  }

  /**
   * 创建触摸事件控制消息
   * @param {number} action - 触摸动作 (ACTION_DOWN/ACTION_UP/ACTION_MOVE)
   * @param {number} pointerId - 触摸点 ID (通常为 0)
   * @param {number} x - X 坐标
   * @param {number} y - Y 坐标
   * @param {number} screenWidth - 屏幕宽度
   * @param {number} screenHeight - 屏幕高度
   * @param {number} pressure - 压力值 (0xFFFF 表示全压力, 默认)
   * @param {number} buttons - 按钮状态 (默认 0)
   * @returns {Uint8Array} 触摸控制消息的二进制数据
   */
  static createTouchEvent(action, pointerId, x, y, screenWidth, screenHeight, pressure = 0xFFFF, buttons = 0) {
    // TouchControlMessage 格式 (29 字节):
    // - 1 byte: type (2)
    // - 1 byte: action
    // - 8 bytes: pointerId (long, Big Endian)
    // - 4 bytes: x (int, Big Endian)
    // - 4 bytes: y (int, Big Endian)
    // - 2 bytes: screenWidth (short, Big Endian)
    // - 2 bytes: screenHeight (short, Big Endian)
    // - 2 bytes: pressure (short, Big Endian)
    // - 4 bytes: buttons (int, Big Endian)
    const buffer = new Uint8Array(29);
    const view = new DataView(buffer.buffer);
    let offset = 0;

    // Type
    view.setUint8(offset, ControlMessage.TYPE_TOUCH);
    offset += 1;

    // Action
    view.setUint8(offset, action);
    offset += 1;

    // PointerId (8 字节 long, Big Endian)
    // JavaScript 只支持 53 位整数, 所以高 32 位为 0
    view.setUint32(offset, 0, false); // 高 32 位
    offset += 4;
    view.setUint32(offset, pointerId, false); // 低 32 位
    offset += 4;

    // X 坐标 (4 字节 int, Big Endian)
    view.setInt32(offset, x, false);
    offset += 4;

    // Y 坐标 (4 字节 int, Big Endian)
    view.setInt32(offset, y, false);
    offset += 4;

    // 屏幕宽度 (2 字节 short, Big Endian)
    view.setUint16(offset, screenWidth, false);
    offset += 2;

    // 屏幕高度 (2 字节 short, Big Endian)
    view.setUint16(offset, screenHeight, false);
    offset += 2;

    // 压力值 (2 字节 short, Big Endian)
    view.setUint16(offset, pressure, false);
    offset += 2;

    // 按钮状态 (4 字节 int, Big Endian)
    view.setInt32(offset, buttons, false);

    return buffer;
  }

  /**
   * 创建按键码控制消息
   * 对应 KeyCodeControlMessage.ts 的 toBuffer() 方法
   * @param {number} action - 动作 (按下/抬起)
   * @param {number} keycode - 按键码
   * @param {number} repeat - 重复次数
   * @param {number} metaState - Meta 状态
   * @returns {Uint8Array} 按键控制消息的二进制数据
   */
  static createKeycodeEvent(action, keycode, repeat = 0, metaState = 0) {
    // KeyCodeControlMessage 格式 (14 字节):
    // - 1 byte: type (0)
    // - 1 byte: action
    // - 4 bytes: keycode (int, Big Endian)
    // - 4 bytes: repeat (int, Big Endian)
    // - 4 bytes: metaState (int, Big Endian)
    const buffer = new Uint8Array(14);
    const view = new DataView(buffer.buffer);
    let offset = 0;

    // Type - 对应 TS: offset = buffer.writeInt8(this.type, offset);
    view.setInt8(offset, ControlMessage.TYPE_KEYCODE);
    offset += 1;

    // Action - 对应 TS: offset = buffer.writeInt8(this.action, offset);
    view.setInt8(offset, action);
    offset += 1;

    // Keycode - 对应 TS: offset = buffer.writeInt32BE(this.keycode, offset);
    view.setInt32(offset, keycode, false); // false = Big Endian
    offset += 4;

    // Repeat - 对应 TS: offset = buffer.writeInt32BE(this.repeat, offset);
    view.setInt32(offset, repeat, false);
    offset += 4;

    // MetaState - 对应 TS: buffer.writeInt32BE(this.metaState, offset);
    view.setInt32(offset, metaState, false);

    return buffer;
  }

  /**
   * 创建文本输入控制消息
   * 对应 TextControlMessage.ts 的 toBuffer() 方法
   * @param {string} text - 要输入的文本
   * @returns {Uint8Array} 文本控制消息的二进制数据
   */
  static createTextEvent(text) {
    // TextControlMessage 格式:
    // - 1 byte: type (1)
    // - 4 bytes: length (int, Big Endian)
    // - N bytes: text (UTF-8 string)

    // 对应 TS: const length = this.text.length;
    const encoder = new TextEncoder();
    const textBytes = encoder.encode(text);
    const length = textBytes.length;

    // 对应 TS: const TEXT_SIZE_FIELD_LENGTH = 4;
    const TEXT_SIZE_FIELD_LENGTH = 4;

    // 对应 TS: const buffer = Buffer.alloc(length + 1 + TextControlMessage.TEXT_SIZE_FIELD_LENGTH);
    const buffer = new Uint8Array(1 + TEXT_SIZE_FIELD_LENGTH + length);
    const view = new DataView(buffer.buffer);
    let offset = 0;

    // Type - 对应 TS: offset = buffer.writeUInt8(this.type, offset);
    view.setUint8(offset, ControlMessage.TYPE_TEXT);
    offset += 1;

    // Length - 对应 TS: offset = buffer.writeUInt32BE(length, offset);
    view.setUint32(offset, length, false); // false = Big Endian
    offset += 4;

    // Text - 对应 TS: buffer.write(this.text, offset);
    buffer.set(textBytes, offset);

    return buffer;
  }

  /**
   * 创建设置剪切板命令
   * 对应 CommandControlMessage.ts 的 createSetClipboardCommand() 方法
   * @param {string} text - 剪切板文本内容
   * @param {boolean} paste - 是否粘贴 (默认 false)
   * @returns {Uint8Array} 设置剪切板控制消息的二进制数据
   */
  static createSetClipboardCommand(text, paste = false) {
    // 对应 TS: const textBytes: Uint8Array | null = text ? Util.stringToUtf8ByteArray(text) : null;
    const encoder = new TextEncoder();
    const textBytes = text ? encoder.encode(text) : null;

    // 对应 TS: const textLength = textBytes ? textBytes.length : 0;
    const textLength = textBytes ? textBytes.length : 0;

    // 对应 TS: const buffer = Buffer.alloc(1 + 1 + 4 + textLength);
    const buffer = new Uint8Array(1 + 1 + 4 + textLength);
    const view = new DataView(buffer.buffer);
    let offset = 0;

    // Type - 对应 TS: offset = buffer.writeInt8(event.type, offset);
    view.setInt8(offset, ControlMessage.TYPE_SET_CLIPBOARD);
    offset += 1;

    // Paste flag - 对应 TS: offset = buffer.writeInt8(paste ? 1 : 0, offset);
    view.setInt8(offset, paste ? 1 : 0);
    offset += 1;

    // Text length - 对应 TS: offset = buffer.writeInt32BE(textLength, offset);
    view.setInt32(offset, textLength, false); // false = Big Endian
    offset += 4;

    // Text bytes - 对应 TS: if (textBytes) { textBytes.forEach(...) }
    if (textBytes) {
      buffer.set(textBytes, offset);
    }

    return buffer;
  }

  /**
   * 创建获取剪切板命令
   * 对应 CommandControlMessage.ts 中 TYPE_GET_CLIPBOARD 的处理
   * @returns {Uint8Array} 获取剪切板控制消息的二进制数据
   */
  static createGetClipboardCommand() {
    // GetClipboard 只需要发送类型字节
    // 对应 TS 中 CommandControlMessage 的默认 toBuffer() 实现
    const buffer = new Uint8Array(1);
    buffer[0] = ControlMessage.TYPE_GET_CLIPBOARD;
    return buffer;
  }
}

// 导出到全局
window.ScrcpyControlMessage = ControlMessage;
