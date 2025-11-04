// 视频流状态管理器
// 统一管理视频流的启动、停止、状态同步和滑块控制
// 确保所有视频流操作都通过这个管理器，避免状态不一致

const VideoStreamStateManager = {
  // 视频流状态
  isStreamActive: false,
  currentDeviceId: null,

  /**
   * 启动视频流（统一入口）
   * @param {string} deviceId - 设备 ID
   * @returns {Promise<boolean>} - 是否成功启动
   */
  async startVideoStream(deviceId) {
    if (!deviceId) {
      window.rError('❌ 启动视频流失败：未提供设备 ID');
      return false;
    }

    // 如果视频流已激活且设备相同，直接返回成功
    if (this.isStreamActive && this.currentDeviceId === deviceId) {
      window.rLog('✅ 视频流已经在运行中，设备:', deviceId);
      return true;
    }

    // 如果视频流已激活但设备不同，先停止旧的视频流
    if (this.isStreamActive && this.currentDeviceId !== deviceId) {
      window.rLog('🔄 切换设备，先停止旧视频流');
      await this.stopVideoStream();
    }

    window.rLog('🚀 启动视频流，设备:', deviceId);

    try {
      // 检查 ScrcpyVideoStream 是否可用
      if (!window.ScrcpyVideoStream) {
        throw new Error('ScrcpyVideoStream 模块未加载');
      }

      // 激活视频流
      const success = await window.ScrcpyVideoStream.activate(deviceId);

      if (success) {
        // 更新状态
        this.isStreamActive = true;
        this.currentDeviceId = deviceId;

        window.rLog('✅ 视频流已成功激活');

        // 移除"点击获取屏幕信息"提示
        if (window.ScreenPrompt && window.ScreenPrompt.removePrompt) {
          window.ScreenPrompt.removePrompt();
        }

        // 解锁滑块
        this._unlockSlider();

        // 更新视频流按钮状态
        this._updateVideoStreamButtonState('streaming');

        return true;
      } else {
        window.rError('❌ 视频流激活失败');
        return false;
      }
    } catch (error) {
      window.rError('❌ 启动视频流时发生错误:', error);
      return false;
    }
  },

  /**
   * 停止视频流（统一入口）
   * @returns {Promise<void>}
   */
  async stopVideoStream() {
    if (!this.isStreamActive) {
      window.rLog('💡 视频流未运行，无需停止');
      return;
    }

    window.rLog('🛑 停止视频流');

    try {
      // 停止视频流
      if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
        await window.ScrcpyVideoStream.deactivate();
      }

      // 更新状态
      this.isStreamActive = false;
      this.currentDeviceId = null;

      window.rLog('✅ 视频流已停止');

      // 锁定滑块并归位到 normal 模式
      this._lockSliderToNormal();

      // 更新视频流按钮状态
      this._updateVideoStreamButtonState('stopped');

    } catch (error) {
      window.rError('❌ 停止视频流时发生错误:', error);
    }
  },

  /**
   * 检查视频流是否激活
   * @returns {boolean}
   */
  isVideoStreamActive() {
    return this.isStreamActive;
  },

  /**
   * 获取当前设备 ID
   * @returns {string|null}
   */
  getCurrentDeviceId() {
    return this.currentDeviceId;
  },

  /**
   * 解锁滑块（内部方法）
   */
  _unlockSlider() {
    if (window.ModeSlider && window.ModeSlider.unlockSlider) {
      window.ModeSlider.unlockSlider();
      window.rLog('🔓 滑块已解锁');
    }
  },

  /**
   * 锁定滑块并归位到 normal 模式（内部方法）
   */
  _lockSliderToNormal() {
    if (window.ScreenCoordinator) {
      const currentMode = window.ScreenCoordinator.getCurrentMode();

      // 如果不在 normal 模式，先切换回 normal
      if (currentMode !== 'normal') {
        window.rLog('🔄 视频流已停止，切换回 normal 模式');
        window.ScreenCoordinator.switchTo('normal');
      }
    }

    // 锁定滑块
    if (window.ModeSlider && window.ModeSlider.lockSlider) {
      window.ModeSlider.lockSlider();
      window.rLog('🔒 滑块已锁定到 normal 模式');
    }
  },

  /**
   * 更新视频流按钮状态（内部方法）
   * @param {string} state - 'streaming' 或 'stopped'
   */
  _updateVideoStreamButtonState(state) {
    const toggleVideoStreamBtn = document.getElementById('toggleVideoStreamBtn');
    if (toggleVideoStreamBtn) {
      toggleVideoStreamBtn.setAttribute('data-state', state);
    }
  }
};

// 导出模块
window.VideoStreamStateManager = VideoStreamStateManager;
