// XML Overlay 模式
// 在屏幕截图上覆盖 UI 元素边界和信息

// 获取全局变量的辅助函数
function getGlobals() {
  return window.AppGlobals;
}

const XmlOverlayMode = {
  /**
   * 激活 XML Overlay 模式
   */
  async activate() {
    window.rLog('🎯 激活 XML Overlay 模式');

    const deviceSelect = document.getElementById('deviceSelect');
    if (!deviceSelect?.value) {
      window.AppNotifications?.deviceRequired();
      throw new Error('未选择设备');
    }

    try {
      const { ipcRenderer } = getGlobals();
      const projectPath = window.AppGlobals.currentProject;

      // 1. 确保有截图
      const deviceImage = document.getElementById('deviceScreenshot');
      if (!deviceImage || !deviceImage.complete || deviceImage.naturalWidth === 0) {
        window.rLog('设备截图未加载,先刷新屏幕');
        if (window.ScreenCapture && window.ScreenCapture.refreshDeviceScreen) {
          await window.ScreenCapture.refreshDeviceScreen();
        }

        // 等待图片加载完成
        await new Promise((resolve, reject) => {
          const img = document.getElementById('deviceScreenshot');
          if (img.complete) {
            resolve();
          } else {
            img.onload = resolve;
            img.onerror = () => reject(new Error('截图加载失败'));
          }
        });
      }

      // 2. 使用 TKE fetcher infer-screen-size 推断屏幕尺寸
      const sizeResult = await ipcRenderer.invoke('tke-fetcher-infer-screen-size', projectPath);

      let screenSize = { width: 1080, height: 1920 }; // 默认值
      if (sizeResult.success) {
        try {
          const sizeData = JSON.parse(sizeResult.output);
          screenSize = {
            width: sizeData.width,
            height: sizeData.height
          };
          window.rLog('推断屏幕尺寸:', screenSize);
        } catch (e) {
          window.rWarn('解析屏幕尺寸失败,使用默认值:', e);
        }
      }

      // 如果有保存的屏幕尺寸且合理,也可以使用
      if (window.AppGlobals.currentScreenSize &&
          window.AppGlobals.currentScreenSize.width > 0 &&
          window.AppGlobals.currentScreenSize.height > 0) {
        const savedSize = window.AppGlobals.currentScreenSize;
        const widthDiff = Math.abs(savedSize.width - screenSize.width);
        const heightDiff = Math.abs(savedSize.height - screenSize.height);

        if (widthDiff < 100 && heightDiff < 100) {
          screenSize = savedSize;
          window.rLog(`使用保存的屏幕尺寸: ${screenSize.width}x${screenSize.height}`);
        }
      }

      // 3. 使用 TKE fetcher extract-ui-elements 提取 UI 元素
      const extractResult = await ipcRenderer.invoke('tke-fetcher-extract-ui-elements', {
        projectPath: projectPath,
        screenWidth: screenSize.width,
        screenHeight: screenSize.height
      });

      if (!extractResult.success) {
        throw new Error('TKE提取UI元素失败: ' + (extractResult.error || '未知错误'));
      }

      // 解析 TKE 返回的 JSON
      let elements;
      try {
        elements = JSON.parse(extractResult.output);
      } catch (e) {
        throw new Error('解析TKE输出失败: ' + e.message);
      }

      if (!Array.isArray(elements)) {
        throw new Error('TKE返回的数据格式不正确');
      }

      // 存储元素和屏幕尺寸
      if (window.ScreenState) {
        window.ScreenState.currentUIElements = elements;
        window.ScreenState.currentScreenSize = screenSize;
      }

      // 4. 创建 overlay
      if (window.OverlayRenderer && window.OverlayRenderer.createUIOverlay) {
        await window.OverlayRenderer.createUIOverlay(elements, screenSize);
      }

      // 5. 显示元素列表
      if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
        window.UIExtractor.displayUIElementList(elements);
      }

      // 6. 更新状态
      if (window.ScreenState) {
        window.ScreenState.setXmlOverlayEnabled(true);
      }

      window.rLog(`✅ XML Overlay 启用成功! 元素数量 = ${elements.length}`);

    } catch (error) {
      const errorMsg = error?.message || error?.toString() || JSON.stringify(error) || '未知错误';
      window.rError('❌ 启用XML Overlay失败:', errorMsg, error);
      window.AppNotifications?.error(`启用XML Overlay失败: ${errorMsg}`);

      if (window.ScreenState) {
        window.ScreenState.setXmlOverlayEnabled(false);
      }
      throw error;
    }
  },

  /**
   * 停用 XML Overlay 模式
   */
  deactivate() {
    window.rLog('📊 停用 XML Overlay 模式');

    // 移除UI叠层
    const screenContent = document.getElementById('screenContent');
    if (screenContent) {
      const overlay = screenContent.querySelector('.ui-overlay');
      if (overlay) {
        overlay.remove();
      }
    }

    // 清空UI元素列表
    if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
      window.UIExtractor.displayUIElementList([]);
    }

    // 重置状态
    if (window.ScreenState) {
      window.ScreenState.currentUIElements = [];
      window.ScreenState.selectedElement = null;
      window.ScreenState.setXmlOverlayEnabled(false);
    }

    // 断开 ResizeObserver
    if (window.ScreenState && window.ScreenState.resizeObserver) {
      window.ScreenState.resizeObserver.disconnect();
      window.ScreenState.resizeObserver = null;
    }
  }
};

// 导出模块
window.XmlOverlayMode = XmlOverlayMode;
