// WebSocket 流接收器 - 从 ws-scrcpy StreamReceiver.ts 简化
// 负责接收和解析 ws-scrcpy 服务器的消息

class StreamReceiver {
  // 魔术字节: 'scrcpy_initial'
  static MAGIC_BYTES_INITIAL = new Uint8Array([
    115, 99, 114, 99, 112, 121, 95, 105, 110, 105, 116, 105, 97, 108
  ]);

  static DEVICE_NAME_FIELD_LENGTH = 64;

  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.ws = null;
    this.isConnected = false;
    this.sentVideoSettings = false;

    // 回调函数
    this.onVideoCallback = null;
    this.onStatsCallback = null;
    this.onDisconnectCallback = null;
  }

  /**
   * 连接 WebSocket
   */
  async connect() {
    return new Promise((resolve, reject) => {
      window.rLog('连接 WebSocket:', this.wsUrl);

      this.ws = new WebSocket(this.wsUrl);
      this.ws.binaryType = 'arraybuffer';

      this.ws.onopen = () => {
        this.isConnected = true;
        window.rLog('✅ WebSocket 已连接');
        resolve();
      };

      this.ws.onmessage = (event) => this.handleMessage(event);
      this.ws.onerror = (error) => {
        window.rError('WebSocket 错误:', error);
        reject(error);
      };
      this.ws.onclose = (event) => {
        this.isConnected = false;
        window.rLog('WebSocket 已断开:', event.code, event.reason);
        if (this.onDisconnectCallback) {
          this.onDisconnectCallback(event);
        }
      };
    });
  }

  /**
   * 处理 WebSocket 消息
   */
  handleMessage(event) {
    if (!(event.data instanceof ArrayBuffer)) {
      return;
    }

    const data = event.data;

    // 检查是否是初始信息
    if (data.byteLength > StreamReceiver.MAGIC_BYTES_INITIAL.length) {
      const magic = new Uint8Array(data, 0, StreamReceiver.MAGIC_BYTES_INITIAL.length);
      if (this.arrayEquals(magic, StreamReceiver.MAGIC_BYTES_INITIAL)) {
        this.handleInitialInfo(data);
        return;
      }
    }

    // 否则当作视频帧处理
    if (this.onVideoCallback) {
      this.onVideoCallback(new Uint8Array(data));
    }
  }

  /**
   * 处理初始信息
   */
  handleInitialInfo(data) {
    window.rLog('📩 收到初始信息');

    let offset = StreamReceiver.MAGIC_BYTES_INITIAL.length;

    // 读取设备名称 (64字节)
    const nameBytes = new Uint8Array(data, offset, StreamReceiver.DEVICE_NAME_FIELD_LENGTH);
    offset += StreamReceiver.DEVICE_NAME_FIELD_LENGTH;

    const deviceName = new TextDecoder().decode(nameBytes).replace(/\0/g, '').trim();
    window.rLog('设备名称:', deviceName);

    // 通知统计信息回调
    if (this.onStatsCallback) {
      this.onStatsCallback({ deviceName });
    }

    // 只发送一次视频设置命令
    if (!this.sentVideoSettings) {
      this.sentVideoSettings = true;
      this.sendVideoSettings();
    }
  }

  /**
   * 发送视频设置命令
   */
  sendVideoSettings() {
    const VideoSettings = window.ScrcpyVideoSettings;
    const ControlMessage = window.ScrcpyControlMessage;

    // 创建视频设置 (默认参数)
    const settings = new VideoSettings({
      bitrate: 524288,    // 512 kbps
      maxFps: 24,         // 24 fps
      iFrameInterval: 5,  // 5秒
      bounds: null,       // 不限制分辨率
      sendFrameMeta: false,
      lockedVideoOrientation: -1,
      displayId: 0
    });

    // 创建控制消息
    const message = ControlMessage.createSetVideoSettingsCommand(settings);

    // 发送
    this.sendMessage(message);
    window.rLog('📤 已发送视频设置命令');
  }

  /**
   * 发送控制消息
   */
  sendMessage(data) {
    if (!this.isConnected || !this.ws) {
      window.rError('WebSocket 未连接, 无法发送消息');
      return;
    }

    try {
      this.ws.send(data);
    } catch (e) {
      window.rError('发送消息失败:', e);
    }
  }

  /**
   * 比较两个 Uint8Array 是否相等
   */
  arrayEquals(a, b) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }

  /**
   * 设置回调函数
   */
  onVideo(callback) {
    this.onVideoCallback = callback;
  }

  onStats(callback) {
    this.onStatsCallback = callback;
  }

  onDisconnect(callback) {
    this.onDisconnectCallback = callback;
  }

  /**
   * 断开连接
   */
  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.isConnected = false;
    this.sentVideoSettings = false;
  }
}

// 导出到全局
window.ScrcpyStreamReceiver = StreamReceiver;
