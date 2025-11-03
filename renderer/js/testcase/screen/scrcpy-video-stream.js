// ws-scrcpy 原生视频流管理器 - 模块化重构版本
// 不使用 iframe, 直接使用 Canvas + WebCodecs 进行原生渲染

// 🔍 立即执行的日志 - 确认脚本正在执行
window.rLog('🚀 [scrcpy-video-stream.js] 脚本开始执行');

const ScrcpyVideoStream = {
  // === 状态 ===
  isActive: false,
  isServerRunning: false,
  currentDeviceId: null,

  // === DOM 元素 ===
  streamContainer: null,
  videoCanvas: null,
  videoWrapper: null,

  // === 核心组件 ===
  streamReceiver: null,  // WebSocket 流接收器
  decoder: null,          // WebCodecs 解码器

  // === 配置 ===
  serverPort: 8000,
  serverUrl: 'http://localhost:8000',

  // === 尺寸调整 ===
  isResizing: false,
  resizeStartX: 0,
  resizeStartY: 0,
  resizeStartWidth: 0,
  resizeStartHeight: 0,
  currentResizeHandle: null,
  videoAspectRatio: 9 / 16, // 默认手机比例

  /**
   * 初始化
   */
  init() {
    // 获取容器
    this.streamContainer = document.getElementById('screenContent');
    if (!this.streamContainer) {
      window.rLog('⏳ screenContent 容器暂时不可用, 将在 activate() 时重试');
      // 不阻止初始化继续,在 activate() 时会重试
    } else {
      window.rLog('✅ ScrcpyVideoStream 初始化完成 (模块化原生渲染)');
    }

    // 动态加载所有模块 (异步)
    this.modulesLoadedPromise = this.loadModules();
  },

  /**
   * 动态加载所有 JS 模块
   * @returns {Promise} 所有模块加载完成的 Promise
   */
  loadModules() {
    const basePath = '../js/testcase/screen/scrcpy';
    const modules = [
      `${basePath}/protocol/VideoSettings.js`,
      `${basePath}/protocol/ControlMessage.js`,
      `${basePath}/decoder/H264Parser.js`,
      `${basePath}/decoder/WebCodecsDecoder.js`,
      `${basePath}/protocol/StreamReceiver.js`
    ];

    window.rLog(`开始加载 ${modules.length} 个模块...`);

    const loadPromises = modules.map(src => {
      return new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = src;
        script.onload = () => {
          window.rLog(`✅ 已加载: ${src}`);
          resolve(src);
        };
        script.onerror = (error) => {
          window.rError(`❌ 加载失败: ${src}`, error);
          reject(new Error(`加载模块失败: ${src}`));
        };
        document.head.appendChild(script);
      });
    });

    return Promise.all(loadPromises).then(() => {
      window.rLog(`✅ 所有 ${modules.length} 个 Scrcpy 模块已加载完成`);
    }).catch(error => {
      window.rError('❌ 模块加载失败:', error);
      throw error;
    });
  },

  /**
   * 启动 ws-scrcpy 服务器
   */
  async startServer() {
    try {
      window.rLog('启动 ws-scrcpy 服务器...');

      const ipcRenderer = window.electron?.ipcRenderer || window.AppGlobals?.ipcRenderer;
      if (!ipcRenderer) {
        throw new Error('ipcRenderer 不可用');
      }

      const result = await ipcRenderer.invoke('scrcpy:start-server', {
        port: this.serverPort
      });

      if (result.success) {
        window.rLog('✅ ws-scrcpy 服务器启动成功');
        this.isServerRunning = true;
        return true;
      } else {
        window.rError('❌ ws-scrcpy 服务器启动失败:', result.error);
        return false;
      }
    } catch (error) {
      window.rError('启动 ws-scrcpy 服务器时发生错误:', error);
      return false;
    }
  },

  /**
   * 激活视频流
   */
  async activate(deviceId) {
    if (this.isActive && this.currentDeviceId === deviceId) {
      window.rLog('视频流已激活, 设备:', deviceId);
      return true;
    }

    try {
      window.rLog('🚀 激活视频流, 设备:', deviceId);
      this.currentDeviceId = deviceId;

      // 确保容器可用
      if (!this.streamContainer) {
        this.streamContainer = document.getElementById('screenContent');
        if (!this.streamContainer) {
          throw new Error('找不到 screenContent 容器');
        }
      }

      // 等待所有模块加载完成
      if (this.modulesLoadedPromise) {
        window.rLog('⏳ 等待 Scrcpy 模块加载...');
        await this.modulesLoadedPromise;
        window.rLog('✅ Scrcpy 模块已加载');
      }

      // 确保服务器已启动
      if (!this.isServerRunning) {
        const serverStarted = await this.startServer();
        if (!serverStarted) {
          window.rError('无法启动服务器');
          return false;
        }
        // 等待服务器完全启动
        await new Promise(resolve => setTimeout(resolve, 2000));
      }

      // 隐藏截图元素
      const deviceScreenshot = document.getElementById('deviceScreenshot');
      if (deviceScreenshot) {
        deviceScreenshot.style.display = 'none';
      }

      // 创建 Canvas 视频流
      this.createVideoCanvas();

      // 连接 WebSocket 并开始接收视频流
      await this.startStreaming(deviceId);

      this.isActive = true;
      window.rLog('✅ 视频流已成功激活');
      return true;

    } catch (error) {
      window.rError('激活视频流失败:', error);
      return false;
    }
  },

  /**
   * 创建 Canvas 元素
   */
  createVideoCanvas() {
    // 移除旧的
    if (this.videoWrapper) {
      this.videoWrapper.remove();
    }

    // 创建包装容器
    this.videoWrapper = document.createElement('div');
    this.videoWrapper.id = 'scrcpyVideoWrapper';
    this.videoWrapper.className = 'scrcpy-video-wrapper';

    // 创建 Canvas
    this.videoCanvas = document.createElement('canvas');
    this.videoCanvas.id = 'scrcpyVideoCanvas';
    this.videoCanvas.className = 'scrcpy-video-canvas';

    // 不创建调整大小的控制点，让视频流自适应容器
    // 就像截图模式一样，根据容器大小自动调整

    this.videoWrapper.appendChild(this.videoCanvas);
    this.streamContainer.appendChild(this.videoWrapper);
    this.streamContainer.classList.add('has-video-stream');

    this.addGlobalStyles();
    // 不设置 resize handlers，使用自适应布局
    this.setupTouchHandlers();

    window.rLog('Canvas 已创建');
  },

  /**
   * 添加全局样式
   */
  addGlobalStyles() {
    if (document.getElementById('scrcpy-stream-styles')) return;

    const style = document.createElement('style');
    style.id = 'scrcpy-stream-styles';
    style.textContent = `
      /* 视频流模式：移除 screen-content 的 padding */
      .screen-content.has-video-stream {
        padding: 0 !important;
      }

      /* 视频包装容器：填满整个父容器 */
      .scrcpy-video-wrapper {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #000;
        position: relative;
      }

      /* Canvas：自适应容器，保持宽高比 */
      .scrcpy-video-canvas {
        width: 100% !important;
        height: 100% !important;
        object-fit: contain;
        background: #000;
        cursor: pointer;
        display: block;
      }

      /* 隐藏截图，因为现在显示视频流 */
      .screen-content.has-video-stream #deviceScreenshot {
        display: none !important;
      }
    `;
    document.head.appendChild(style);
  },

  /**
   * 开始视频流传输
   */
  async startStreaming(deviceId) {
    // 重要发现：从网页版 URL 可以看出，实际使用的是 action=proxy-adb
    // 网页版 URL: ws://localhost:8000/?action=proxy-adb&remote=tcp%3A8886&udid=127.0.0.1%3A26656
    // 这意味着 ws-scrcpy 先启动了 scrcpy-server (监听 tcp:8886)，然后代理 ADB 连接到这个端口
    // 因此我们也需要使用 action=proxy-adb&remote=tcp:8886

    const wsUrl = `${this.serverUrl.replace('http', 'ws')}/?action=proxy-adb&remote=tcp%3A8886&udid=${deviceId}`;
    window.rLog('连接 WebSocket (proxy-adb 模式):', wsUrl);

    // 创建 WebSocket 连接
    const ws = new WebSocket(wsUrl);
    ws.binaryType = 'arraybuffer';
    this.streamReceiver = ws;

    // 创建解码器
    const WebCodecsDecoder = window.ScrcpyWebCodecsDecoder;
    this.decoder = new WebCodecsDecoder(this.videoCanvas);
    this.decoder.init();

    // 等待连接建立
    await new Promise((resolve, reject) => {
      ws.onopen = () => {
        window.rLog('✅ WebSocket 已连接 (proxy-adb 模式)');
        resolve();
      };
      ws.onerror = (error) => {
        window.rError('WebSocket 错误:', error);
        reject(error);
      };
    });

    // 标记是否已发送视频设置
    let sentVideoSettings = false;

    // 处理接收到的消息 (直接连接模式的消息格式)
    ws.onmessage = (event) => {
      if (!(event.data instanceof ArrayBuffer)) {
        window.rLog('收到非二进制消息，忽略');
        return;
      }

      const data = new Uint8Array(event.data);

      // 检查是否是初始信息 (magic bytes: 'scrcpy_initial')
      const MAGIC_BYTES = new Uint8Array([115, 99, 114, 99, 112, 121, 95, 105, 110, 105, 116, 105, 97, 108]);
      if (data.length >= MAGIC_BYTES.length) {
        let isInitial = true;
        for (let i = 0; i < MAGIC_BYTES.length; i++) {
          if (data[i] !== MAGIC_BYTES[i]) {
            isInitial = false;
            break;
          }
        }

        if (isInitial) {
          window.rLog('📩 收到初始信息 (scrcpy_initial)');

          // 解析设备名称 (64字节)
          const nameBytes = data.slice(MAGIC_BYTES.length, MAGIC_BYTES.length + 64);
          const deviceName = new TextDecoder().decode(nameBytes).replace(/\0/g, '').trim();
          window.rLog('设备名称:', deviceName);

          // 解析 displaysCount (4字节, Big Endian)
          let offset = MAGIC_BYTES.length + 64;
          const view = new DataView(data.buffer, data.byteOffset);
          const displaysCount = view.getInt32(offset, false); // false = Big Endian
          window.rLog(`显示器数量: ${displaysCount}`);

          // 关键发现：根据浏览器日志，必须在收到 scrcpy_initial 后发送视频设置命令
          // 服务器收到视频设置命令后才会开始推送视频流！
          if (!sentVideoSettings) {
            sentVideoSettings = true;
            window.rLog('🚀 发送视频设置命令以启动视频流...');
            this.sendVideoSettings(ws);
          } else {
            window.rLog('⚠️  已发送过视频设置，跳过');
          }

          return;
        }
      }

      // 否则当作视频帧数据处理
      this.decoder.decode(data);
    };

    ws.onclose = (event) => {
      window.rLog('WebSocket 已断开:', event.code, event.reason);
      this.cleanup();
    };
  },

  /**
   * 发送视频设置命令
   */
  sendVideoSettings(ws) {
    const VideoSettings = window.ScrcpyVideoSettings;
    const ControlMessage = window.ScrcpyControlMessage;

    // 创建视频设置 (默认参数)
    // 关键发现：bounds 不能为 null，必须设置一个合理的值！
    // 参考浏览器版本：bounds 设置为 480x480
    const settings = new VideoSettings({
      bitrate: 524288,    // 512 kbps
      maxFps: 24,         // 24 fps
      iFrameInterval: 5,  // 5秒
      bounds: { width: 480, height: 480 },  // 限制最大分辨率 (关键!)
      sendFrameMeta: false,
      lockedVideoOrientation: -1,
      displayId: 0
    });

    // 创建控制消息
    const message = ControlMessage.createSetVideoSettingsCommand(settings);

    // 显示消息内容（十六进制）
    const hex = Array.from(message.slice(0, Math.min(64, message.length)))
      .map(b => b.toString(16).padStart(2, '0'))
      .join(' ');
    window.rLog(`📤 准备发送视频设置命令: ${message.length} 字节`);
    window.rLog(`   前${Math.min(64, message.length)}字节: ${hex}`);

    // 发送
    try {
      ws.send(message);
      window.rLog('✅ 视频设置命令已发送');
    } catch (e) {
      window.rError('❌ 发送视频设置失败:', e);
    }
  },

  /**
   * 设置尺寸调整处理器
   */
  setupResizeHandlers() {
    if (!this.videoWrapper) return;

    const handles = this.videoWrapper.querySelectorAll('.resize-handle');

    handles.forEach(handle => {
      handle.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();

        this.isResizing = true;
        this.videoWrapper.classList.add('resizing');
        this.currentResizeHandle = handle.dataset.position;
        this.resizeStartX = e.clientX;
        this.resizeStartY = e.clientY;

        const rect = this.videoWrapper.getBoundingClientRect();
        this.resizeStartWidth = rect.width;
        this.resizeStartHeight = rect.height;

        document.addEventListener('mousemove', this.handleResize);
        document.addEventListener('mouseup', this.handleResizeEnd);
      });
    });
  },

  handleResize(e) {
    const self = ScrcpyVideoStream;
    if (!self.isResizing) return;

    const deltaX = e.clientX - self.resizeStartX;
    const deltaY = e.clientY - self.resizeStartY;

    let newWidth = self.resizeStartWidth;
    let newHeight = self.resizeStartHeight;

    const handle = self.currentResizeHandle;

    // 根据拖拽的控制点计算新尺寸
    if (handle.includes('e')) {
      newWidth = self.resizeStartWidth + deltaX;
    } else if (handle.includes('w')) {
      newWidth = self.resizeStartWidth - deltaX;
    }

    if (handle.includes('s')) {
      newHeight = self.resizeStartHeight + deltaY;
    } else if (handle.includes('n')) {
      newHeight = self.resizeStartHeight - deltaY;
    }

    // 保持宽高比
    const videoSize = self.decoder ? self.decoder.getVideoSize() : null;
    if (videoSize && videoSize.width > 0 && videoSize.height > 0) {
      const aspectRatio = videoSize.height / videoSize.width; // 注意: 手机通常是竖屏
      if (handle.includes('e') || handle.includes('w')) {
        newHeight = newWidth * aspectRatio;
      } else if (handle.includes('n') || handle.includes('s')) {
        newWidth = newHeight / aspectRatio;
      }
    }

    // 限制最小尺寸
    const minWidth = 200;
    const minHeight = 300;
    if (newWidth < minWidth) newWidth = minWidth;
    if (newHeight < minHeight) newHeight = minHeight;

    // 限制最大尺寸 (不超过容器)
    const container = self.streamContainer.getBoundingClientRect();
    if (newWidth > container.width) newWidth = container.width;
    if (newHeight > container.height) newHeight = container.height;

    self.videoWrapper.style.width = newWidth + 'px';
    self.videoWrapper.style.height = newHeight + 'px';
  },

  handleResizeEnd() {
    const self = ScrcpyVideoStream;
    if (self.videoWrapper) {
      self.videoWrapper.classList.remove('resizing');
    }
    self.isResizing = false;
    self.currentResizeHandle = null;

    document.removeEventListener('mousemove', self.handleResize);
    document.removeEventListener('mouseup', self.handleResizeEnd);
  },

  /**
   * 设置触摸处理器
   */
  setupTouchHandlers() {
    if (!this.videoCanvas) return;

    this.videoCanvas.addEventListener('mousedown', (e) => {
      if (this.isResizing) return;
      this.handleTouch('down', e);
    });

    this.videoCanvas.addEventListener('mousemove', (e) => {
      if (this.isResizing) return;
      if (e.buttons === 1) { // 左键按下
        this.handleTouch('move', e);
      }
    });

    this.videoCanvas.addEventListener('mouseup', (e) => {
      if (this.isResizing) return;
      this.handleTouch('up', e);
    });
  },

  /**
   * 处理触摸事件
   */
  handleTouch(action, event) {
    if (!this.decoder) return;

    const rect = this.videoCanvas.getBoundingClientRect();
    const videoSize = this.decoder.getVideoSize();

    if (!videoSize || videoSize.width === 0 || videoSize.height === 0) {
      window.rLog('视频尺寸未知, 无法计算触摸坐标');
      return;
    }

    // Canvas 坐标
    const canvasX = event.clientX - rect.left;
    const canvasY = event.clientY - rect.top;

    // 转换为设备坐标
    const deviceX = Math.round((canvasX / rect.width) * videoSize.width);
    const deviceY = Math.round((canvasY / rect.height) * videoSize.height);

    window.rLog(`触摸 ${action}: Canvas(${canvasX.toFixed(0)}, ${canvasY.toFixed(0)}) -> 设备(${deviceX}, ${deviceY})`);

    // TODO: 发送真正的触摸控制消息
    // const ControlMessage = window.ScrcpyControlMessage;
    // const touchMsg = ControlMessage.createTouchEvent(action, deviceX, deviceY);
    // if (touchMsg && this.streamReceiver) {
    //   this.streamReceiver.sendMessage(touchMsg);
    // }
  },

  /**
   * 停用视频流
   */
  async deactivate() {
    if (!this.isActive) {
      window.rLog('视频流未激活');
      return true;
    }

    try {
      window.rLog('停用视频流');

      this.cleanup();

      this.isActive = false;
      this.currentDeviceId = null;

      window.rLog('✅ 视频流已停用');
      return true;

    } catch (error) {
      window.rError('停用视频流失败:', error);
      return false;
    }
  },

  /**
   * 清理资源
   */
  cleanup() {
    // 停止解码器
    if (this.decoder) {
      this.decoder.stop();
      this.decoder = null;
    }

    // 断开 WebSocket
    if (this.streamReceiver) {
      this.streamReceiver.disconnect();
      this.streamReceiver = null;
    }

    // 移除 DOM 元素
    if (this.videoWrapper) {
      this.videoWrapper.remove();
      this.videoWrapper = null;
    }
    this.videoCanvas = null;

    // 移除容器标记
    if (this.streamContainer) {
      this.streamContainer.classList.remove('has-video-stream');
    }

    // 恢复显示截图
    const deviceScreenshot = document.getElementById('deviceScreenshot');
    if (deviceScreenshot) {
      deviceScreenshot.style.display = 'block';
    }
  },

  /**
   * 检查视频流是否激活
   */
  isStreamActive() {
    return this.isActive;
  }
};

// 导出到全局
window.ScrcpyVideoStream = ScrcpyVideoStream;
window.rLog('✅ [scrcpy-video-stream.js] ScrcpyVideoStream 已导出到 window');

// 页面加载后自动初始化
window.rLog(`🔍 [scrcpy-video-stream.js] document.readyState = ${document.readyState}`);
if (document.readyState === 'loading') {
  window.rLog('⏳ [scrcpy-video-stream.js] 页面加载中, 等待 DOMContentLoaded');
  document.addEventListener('DOMContentLoaded', () => {
    window.rLog('✅ [scrcpy-video-stream.js] DOMContentLoaded 触发, 调用 init()');
    ScrcpyVideoStream.init();
  });
} else {
  window.rLog('✅ [scrcpy-video-stream.js] 页面已加载, 立即调用 init()');
  ScrcpyVideoStream.init();
}
