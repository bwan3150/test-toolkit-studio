// 普通屏幕模式
// 纯屏幕显示，不显示任何覆盖层或交互
// 在此模式下可以显示实时视频流

const NormalMode = {
  /**
   * 激活普通模式
   */
  async activate() {
    window.rLog('📱 激活普通屏幕模式');

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

    // 尝试启动视频流（如果设备已连接且获取了屏幕信息）
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
        window.rLog('未选择设备，不启动视频流');
        return;
      }

      const deviceId = deviceSelect.value;

      // 检查是否已经获取了屏幕信息（即点击了"获取屏幕信息"按钮）
      const deviceScreenshot = document.getElementById('deviceScreenshot');
      const hasScreenData = deviceScreenshot &&
                           deviceScreenshot.complete &&
                           deviceScreenshot.naturalWidth > 0 &&
                           deviceScreenshot.style.display !== 'none';

      if (!hasScreenData) {
        window.rLog('尚未获取屏幕信息，不启动视频流');
        return;
      }

      // 检查 ScrcpyVideoStream 是否可用
      if (!window.ScrcpyVideoStream) {
        window.rError('ScrcpyVideoStream 模块未加载');
        return;
      }

      // 激活视频流
      window.rLog('尝试激活视频流，设备:', deviceId);
      const success = await window.ScrcpyVideoStream.activate(deviceId);

      if (success) {
        window.rLog('✅ 视频流已成功激活');
      } else {
        window.rError('❌ 视频流激活失败');
      }

    } catch (error) {
      window.rError('激活视频流时发生错误:', error);
    }
  },

  /**
   * 停用普通模式
   */
  async deactivate() {
    window.rLog('📱 停用普通屏幕模式');

    // 停用视频流
    if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
      await window.ScrcpyVideoStream.deactivate();
    }
  }
};

// 导出模块
window.NormalMode = NormalMode;
