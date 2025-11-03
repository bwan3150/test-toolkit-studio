// 控制消息类 - 从 ws-scrcpy ControlMessage.ts 转换
// 用于向设备发送控制指令

class ControlMessage {
  // 控制消息类型常量
  static TYPE_TOUCH = 2;
  static TYPE_EXPAND_NOTIFICATION_PANEL = 0;
  static TYPE_EXPAND_SETTINGS_PANEL = 1;
  static TYPE_COLLAPSE_PANELS = 2;
  static TYPE_GET_CLIPBOARD = 3;
  static TYPE_SET_CLIPBOARD = 4;
  static TYPE_SET_SCREEN_POWER_MODE = 5;
  static TYPE_ROTATE_DEVICE = 6;
  static TYPE_CHANGE_STREAM_PARAMETERS = 101; // 修改视频流参数

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
}

// 导出到全局
window.ScrcpyControlMessage = ControlMessage;
