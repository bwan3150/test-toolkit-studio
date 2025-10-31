// 普通屏幕模式 - 实时视频流模式
// 在此模式下显示设备的实时视频流，支持触摸操作
// 完全替代截图功能

const NormalMode = {
  /**
   * 激活普通模式（视频流模式）
   */
  async activate() {
    window.rLog('📱 激活普通屏幕模式（视频流模式）');

    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');
    const coordinateMarker = document.getElementById('coordinateMarker');

    // 移除所有模式类
    if (screenContent) {
      screenContent.classList.remove('screenshot-mode', 'coordinate-mode');
    }

    // 隐藏所有交互元素
    if (screenshotSelector) {
      screenshotSelector.style.display = 'none';
    }

    if (coordinateMarker) {
      coordinateMarker.style.display = 'none';
    }

    // 移除 XML overlay
    const existingOverlay = document.querySelector('.ui-overlay');
    if (existingOverlay) {
      existingOverlay.remove();
    }

    // 清空元素列表
    if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
      window.UIExtractor.displayUIElementList([]);
    }

    // 更新全局状态
    if (window.ScreenState) {
      window.ScreenState.setXmlOverlayEnabled(false);
    }

    // 尝试启动视频流（如果设备已连接）
    await this.tryActivateVideoStream();
  },

  /**
   * 尝试激活视频流
   */
  async tryActivateVideoStream() {
    try {
      // 检查设备是否已连接
      const deviceSelect = document.getElementById('deviceSelect');
      if (!deviceSelect || !deviceSelect.value) {
        window.rLog('💡 未选择设备，显示连接设备提示');
        // 保持显示"连接设备"提示
        return;
      }

      const deviceId = deviceSelect.value;

      // 检查 ScrcpyVideoStream 是否可用
      if (!window.ScrcpyVideoStream) {
        window.rError('❌ ScrcpyVideoStream 模块未加载');
        return;
      }

      // 激活视频流
      window.rLog('🚀 尝试激活视频流，设备:', deviceId);
      const success = await window.ScrcpyVideoStream.activate(deviceId);

      if (success) {
        window.rLog('✅ 视频流已成功激活');

        // 移除提示（如果有）
        if (window.ScreenPrompt && window.ScreenPrompt.removePrompt) {
          window.ScreenPrompt.removePrompt();
        }

        // 解锁滑块
        if (window.ModeSlider && window.ModeSlider.unlockSlider) {
          window.ModeSlider.unlockSlider();
        }
      } else {
        window.rError('❌ 视频流激活失败');
      }

    } catch (error) {
      window.rError('❌ 激活视频流时发生错误:', error);
    }
  },

  /**
   * 停用普通模式
   */
  async deactivate() {
    window.rLog('📱 停用普通屏幕模式（视频流模式）');

    // 停用视频流
    if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
      await window.ScrcpyVideoStream.deactivate();
    }
  }
};

// 导出模块
window.NormalMode = NormalMode;
