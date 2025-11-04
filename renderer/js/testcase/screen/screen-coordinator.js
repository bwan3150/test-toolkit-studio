// 屏幕模式协调器
// 统一管理4种屏幕交互模式的切换和生命周期
//
// 4种模式:
// - normal: 普通屏幕显示
// - xml: XML Overlay 覆盖层
// - screenshot: 截图取坐标
// - coordinate: 坐标点取值

const ScreenCoordinator = {
  currentMode: 'normal',
  initialized: false,

  // 模式映射
  modes: {
    'normal': 'NormalMode',
    'xml': 'XmlOverlayMode',
    'screenshot': 'ScreenshotMode',
    'coordinate': 'CoordinateMode'
  },

  /**
   * 初始化协调器
   */
  init() {
    if (this.initialized) {
      window.rLog('ScreenCoordinator 已经初始化过了,跳过重复初始化');
      return;
    }

    // 检查所有必需的模块是否已加载
    const requiredModules = [
      'NormalMode',
      'XmlOverlayMode',
      'ScreenshotMode',
      'CoordinateMode',
      'ModeSlider',
      'ScreenPrompt'
    ];

    for (const moduleName of requiredModules) {
      if (!window[moduleName]) {
        window.rError(`${moduleName} 未加载,无法初始化 ScreenCoordinator`);
        return;
      }
    }

    // 检查必要的 DOM 元素
    const screenContent = document.getElementById('screenContent');
    if (!screenContent) {
      window.rError('screenContent 元素未找到,延迟初始化');
      setTimeout(() => this.init(), 500);
      return;
    }

    // 设置模式滑块,传入切换模式的回调
    window.ModeSlider.setupModeButtons((mode) => {
      this.switchTo(mode);
    });

    // 检查设备状态并显示相应提示
    this.checkDeviceStatusAndPrompt();

    // 监听设备选择变化
    const deviceSelect = document.getElementById('deviceSelect');
    if (deviceSelect) {
      deviceSelect.addEventListener('change', () => {
        this.checkDeviceStatusAndPrompt();
      });
    }

    this.initialized = true;
    window.rLog('✅ ScreenCoordinator 初始化完成');
  },

  /**
   * 检查设备状态并显示相应提示（只在 normal 模式下显示提示）
   */
  async checkDeviceStatusAndPrompt() {
    const deviceSelect = document.getElementById('deviceSelect');

    // 只在 normal 模式下检查和显示提示
    if (this.currentMode !== 'normal') {
      // 非 normal 模式，移除所有提示
      window.ScreenPrompt.removePrompt();
      return;
    }

    // 情况1: 没有选择设备
    if (!deviceSelect?.value) {
      window.rLog('📱 未选择设备，显示连接设备提示');
      window.ModeSlider.lockSlider();
      window.ScreenPrompt.showConnectDevicePrompt();
      return;
    }

    // 情况2: 已选择设备，检查视频流状态
    if (window.VideoStreamStateManager && window.VideoStreamStateManager.isVideoStreamActive()) {
      // 视频流已激活，解锁滑块，移除提示
      window.rLog('✅ 视频流已激活，解锁滑块');
      window.ScreenPrompt.removePrompt();
      window.ModeSlider.unlockSlider();
    } else {
      // 视频流未激活，显示获取屏幕提示，锁定滑块
      window.rLog('📷 设备已连接但视频流未激活，显示获取屏幕提示');
      window.ModeSlider.lockSlider();
      window.ScreenPrompt.showCaptureScreenPrompt();
    }
  },

  /**
   * 切换到指定模式
   * @param {string} modeName - 模式名称 ('normal', 'xml', 'screenshot', 'coordinate')
   */
  async switchTo(modeName) {
    if (!this.initialized) {
      window.rError('ScreenCoordinator 未初始化');
      return;
    }

    // 验证模式名称
    if (!this.modes[modeName]) {
      window.rError(`无效的模式名称: ${modeName}`);
      return;
    }

    window.rLog(`🔄 切换模式: ${this.currentMode} → ${modeName}`);

    // 如果已经在目标模式,不需要切换
    if (this.currentMode === modeName) {
      window.rLog(`已经在 ${modeName} 模式,无需切换`);
      return;
    }

    // 1. 停用当前模式
    await this._deactivateCurrentMode();

    // 2. 更新当前模式
    const previousMode = this.currentMode;
    this.currentMode = modeName;

    // 3. 立即更新滑块UI，确保同步
    const uiMode = modeName === 'screenshot' ? 'crop' : modeName;
    window.ModeSlider.updateSliderPosition(uiMode);
    window.rLog(`🎯 滑块已更新到: ${uiMode}`);

    // 4. 准备模式切换所需的数据（截图和/或 XML）
    await this._prepareDataForMode(previousMode, modeName);

    // 5. 激活新模式
    try {
      await this._activateMode(modeName);
      window.rLog(`✅ 模式切换成功: ${previousMode} → ${modeName}`);

      // 切换成功后，检查并显示提示（只在 normal 模式下）
      await this.checkDeviceStatusAndPrompt();

    } catch (error) {
      window.rError(`❌ 激活模式 ${modeName} 失败:`, error);
      // 切换失败,回退到 normal 模式
      this.currentMode = 'normal';
      window.ModeSlider.updateSliderPosition('normal');
      await this._activateMode('normal');
    }

    // 6. 更新缩放控制的可见性
    this._updateZoomControlsVisibility();
  },

  /**
   * 准备模式切换所需的数据（截图和/或 XML）
   * @param {string} fromMode - 源模式
   * @param {string} toMode - 目标模式
   */
  async _prepareDataForMode(fromMode, toMode) {
    try {
      const needsScreenshot = (fromMode === 'normal' && (toMode === 'xml' || toMode === 'screenshot' || toMode === 'coordinate'));
      const needsXml = (toMode === 'xml');

      // 情况1: normal → xml overlay
      // 需要：捕获视频帧 + 获取 XML
      if (fromMode === 'normal' && toMode === 'xml') {
        window.rLog('📸 从 normal 切换到 xml：捕获视频帧 + 获取 XML');

        // 1. 捕获视频帧
        if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
          const result = await window.ScrcpyVideoStream.captureLatestFrame();
          window.rLog('✅ 视频帧已捕获');

          // 立即加载并显示截图
          if (result && result.success && result.path) {
            const deviceScreenshot = document.getElementById('deviceScreenshot');
            if (deviceScreenshot) {
              deviceScreenshot.src = `file://${result.path}?t=${Date.now()}`;
              deviceScreenshot.style.display = 'block';
              window.rLog('✅ 截图已加载并显示');
            }
          }
        } else {
          window.rWarn('⚠️ 视频流未激活，无法捕获视频帧');
        }

        // 2. 获取 XML
        const deviceSelect = document.getElementById('deviceSelect');
        if (deviceSelect?.value) {
          const deviceId = deviceSelect.value;
          const projectPath = window.AppGlobals?.currentProject;
          const ipcRenderer = window.electron?.ipcRenderer || window.AppGlobals?.ipcRenderer;

          if (ipcRenderer && projectPath) {
            window.rLog('📄 获取设备 UI XML...');
            const result = await ipcRenderer.invoke('tke-controller-capture-xml', deviceId, projectPath);

            if (result.success) {
              window.rLog('✅ UI XML 已获取');
            } else {
              window.rError('❌ 获取 UI XML 失败:', result.error);
            }
          } else {
            window.rWarn('⚠️ ipcRenderer 或项目路径不可用，跳过获取 XML');
          }
        }
      }

      // 情况2: normal → screenshot/coordinate
      // 需要：只捕获视频帧，不获取 XML
      else if (fromMode === 'normal' && (toMode === 'screenshot' || toMode === 'coordinate')) {
        window.rLog('📸 从 normal 切换到 screenshot/coordinate：仅捕获视频帧');

        if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
          const result = await window.ScrcpyVideoStream.captureLatestFrame();
          window.rLog('✅ 视频帧已捕获');

          // 立即加载并显示截图
          if (result && result.success && result.path) {
            const deviceScreenshot = document.getElementById('deviceScreenshot');
            if (deviceScreenshot) {
              deviceScreenshot.src = `file://${result.path}?t=${Date.now()}`;
              deviceScreenshot.style.display = 'block';
              window.rLog('✅ 截图已加载并显示');
            }
          }
        } else {
          window.rWarn('⚠️ 视频流未激活，无法捕获视频帧');
        }
      }

      // 情况3: screenshot/coordinate → xml overlay
      // 需要：获取 XML（截图已存在）
      else if ((fromMode === 'screenshot' || fromMode === 'coordinate') && toMode === 'xml') {
        window.rLog('📄 从 screenshot/coordinate 切换到 xml：仅获取 XML');

        const deviceSelect = document.getElementById('deviceSelect');
        if (deviceSelect?.value) {
          const deviceId = deviceSelect.value;
          const projectPath = window.AppGlobals?.currentProject;
          const ipcRenderer = window.electron?.ipcRenderer || window.AppGlobals?.ipcRenderer;

          if (ipcRenderer && projectPath) {
            window.rLog('📄 获取设备 UI XML...');
            const result = await ipcRenderer.invoke('tke-controller-capture-xml', deviceId, projectPath);

            if (result.success) {
              window.rLog('✅ UI XML 已获取');
            } else {
              window.rError('❌ 获取 UI XML 失败:', result.error);
            }
          } else {
            window.rWarn('⚠️ ipcRenderer 或项目路径不可用，跳过获取 XML');
          }
        }
      }

    } catch (error) {
      window.rError('❌ 准备模式数据失败:', error);
    }
  },

  /**
   * 停用当前模式
   */
  async _deactivateCurrentMode() {
    const moduleClass = this.modes[this.currentMode];
    const module = window[moduleClass];

    if (module && typeof module.deactivate === 'function') {
      try {
        await module.deactivate();
      } catch (error) {
        window.rError(`停用模式 ${this.currentMode} 失败:`, error);
      }
    }
  },

  /**
   * 激活指定模式
   */
  async _activateMode(modeName) {
    const moduleClass = this.modes[modeName];
    const module = window[moduleClass];

    if (module && typeof module.activate === 'function') {
      await module.activate();
    } else {
      window.rError(`模式 ${modeName} 没有 activate 方法`);
    }
  },

  /**
   * 获取当前模式
   */
  getCurrentMode() {
    return this.currentMode;
  },

  /**
   * 设置测试运行状态
   * @param {boolean} isRunning - 是否正在运行测试
   */
  setTestRunning(isRunning) {
    window.ModeSlider.setTestRunning(isRunning);

    // 如果测试开始,强制切换到纯屏幕模式
    if (isRunning && this.currentMode !== 'normal') {
      this.switchTo('normal');
    }
  },

  /**
   * 更新缩放控制的可见性
   */
  _updateZoomControlsVisibility() {
    const screenZoomControls = document.getElementById('screenZoomControls');
    const deviceImage = document.getElementById('deviceScreenshot');

    if (screenZoomControls && deviceImage && deviceImage.style.display !== 'none') {
      screenZoomControls.style.display = 'flex';
    } else if (screenZoomControls) {
      screenZoomControls.style.display = 'none';
    }
  },

  /**
   * 刷新设备屏幕（调用工具模块）
   */
  async refreshDeviceScreen() {
    if (window.ScreenCapture && window.ScreenCapture.refreshDeviceScreen) {
      return await window.ScreenCapture.refreshDeviceScreen();
    }
  }
};

// 延迟初始化,确保在 testcase 页面显示后再初始化
function initializeScreenCoordinator() {
  window.rLog('开始初始化 ScreenCoordinator');

  // 检查必要的 DOM 元素是否存在
  const screenContent = document.getElementById('screenContent');

  if (screenContent) {
    window.rLog('DOM 元素已准备好,初始化 ScreenCoordinator');
    ScreenCoordinator.init();
  } else {
    window.rLog('DOM 元素未准备好,延迟初始化');
    // 延迟重试
    setTimeout(initializeScreenCoordinator, 500);
  }
}

// 导出模块
window.ScreenCoordinator = ScreenCoordinator;
window.initializeScreenCoordinator = initializeScreenCoordinator;
