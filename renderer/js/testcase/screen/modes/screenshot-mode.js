// 截图取坐标模式
// 允许用户在屏幕上框选区域并保存为图片定位器

// 获取全局变量的辅助函数
function getGlobals() {
  return window.AppGlobals;
}

const ScreenshotMode = {
  // 选择状态
  isSelecting: false,
  startX: 0,
  startY: 0,
  endX: 0,
  endY: 0,

  // 事件处理器引用（用于清理）
  _handlers: {
    mousedown: null,
    mousemove: null,
    mouseup: null,
    confirm: null,
    cancel: null
  },

  /**
   * 激活截图模式
   */
  activate() {
    window.rLog('📷 激活截图取坐标模式');

    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');
    const deviceScreenshot = document.getElementById('deviceScreenshot');

    if (!screenContent || !screenshotSelector) {
      window.rError('截图模式所需元素未找到');
      return;
    }

    // 添加截图模式类
    screenContent.classList.add('screenshot-mode');

    // 显示静态截图（如果有）
    if (deviceScreenshot && deviceScreenshot.src) {
      deviceScreenshot.style.display = 'block';
      window.rLog('✅ 静态截图已显示');
    }

    // 设置事件监听器
    this._setupEventListeners();
  },

  /**
   * 停用截图模式
   */
  deactivate() {
    window.rLog('📷 停用截图取坐标模式');

    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');

    if (screenContent) {
      screenContent.classList.remove('screenshot-mode');
    }

    if (screenshotSelector) {
      screenshotSelector.style.display = 'none';
    }

    // 移除事件监听器
    this._removeEventListeners();

    // 重置状态
    this.isSelecting = false;
  },

  /**
   * 设置事件监听器
   */
  _setupEventListeners() {
    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');
    const selectorBox = screenshotSelector.querySelector('.selector-box');
    const confirmBtn = document.getElementById('confirmScreenshotBtn');
    const cancelBtn = document.getElementById('cancelScreenshotBtn');

    let isSelecting = false;
    let startX = 0, startY = 0;

    // 鼠标按下开始选择
    this._handlers.mousedown = (e) => {
      if (!this._isInScreenshotMode()) return;

      // 检查是否点击在控制按钮上
      if (e.target.closest('.selector-controls')) return;

      // 获取相对于 screenContent 的坐标
      const rect = screenContent.getBoundingClientRect();
      const screenX = e.clientX - rect.left;
      const screenY = e.clientY - rect.top;

      // 检查是否在图片区域内
      if (!window.CoordinateConverter || !window.CoordinateConverter.isPointInImage(screenX, screenY)) return;

      isSelecting = true;
      startX = screenX;
      startY = screenY;

      screenshotSelector.style.display = 'block';
      selectorBox.style.left = startX + 'px';
      selectorBox.style.top = startY + 'px';
      selectorBox.style.width = '0px';
      selectorBox.style.height = '0px';

      // 隐藏控制按钮
      screenshotSelector.querySelector('.selector-controls').style.display = 'none';

      e.preventDefault();
    };

    // 鼠标移动更新选择框
    this._handlers.mousemove = (e) => {
      if (!isSelecting || !this._isInScreenshotMode()) return;

      const rect = screenContent.getBoundingClientRect();
      const currentX = e.clientX - rect.left;
      const currentY = e.clientY - rect.top;

      // 限制在图片区域内
      const imageInfo = window.CoordinateConverter.getImageDisplayInfo();
      if (imageInfo) {
        const clampedX = Math.max(imageInfo.left, Math.min(currentX, imageInfo.left + imageInfo.width));
        const clampedY = Math.max(imageInfo.top, Math.min(currentY, imageInfo.top + imageInfo.height));

        const left = Math.min(startX, clampedX);
        const top = Math.min(startY, clampedY);
        const width = Math.abs(clampedX - startX);
        const height = Math.abs(clampedY - startY);

        selectorBox.style.left = left + 'px';
        selectorBox.style.top = top + 'px';
        selectorBox.style.width = width + 'px';
        selectorBox.style.height = height + 'px';
      }
    };

    // 鼠标释放结束选择
    this._handlers.mouseup = (e) => {
      if (!isSelecting || !this._isInScreenshotMode()) return;

      isSelecting = false;
      const rect = screenContent.getBoundingClientRect();
      const endX = e.clientX - rect.left;
      const endY = e.clientY - rect.top;

      // 限制在图片区域内
      const imageInfo = window.CoordinateConverter.getImageDisplayInfo();
      if (imageInfo) {
        const clampedEndX = Math.max(imageInfo.left, Math.min(endX, imageInfo.left + imageInfo.width));
        const clampedEndY = Math.max(imageInfo.top, Math.min(endY, imageInfo.top + imageInfo.height));

        // 转换为图片内坐标
        const startCoords = window.CoordinateConverter.screenToImageCoords(startX, startY);
        const endCoords = window.CoordinateConverter.screenToImageCoords(clampedEndX, clampedEndY);

        this.startX = Math.min(startCoords.x, endCoords.x);
        this.startY = Math.min(startCoords.y, endCoords.y);
        this.endX = Math.max(startCoords.x, endCoords.x);
        this.endY = Math.max(startCoords.y, endCoords.y);

        // 如果选择区域太小,忽略
        if ((this.endX - this.startX) < 10 || (this.endY - this.startY) < 10) {
          screenshotSelector.style.display = 'none';
          return;
        }

        // 显示控制按钮
        const controls = screenshotSelector.querySelector('.selector-controls');
        controls.style.display = 'flex';
        controls.style.left = Math.max(startX, clampedEndX) + 'px';
        controls.style.top = Math.min(startY, clampedEndY) + 'px';
      }
    };

    // 确认按钮
    let isProcessing = false;
    this._handlers.confirm = (e) => {
      e.preventDefault();
      e.stopPropagation();

      if (isProcessing) {
        window.rLog('正在处理中,跳过重复点击');
        return;
      }

      isProcessing = true;
      this._captureSelectedArea().finally(() => {
        isProcessing = false;
      });
    };

    // 取消按钮
    this._handlers.cancel = () => {
      window.rLog('截图取消按钮被点击');
      screenshotSelector.style.display = 'none';
    };

    // 绑定事件
    screenContent.addEventListener('mousedown', this._handlers.mousedown);
    screenContent.addEventListener('mousemove', this._handlers.mousemove);
    screenContent.addEventListener('mouseup', this._handlers.mouseup);

    if (confirmBtn) {
      confirmBtn.addEventListener('click', this._handlers.confirm);
    }

    if (cancelBtn) {
      cancelBtn.addEventListener('click', this._handlers.cancel);
    }
  },

  /**
   * 移除事件监听器
   */
  _removeEventListeners() {
    const screenContent = document.getElementById('screenContent');
    const confirmBtn = document.getElementById('confirmScreenshotBtn');
    const cancelBtn = document.getElementById('cancelScreenshotBtn');

    if (screenContent && this._handlers.mousedown) {
      screenContent.removeEventListener('mousedown', this._handlers.mousedown);
      screenContent.removeEventListener('mousemove', this._handlers.mousemove);
      screenContent.removeEventListener('mouseup', this._handlers.mouseup);
    }

    if (confirmBtn && this._handlers.confirm) {
      confirmBtn.removeEventListener('click', this._handlers.confirm);
    }

    if (cancelBtn && this._handlers.cancel) {
      cancelBtn.removeEventListener('click', this._handlers.cancel);
    }
  },

  /**
   * 检查当前是否处于截图模式
   */
  _isInScreenshotMode() {
    const screenContent = document.getElementById('screenContent');
    return screenContent && screenContent.classList.contains('screenshot-mode');
  },

  /**
   * 截取选中区域
   */
  async _captureSelectedArea() {
    const { fs, path } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;

    if (!projectPath) {
      window.AppNotifications?.error('请先打开项目');
      return;
    }

    // 使用 workarea 中的截图文件
    const screenshotPath = path.join(projectPath, 'workarea', 'current_screenshot.png');

    try {
      // 检查截图文件是否存在
      const screenshotExists = await fs.access(screenshotPath).then(() => true).catch(() => false);
      if (!screenshotExists) {
        window.AppNotifications?.error('请先刷新设备屏幕截图');
        document.getElementById('screenshotSelector').style.display = 'none';
        return;
      }

      // 读取截图文件
      const imageBuffer = await fs.readFile(screenshotPath);
      const base64Full = `data:image/png;base64,${imageBuffer.toString('base64')}`;

      // 创建临时图片对象来获取原始尺寸
      const tempImg = new Image();
      tempImg.src = base64Full;

      await new Promise((resolve) => {
        tempImg.onload = resolve;
      });

      // 使用统一的坐标转换系统
      const deviceStartCoords = window.CoordinateConverter.imageToDeviceCoords(this.startX, this.startY);
      const deviceEndCoords = window.CoordinateConverter.imageToDeviceCoords(this.endX, this.endY);

      const realStartX = deviceStartCoords.x;
      const realStartY = deviceStartCoords.y;
      const realEndX = deviceEndCoords.x;
      const realEndY = deviceEndCoords.y;

      // 创建Canvas来裁切图片
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      const width = realEndX - realStartX;
      const height = realEndY - realStartY;

      canvas.width = width;
      canvas.height = height;

      // 从完整图片绘制裁切区域
      ctx.drawImage(tempImg, realStartX, realStartY, width, height, 0, 0, width, height);

      // 转换为Base64
      const base64Image = canvas.toDataURL('image/png');

      // 临时保存 base64 图片供后续使用
      this._pendingBase64Image = base64Image;

      // 使用新的元素保存模态框
      await this._showSaveModal();

      // 隐藏选择器
      document.getElementById('screenshotSelector').style.display = 'none';

    } catch (error) {
      window.rError('截取图片失败:', error);
      window.AppNotifications?.error('截取失败: ' + error.message);
      document.getElementById('screenshotSelector').style.display = 'none';
    }
  },

  /**
   * 显示元素保存模态框
   */
  async _showSaveModal() {
    const { fs, path } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;

    if (!window.ElementSaveModal) {
      window.rError('ElementSaveModal 未加载');
      return;
    }

    // 准备元素数据（临时路径，实际路径在保存时确定）
    const elementData = {
      img_path: 'locator/img/pending.png' // 临时占位
    };

    try {
      const result = await window.ElementSaveModal.show({
        title: '保存截图元素',
        saveType: 'img',
        elementData: elementData,
        defaultName: `screenshot_${Date.now()}`
      });

      if (!result) {
        window.rLog('用户取消了保存');
        return;
      }

      if (result.action === 'new') {
        // 新建元素
        await this._saveAsNewElement(result.name, result.note);
      } else if (result.action === 'merge') {
        // 合并到已有元素
        await this._mergeToExistingElement(result.targetName);
      }
    } catch (error) {
      window.rError('保存元素失败:', error);
      window.AppNotifications?.error('保存失败: ' + error.message);
    }
  },

  /**
   * 保存为新元素
   */
  async _saveAsNewElement(name, note) {
    const { fs, path } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;

    if (!projectPath || !this._pendingBase64Image) {
      return;
    }

    try {
      // 确保locator/img目录存在
      const locatorDir = path.join(projectPath, 'locator');
      const imgDir = path.join(locatorDir, 'img');
      await fs.mkdir(imgDir, { recursive: true });

      // 生成图片文件名
      const imageFileName = `${name}.png`;
      const imagePath = path.join(imgDir, imageFileName);

      // 保存图片文件
      const imageBuffer = Buffer.from(this._pendingBase64Image.split(',')[1], 'base64');
      await fs.writeFile(imagePath, imageBuffer);

      // 创建图像定位器对象 - 统一格式
      const locator = {
        name: name,
        note: note || '',
        img_path: `locator/img/${imageFileName}`,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      };

      // 添加到LocatorLibraryPanel并保存到element.json
      if (window.LocatorLibraryPanel) {
        window.LocatorLibraryPanel.locators[name] = locator;
        await window.LocatorLibraryPanel.saveLocators();
        window.LocatorLibraryPanel.renderLocators();

        // 切换到Locator库标签
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) {
          locatorTab.click();
        }
      }

      window.AppNotifications?.success(`图像元素 "${name}" 已保存`);
      window.rLog(`图像元素已保存: ${imagePath}`);

      // 清理临时数据
      this._pendingBase64Image = null;

    } catch (error) {
      window.rError('保存图片元素失败:', error);
      window.AppNotifications?.error('保存失败: ' + error.message);
    }
  },

  /**
   * 合并到已有元素
   */
  async _mergeToExistingElement(targetName) {
    const { fs, path } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;

    if (!projectPath || !this._pendingBase64Image) {
      return;
    }

    const locators = window.LocatorLibraryPanel?.locators || {};
    const existingLocator = locators[targetName];

    if (!existingLocator) {
      window.AppNotifications?.error('目标元素不存在');
      return;
    }

    try {
      // 确保locator/img目录存在
      const locatorDir = path.join(projectPath, 'locator');
      const imgDir = path.join(locatorDir, 'img');
      await fs.mkdir(imgDir, { recursive: true });

      // 如果已有旧图片，先删除
      if (existingLocator.img_path) {
        const oldImgPath = path.join(projectPath, existingLocator.img_path);
        try {
          await fs.unlink(oldImgPath);
          window.rLog(`已删除旧图片: ${oldImgPath}`);
        } catch (e) {
          // 旧文件可能不存在，忽略
        }
      }

      // 生成新的图片文件名
      const imageFileName = `${targetName}.png`;
      const imagePath = path.join(imgDir, imageFileName);

      // 保存图片文件
      const imageBuffer = Buffer.from(this._pendingBase64Image.split(',')[1], 'base64');
      await fs.writeFile(imagePath, imageBuffer);

      // 更新元素的 img_path
      existingLocator.img_path = `locator/img/${imageFileName}`;
      existingLocator.updated_at = new Date().toISOString();

      // 保存到 element.json
      if (window.LocatorLibraryPanel) {
        await window.LocatorLibraryPanel.saveLocators();
        window.LocatorLibraryPanel.renderLocators();

        // 切换到Locator库标签
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) {
          locatorTab.click();
        }
      }

      window.AppNotifications?.success(`图片已合并到元素 "${targetName}"`);
      window.rLog(`图片已合并到元素: ${targetName}`);

      // 清理临时数据
      this._pendingBase64Image = null;

    } catch (error) {
      window.rError('合并图片失败:', error);
      window.AppNotifications?.error('合并失败: ' + error.message);
    }
  }
};

// 导出模块
window.ScreenshotMode = ScreenshotMode;
