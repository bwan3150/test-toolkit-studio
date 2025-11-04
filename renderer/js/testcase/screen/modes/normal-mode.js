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
    const deviceScreenshot = document.getElementById('deviceScreenshot');

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

    // 隐藏静态截图
    if (deviceScreenshot) {
      deviceScreenshot.style.display = 'none';
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

    // 显示视频流 Canvas（如果视频流已激活）
    this._showVideoStreamCanvas();

    // 如果视频流未激活，尝试启动（会自动显示 loading）
    if (!window.VideoStreamStateManager || !window.VideoStreamStateManager.isVideoStreamActive()) {
      await this._tryActivateVideoStream();
    }
  },

  /**
   * 显示视频流 Canvas
   */
  _showVideoStreamCanvas() {
    if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.videoCanvas) {
      const canvas = window.ScrcpyVideoStream.videoCanvas;
      const wrapper = window.ScrcpyVideoStream.videoWrapper;
      const screenContent = document.getElementById('screenContent');

      // 添加 has-video-stream 类，隐藏静态截图
      if (screenContent) {
        screenContent.classList.add('has-video-stream');
      }

      // 显示 Canvas 和容器
      if (wrapper) {
        wrapper.style.display = 'flex';
      }
      canvas.style.display = 'block';
      canvas.style.opacity = '1';

      // 强制重新计算布局，修复视频模糊问题
      // 触发重绘以确保 Canvas 正常渲染
      canvas.style.transform = 'translateZ(0)';

      // 请求下一帧重绘（确保渲染管线正常工作）
      requestAnimationFrame(() => {
        if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.decoder) {
          // 解码器会持续更新 Canvas，无需手动干预
          window.rLog('✅ 视频流 Canvas 已显示并触发重绘');
        }
      });

      window.rLog('✅ 视频流 Canvas 已显示');
    }
  },

  /**
   * 隐藏视频流 Canvas
   */
  _hideVideoStreamCanvas() {
    if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.videoCanvas) {
      const canvas = window.ScrcpyVideoStream.videoCanvas;
      const wrapper = window.ScrcpyVideoStream.videoWrapper;
      const screenContent = document.getElementById('screenContent');

      // 隐藏 Canvas 和容器
      canvas.style.display = 'none';
      canvas.style.opacity = '0';
      if (wrapper) {
        wrapper.style.display = 'none';
      }

      // 移除 has-video-stream 类，允许显示静态截图
      if (screenContent) {
        screenContent.classList.remove('has-video-stream');
      }

      window.rLog('✅ 视频流 Canvas 已隐藏');
    }
  },

  /**
   * 尝试激活视频流（通过统一管理器）
   */
  async _tryActivateVideoStream() {
    try {
      // 检查设备是否已连接
      const deviceSelect = document.getElementById('deviceSelect');
      if (!deviceSelect || !deviceSelect.value) {
        window.rLog('💡 未选择设备，显示连接设备提示');

        // 显示连接设备提示（只在 normal 模式下）
        if (window.ScreenPrompt && window.ScreenPrompt.showConnectDevicePrompt) {
          window.ScreenPrompt.showConnectDevicePrompt();
        }

        return;
      }

      const deviceId = deviceSelect.value;

      // 通过统一管理器启动视频流
      if (window.VideoStreamStateManager) {
        const success = await window.VideoStreamStateManager.startVideoStream(deviceId);

        if (!success) {
          // 显示获取屏幕提示（只在 normal 模式下）
          if (window.ScreenPrompt && window.ScreenPrompt.showCaptureScreenPrompt) {
            window.ScreenPrompt.showCaptureScreenPrompt();
          }
        }
      } else {
        window.rError('❌ VideoStreamStateManager 模块未加载');
      }

    } catch (error) {
      window.rError('❌ 激活视频流时发生错误:', error);
    }
  },

  /**
   * 停用普通模式（不停止视频流，只隐藏 Canvas）
   */
  async deactivate() {
    window.rLog('📱 停用普通屏幕模式（隐藏视频流，但不停止）');

    // 只隐藏视频流 Canvas，不停止视频流
    this._hideVideoStreamCanvas();

    // 移除提示
    if (window.ScreenPrompt && window.ScreenPrompt.removePrompt) {
      window.ScreenPrompt.removePrompt();
    }
  }
};

// 导出模块
window.NormalMode = NormalMode;
