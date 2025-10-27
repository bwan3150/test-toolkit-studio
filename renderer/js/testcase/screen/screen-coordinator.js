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
      'ModeSlider'
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

    this.initialized = true;
    window.rLog('✅ ScreenCoordinator 初始化完成');
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

    // 3. 更新滑块UI
    const uiMode = modeName === 'screenshot' ? 'crop' : modeName;
    window.ModeSlider.updateSliderPosition(uiMode);

    // 4. 激活新模式
    try {
      await this._activateMode(modeName);
      window.rLog(`✅ 模式切换成功: ${previousMode} → ${modeName}`);
    } catch (error) {
      window.rError(`❌ 激活模式 ${modeName} 失败:`, error);
      // 切换失败,回退到 normal 模式
      this.currentMode = 'normal';
      window.ModeSlider.updateSliderPosition('normal');
      await this._activateMode('normal');
    }

    // 5. 更新缩放控制的可见性
    this._updateZoomControlsVisibility();
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
