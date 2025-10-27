// 坐标点取值模式
// 点击设备屏幕获取坐标并复制到剪贴板

const CoordinateMode = {
  // 事件处理器引用（用于清理）
  _clickHandler: null,

  /**
   * 激活坐标模式
   */
  activate() {
    window.rLog('📍 激活坐标点取值模式');

    const screenContent = document.getElementById('screenContent');

    if (!screenContent) {
      window.rError('坐标模式所需元素未找到');
      return;
    }

    // 添加坐标模式类
    screenContent.classList.add('coordinate-mode');

    // 设置点击事件
    this._setupClickHandler();
  },

  /**
   * 停用坐标模式
   */
  deactivate() {
    window.rLog('📍 停用坐标点取值模式');

    const screenContent = document.getElementById('screenContent');
    const coordinateMarker = document.getElementById('coordinateMarker');

    if (screenContent) {
      screenContent.classList.remove('coordinate-mode');
    }

    if (coordinateMarker) {
      coordinateMarker.style.display = 'none';
    }

    // 移除点击事件
    this._removeClickHandler();
  },

  /**
   * 设置点击事件处理器
   */
  _setupClickHandler() {
    const screenContent = document.getElementById('screenContent');

    this._clickHandler = async (e) => {
      if (!this._isInCoordinateMode()) return;

      // 获取相对于 screenContent 的坐标
      const rect = screenContent.getBoundingClientRect();
      const screenX = e.clientX - rect.left;
      const screenY = e.clientY - rect.top;

      // 检查是否在图片区域内
      if (!window.CoordinateConverter || !window.CoordinateConverter.isPointInImage(screenX, screenY)) return;

      // 转换为图片内坐标,然后转换为设备坐标
      const imageCoords = window.CoordinateConverter.screenToImageCoords(screenX, screenY);
      const deviceCoords = window.CoordinateConverter.imageToDeviceCoords(imageCoords.x, imageCoords.y);

      // 显示坐标标记
      const coordinateMarker = document.getElementById('coordinateMarker');
      const coordinateLabel = coordinateMarker?.querySelector('.coordinate-label');

      if (coordinateMarker) {
        coordinateMarker.style.display = 'block';
        coordinateMarker.style.left = screenX + 'px';
        coordinateMarker.style.top = screenY + 'px';
      }

      // 更新坐标标签(新TKS语法格式)
      const coordText = `{${deviceCoords.x},${deviceCoords.y}}`;
      if (coordinateLabel) {
        coordinateLabel.textContent = coordText;
      }

      // 复制到剪贴板
      try {
        await navigator.clipboard.writeText(coordText);
        window.AppNotifications?.success(`坐标 ${coordText} 已复制到剪贴板`);
      } catch (err) {
        window.rError('Failed to copy coordinates:', err);
        window.AppNotifications?.error('复制坐标失败');
      }

      // 3秒后自动隐藏标记
      setTimeout(() => {
        if (coordinateMarker) {
          coordinateMarker.style.display = 'none';
        }
      }, 3000);
    };

    if (screenContent) {
      screenContent.addEventListener('click', this._clickHandler);
    }
  },

  /**
   * 移除点击事件处理器
   */
  _removeClickHandler() {
    const screenContent = document.getElementById('screenContent');

    if (screenContent && this._clickHandler) {
      screenContent.removeEventListener('click', this._clickHandler);
      this._clickHandler = null;
    }
  },

  /**
   * 检查当前是否处于坐标模式
   */
  _isInCoordinateMode() {
    const screenContent = document.getElementById('screenContent');
    return screenContent && screenContent.classList.contains('coordinate-mode');
  }
};

// 导出模块
window.CoordinateMode = CoordinateMode;
