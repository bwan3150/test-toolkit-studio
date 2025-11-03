// 控制消息类 - 从 ws-scrcpy ControlMessage.ts 转换
// 用于向设备发送控制指令

class ControlMessage {
  // 控制消息类型常量
  static TYPE_EXPAND_NOTIFICATION_PANEL = 0;
  static TYPE_EXPAND_SETTINGS_PANEL = 1;
  static TYPE_COLLAPSE_PANELS = 2;
  static TYPE_GET_CLIPBOARD = 3;
  static TYPE_SET_CLIPBOARD = 4;
  static TYPE_SET_SCREEN_POWER_MODE = 5;
  static TYPE_ROTATE_DEVICE = 6;
  static TYPE_CHANGE_STREAM_PARAMETERS = 101; // 修改视频流参数

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
   * 创建触摸事件控制消息 (简化版, 后续可扩展)
   */
  static createTouchEvent(action, x, y) {
    // TODO: 实现真正的触摸事件协议
    // 当前只是占位符
    window.rLog(`[ControlMessage] 触摸事件: ${action} at (${x}, ${y})`);
    return null;
  }
}

// 导出到全局
window.ScrcpyControlMessage = ControlMessage;
