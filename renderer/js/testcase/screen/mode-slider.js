// 模式滑块控制器模块
// 负责四种模式切换的UI交互(滑块动画和按钮状态)

const ModeSlider = {
  switchModeCallback: null, // 保存回调函数引用
  isTestRunning: false, // 测试运行状态标志

  /**
   * 设置模式切换滑块
   * @param {Function} switchModeCallback - 切换模式的回调函数
   */
  setupModeButtons(switchModeCallback) {
    // 保存回调函数引用，供后续使用
    this.switchModeCallback = switchModeCallback;

    const modeOptions = document.querySelectorAll('.mode-option');

    modeOptions.forEach(option => {
      option.addEventListener('click', () => {
        // 检查是否被禁用(测试运行期间)
        if (option.classList.contains('disabled')) {
          return;
        }

        const mode = option.dataset.mode;
        // 处理不同的模式名称映射
        let actualMode = mode;
        if (mode === 'crop') {
          actualMode = 'screenshot';
        }

        // 调用回调函数切换模式
        if (switchModeCallback) {
          switchModeCallback(actualMode);
        }
      });
    });

    // 初始化滑块位置
    this.updateSliderPosition('normal');
  },

  /**
   * 更新滑块位置
   * @param {string} mode - 模式名称 ('normal', 'xml', 'crop', 'coordinate')
   */
  updateSliderPosition(mode) {
    const modeSlider = document.getElementById('modeSlider');
    const modeOptions = document.querySelectorAll('.mode-option');

    if (!modeSlider) return;

    // 设置滑块的data-active属性来控制指示器位置
    modeSlider.setAttribute('data-active', mode);

    // 更新选项的激活状态
    modeOptions.forEach(option => {
      if (option.dataset.mode === mode) {
        option.classList.add('active');
      } else {
        option.classList.remove('active');
      }
    });

    // 为不同模式设置不同颜色
    const sliderIndicator = document.getElementById('sliderIndicator');
    if (sliderIndicator) {
      sliderIndicator.className = 'slider-indicator';
      sliderIndicator.classList.add(`mode-${mode}`);
    }
  },

  /**
   * 设置测试运行状态 - 禁用/启用模式切换
   * @param {boolean} isRunning - 是否正在运行测试
   */
  setTestRunning(isRunning) {
    this.isTestRunning = isRunning; // 保存测试运行状态

    const modeOptions = document.querySelectorAll('.mode-option');

    modeOptions.forEach(option => {
      if (isRunning) {
        option.classList.add('disabled');
      } else {
        option.classList.remove('disabled');
      }
    });
  },

  /**
   * 锁定滑块 - 先切换到 normal 模式，然后禁止切换到其他模式
   */
  lockSlider() {
    // 1. 先切换到 normal 模式
    if (this.switchModeCallback) {
      this.switchModeCallback('normal');
    }

    // 2. 禁用其他模式的按钮
    const modeOptions = document.querySelectorAll('.mode-option');
    modeOptions.forEach(option => {
      // 只锁定非 normal 模式
      if (option.dataset.mode !== 'normal') {
        option.classList.add('disabled');
      }
    });

    window.rLog('🔒 滑块已锁定到 normal 模式');
  },

  /**
   * 解锁滑块 - 允许所有模式切换
   */
  unlockSlider() {
    // 如果测试正在运行，不解锁滑块
    if (this.isTestRunning) {
      window.rLog('⚠️ 测试正在运行，跳过滑块解锁');
      return;
    }

    const modeOptions = document.querySelectorAll('.mode-option');

    modeOptions.forEach(option => {
      option.classList.remove('disabled');
    });

    window.rLog('🔓 滑块已解锁，允许切换模式');
  }
};

// 导出模块
window.ModeSlider = ModeSlider;
