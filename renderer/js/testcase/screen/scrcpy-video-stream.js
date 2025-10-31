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

    // 设置样式
    this.streamIframe.style.width = '100%';
    this.streamIframe.style.height = '100%';
    this.streamIframe.style.border = 'none';
    this.streamIframe.style.display = 'block';
    this.streamIframe.style.position = 'absolute';
    this.streamIframe.style.top = '0';
    this.streamIframe.style.left = '0';

    // 添加加载完成事件
    this.streamIframe.onload = () => {
      window.rLog('视频流 iframe 加载完成');
    };

    // 添加错误处理
    this.streamIframe.onerror = (error) => {
      window.rError('视频流 iframe 加载失败:', error);
    };

    // 添加到容器
    this.streamContainer.appendChild(this.streamIframe);

    window.rLog('视频流 iframe 已创建');
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
