// Drag Upload Module - 拖拽上传模块
// 负责处理从本地拖拽文件到应用进行上传

window.DragUploadManager = {
  dropZone: null,
  onFilesDropped: null,
  deviceId: null,

  /**
   * 初始化拖拽上传功能
   * @param {HTMLElement} dropZone - 拖放区域元素
   * @param {Function} onFilesDropped - 文件放置后的回调 (filePaths) => {}
   */
  init(dropZone, onFilesDropped) {
    this.dropZone = dropZone;
    this.onFilesDropped = onFilesDropped;

    this.setupEventListeners();
  },

  /**
   * 设置当前设备ID
   * @param {string} deviceId - 设备ID
   */
  setDeviceId(deviceId) {
    this.deviceId = deviceId;
  },

  /**
   * 设置拖拽事件监听器
   */
  setupEventListeners() {
    // 防止默认拖拽行为
    this.dropZone.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.stopPropagation();

      // 只在有设备选择时允许拖拽
      if (this.deviceId) {
        this.dropZone.classList.add('drag-over');
      }
    });

    this.dropZone.addEventListener('dragleave', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.dropZone.classList.remove('drag-over');
    });

    this.dropZone.addEventListener('drop', async (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.dropZone.classList.remove('drag-over');

      if (!this.deviceId) {
        alert('Please select a device first');
        return;
      }

      await this.handleDrop(e);
    });
  },

  /**
   * 处理文件拖放
   * @param {DragEvent} e - 拖放事件
   */
  async handleDrop(e) {
    const files = e.dataTransfer.files;

    // 检查是否有文件
    if (files.length === 0) {
      return;
    }

    // 获取文件路径列表
    const filePaths = [];

    try {
      const { webUtils } = require('electron');

      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const filePath = webUtils.getPathForFile(file);
        if (filePath) {
          filePaths.push(filePath);
        }
      }
    } catch (error) {
      window.rError('无法获取文件路径:', error);
      alert('Failed to get file paths');
      return;
    }

    if (filePaths.length === 0) {
      return;
    }

    // 调用回调函数处理文件上传
    if (this.onFilesDropped && typeof this.onFilesDropped === 'function') {
      await this.onFilesDropped(filePaths);
    }
  }
};
