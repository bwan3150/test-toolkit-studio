// ws-scrcpy 视频流管理器
// 负责在 normal 模式下显示实时视频流并支持触摸交互

const ScrcpyVideoStream = {
  // 状态
  isActive: false,
  isServerRunning: false,
  currentDeviceId: null,

  // DOM 元素
  streamContainer: null,
  streamIframe: null,

  // 配置
  serverPort: 8000,
  serverUrl: 'http://localhost:8000',

  /**
   * 初始化视频流管理器
   */
  init() {
    window.rLog('初始化 ScrcpyVideoStream');

    // 获取 screen content 容器
    this.streamContainer = document.getElementById('screenContent');

    if (!this.streamContainer) {
      window.rError('找不到 screenContent 容器');
      return;
    }

    window.rLog('✅ ScrcpyVideoStream 初始化完成');
  },

  /**
   * 启动 ws-scrcpy 服务器
   * @returns {Promise<boolean>}
   */
  async startServer() {
    try {
      window.rLog('启动 ws-scrcpy 服务器...');

      // 检查 IPC 是否可用
      const ipcRenderer = window.electron?.ipcRenderer || window.AppGlobals?.ipcRenderer;
      if (!ipcRenderer) {
        throw new Error('ipcRenderer 不可用');
      }

      window.rLog('调用 IPC: scrcpy:start-server, 端口:', this.serverPort);

      // 调用主进程启动服务器
      const result = await ipcRenderer.invoke('scrcpy:start-server', {
        port: this.serverPort
      });

      window.rLog('IPC 返回结果:', result);

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
   * 停止 ws-scrcpy 服务器
   * @returns {Promise<boolean>}
   */
  async stopServer() {
    try {
      window.rLog('停止 ws-scrcpy 服务器...');

      const ipcRenderer = window.electron?.ipcRenderer || window.AppGlobals?.ipcRenderer;
      if (!ipcRenderer) {
        throw new Error('ipcRenderer 不可用');
      }

      const result = await ipcRenderer.invoke('scrcpy:stop-server');

      if (result.success) {
        window.rLog('✅ ws-scrcpy 服务器已停止');
        this.isServerRunning = false;
        return true;
      } else {
        window.rError('❌ ws-scrcpy 服务器停止失败:', result.error);
        return false;
      }
    } catch (error) {
      window.rError('停止 ws-scrcpy 服务器时发生错误:', error);
      return false;
    }
  },

  /**
   * 激活视频流显示
   * @param {string} deviceId - 设备 ID
   * @returns {Promise<boolean>}
   */
  async activate(deviceId) {
    if (this.isActive && this.currentDeviceId === deviceId) {
      window.rLog('视频流已经激活，设备:', deviceId);
      return true;
    }

    try {
      window.rLog('激活视频流，设备:', deviceId);
      this.currentDeviceId = deviceId;

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

      // 创建并显示 iframe 来嵌入 ws-scrcpy 页面
      this.createStreamIframe(deviceId);

      this.isActive = true;
      window.rLog('✅ 视频流已激活');
      return true;

    } catch (error) {
      window.rError('激活视频流失败:', error);
      return false;
    }
  },

  /**
   * 创建视频流 iframe
   * @param {string} deviceId - 设备 ID
   */
  createStreamIframe(deviceId) {
    // 移除旧的 iframe
    if (this.streamIframe) {
      this.streamIframe.remove();
      this.streamIframe = null;
    }

    // 构建 ws-scrcpy URL 参数
    const params = new URLSearchParams();
    params.set('action', 'stream');
    params.set('udid', deviceId);
    params.set('player', 'webcodecs');  // 使用 WebCodecs 播放器
    params.set('ws', `ws://localhost:${this.serverPort}/?action=proxy-adb&remote=tcp:8886&udid=${deviceId}`);

    // 创建 iframe
    this.streamIframe = document.createElement('iframe');
    this.streamIframe.id = 'scrcpyStreamFrame';
    this.streamIframe.src = `${this.serverUrl}/#!${params.toString()}`;

    // 设置样式 - 填充整个容器
    this.streamIframe.style.width = '100%';
    this.streamIframe.style.height = '100%';
    this.streamIframe.style.border = 'none';
    this.streamIframe.style.display = 'block';
    this.streamIframe.style.backgroundColor = '#000';
    this.streamIframe.style.flex = '1';  // 使用 flex 布局填充父容器

    // 添加加载完成事件
    this.streamIframe.onload = () => {
      window.rLog('视频流 iframe 加载完成');

      // 尝试注入样式到 iframe 内部（跨域可能失败）
      try {
        this.injectIframeStyles();
      } catch (e) {
        window.rLog('无法注入 iframe 样式（跨域限制）:', e.message);
      }
    };

    // 添加错误处理
    this.streamIframe.onerror = (error) => {
      window.rError('视频流 iframe 加载失败:', error);
    };

    // 添加到容器
    this.streamContainer.appendChild(this.streamIframe);

    // 给容器添加 class 标记，表示正在显示视频流
    this.streamContainer.classList.add('has-video-stream');

    // 添加全局样式来隐藏控制按钮
    this.addGlobalStyles();

    window.rLog('视频流 iframe 已创建');
  },

  /**
   * 添加全局样式来优化视频流显示
   */
  addGlobalStyles() {
    // 检查是否已经添加过样式
    if (document.getElementById('scrcpy-stream-styles')) {
      return;
    }

    const style = document.createElement('style');
    style.id = 'scrcpy-stream-styles';
    style.textContent = `
      /* 优化 iframe 容器 */
      #scrcpyStreamFrame {
        background: #000 !important;
        min-width: 100% !important;
        min-height: 100% !important;
      }

      /* 确保容器是黑色背景，并移除 padding */
      #screenContent {
        background-color: #000 !important;
        padding: 0 !important;
      }

      /* 当有视频流时，隐藏截图 */
      #screenContent.has-video-stream #deviceScreenshot {
        display: none !important;
      }
    `;
    document.head.appendChild(style);
  },

  /**
   * 尝试注入样式到 iframe 内部
   * 注意：由于同源策略，这只在同域名时有效
   */
  injectIframeStyles() {
    if (!this.streamIframe) return;

    const iframeDoc = this.streamIframe.contentDocument || this.streamIframe.contentWindow.document;
    if (!iframeDoc) {
      window.rLog('无法访问 iframe document');
      return;
    }

    // 创建样式元素
    const style = iframeDoc.createElement('style');
    style.textContent = `
      /* 隐藏所有控制按钮和工具栏 */
      .control-buttons-list,
      .action-button,
      .stream-controls,
      .toolbox,
      .more-box,
      [class*="control"],
      [class*="button"] {
        display: none !important;
      }

      /* 确保视频填充整个区域 */
      body {
        margin: 0 !important;
        padding: 0 !important;
        overflow: hidden !important;
        background: #000 !important;
      }

      /* 视频容器样式 */
      .screen {
        width: 100% !important;
        height: 100% !important;
        display: flex !important;
        justify-content: center !important;
        align-items: center !important;
        background: #000 !important;
      }

      /* canvas 或 video 元素 */
      canvas, video {
        max-width: 100% !important;
        max-height: 100% !important;
        object-fit: contain !important;
        background: #000 !important;
      }
    `;

    iframeDoc.head.appendChild(style);
    window.rLog('✅ iframe 样式注入成功');
  },

  /**
   * 停用视频流显示
   * @returns {Promise<boolean>}
   */
  async deactivate() {
    if (!this.isActive) {
      window.rLog('视频流未激活');
      return true;
    }

    try {
      window.rLog('停用视频流');

      // 移除 iframe
      if (this.streamIframe) {
        this.streamIframe.remove();
        this.streamIframe = null;
      }

      // 移除容器的 class 标记
      if (this.streamContainer) {
        this.streamContainer.classList.remove('has-video-stream');
      }

      // 恢复显示截图元素
      const deviceScreenshot = document.getElementById('deviceScreenshot');
      if (deviceScreenshot) {
        deviceScreenshot.style.display = 'block';
      }

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
   * 检查视频流是否激活
   * @returns {boolean}
   */
  isStreamActive() {
    return this.isActive;
  },

  /**
   * 清理资源
   */
  async cleanup() {
    await this.deactivate();

    // 停止服务器（可选，根据需求决定）
    // await this.stopServer();
  }
};

// 导出到 window
window.ScrcpyVideoStream = ScrcpyVideoStream;

// 页面加载后自动初始化
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    ScrcpyVideoStream.init();
  });
} else {
  ScrcpyVideoStream.init();
}
