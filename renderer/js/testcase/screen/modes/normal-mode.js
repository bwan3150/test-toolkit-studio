// 普通屏幕模式
// 纯屏幕显示，不显示任何覆盖层或交互

const NormalMode = {
  /**
   * 激活普通模式
   */
  activate() {
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
  },

  /**
   * 停用普通模式
   */
  deactivate() {
    // 普通模式没有需要清理的状态
    window.rLog('📱 停用普通屏幕模式');
  }
};

// 导出模块
window.NormalMode = NormalMode;
