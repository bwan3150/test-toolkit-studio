// Photos View Module - 相册视图模块
// 负责加载和显示相机照片、截图,以及缩略图生成

window.PhotosView = {
  currentFolder: 'camera', // 'camera' 或 'screenshots'
  currentDeviceId: null,
  mediaFiles: [], // 当前文件夹的媒体文件列表
  selectedMedia: new Set(), // 选中的媒体文件路径
  currentPreviewIndex: -1, // 当前预览的文件索引

  // 文件夹路径映射
  folderPaths: {
    camera: '/sdcard/DCIM/Camera/',
    screenshots: '/sdcard/Pictures/Screenshots/'
  },

  // 支持的图片和视频格式
  imageExtensions: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'],
  videoExtensions: ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', '3gp'],

  /**
   * 初始化 Photos 视图
   * @param {Object} elements - DOM 元素对象
   */
  init(elements) {
    this.photosGrid = elements.photosGrid;
    this.selectAllBtn = elements.selectAllBtn;
    this.downloadSelectedBtn = elements.downloadSelectedBtn;
    this.photosTabs = elements.photosTabs;

    // 绑定相机/截图 tab 切换
    this.photosTabs.forEach(tab => {
      tab.addEventListener('click', (e) => {
        const folder = e.currentTarget.dataset.folder;
        this.switchFolder(folder);
      });
    });

    // 绑定全选按钮
    this.selectAllBtn?.addEventListener('click', () => {
      this.selectAll();
    });

    // 绑定下载选中按钮
    this.downloadSelectedBtn?.addEventListener('click', () => {
      this.downloadSelected();
    });

    window.rLog('PhotosView 模块已初始化');
  },

  /**
   * 切换文件夹 (相机/截图)
   * @param {string} folder - 文件夹名称
   */
  async switchFolder(folder) {
    if (this.currentFolder === folder) return;

    this.currentFolder = folder;

    // 更新 tab 激活状态
    this.photosTabs.forEach(tab => {
      if (tab.dataset.folder === folder) {
        tab.classList.add('active');
      } else {
        tab.classList.remove('active');
      }
    });

    // 清空选择
    this.selectedMedia.clear();
    this.updateDownloadButton();

    // 重新加载
    await this.loadMedia();
  },

  /**
   * 加载媒体文件列表
   * @param {string} deviceId - 设备ID
   */
  async loadMedia(deviceId) {
    if (deviceId) {
      this.currentDeviceId = deviceId;
    }

    if (!this.currentDeviceId) {
      window.rError('没有选择设备');
      return;
    }

    const path = this.folderPaths[this.currentFolder];
    window.rLog(`加载媒体文件: ${path}`);

    try {
      // 使用 tkeFileLs 列出目录
      const result = await window.api.tkeFileLs({
        path: path,
        level: 1,
        deviceId: this.currentDeviceId
      });

      if (!result.success) {
        throw new Error(result.error || '加载失败');
      }

      // 解析文件列表 (假设返回格式和 FileRenderer.parseTreeOutput 类似)
      const files = this.parseMediaFiles(result.output, path);

      // 过滤出图片和视频文件
      this.mediaFiles = files.filter(file => {
        if (file.isDir) return false;
        const ext = this.getFileExtension(file.name);
        return this.imageExtensions.includes(ext) || this.videoExtensions.includes(ext);
      });

      // 按修改时间倒序排序 (最新的在前面)
      this.mediaFiles.sort((a, b) => b.name.localeCompare(a.name));

      window.rLog(`找到 ${this.mediaFiles.length} 个媒体文件`);

      // 渲染网格
      this.renderGrid();
    } catch (error) {
      window.rError('加载媒体文件失败:', error);
      this.showError('加载失败: ' + error.message);
    }
  },

  /**
   * 解析媒体文件列表
   * @param {string} output - tree 命令输出
   * @param {string} basePath - 基础路径
   * @returns {Array} 文件列表
   */
  parseMediaFiles(output, basePath) {
    if (!output) return [];

    const lines = output.trim().split('\n');
    const files = [];

    for (let i = 1; i < lines.length; i++) {
      const line = lines[i];
      if (!line || line.includes('directories,') || line.includes('files')) {
        continue;
      }

      const match = line.match(/[├└]── (.+?)(?:\s+\((.+?)\))?$/);
      if (match) {
        const name = match[1].trim();
        const size = match[2] || '';
        const isDir = !size;

        files.push({
          name: name,
          size: size,
          isDir: isDir,
          path: `${basePath.replace(/\/$/, '')}/${name}`
        });
      }
    }

    return files;
  },

  /**
   * 获取文件扩展名
   * @param {string} fileName - 文件名
   * @returns {string} 扩展名(小写)
   */
  getFileExtension(fileName) {
    const parts = fileName.split('.');
    if (parts.length > 1) {
      return parts[parts.length - 1].toLowerCase();
    }
    return '';
  },

  /**
   * 判断是否为视频文件
   * @param {string} fileName - 文件名
   * @returns {boolean}
   */
  isVideo(fileName) {
    const ext = this.getFileExtension(fileName);
    return this.videoExtensions.includes(ext);
  },

  /**
   * 渲染照片网格
   */
  renderGrid() {
    if (this.mediaFiles.length === 0) {
      this.photosGrid.innerHTML = `
        <div class="empty-state" style="grid-column: 1 / -1;">
          <svg viewBox="0 0 24 24" width="64" height="64">
            <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
          </svg>
          <p>No media files found</p>
        </div>
      `;
      return;
    }

    this.photosGrid.innerHTML = '';

    this.mediaFiles.forEach((file, index) => {
      const item = this.createPhotoItem(file, index);
      this.photosGrid.appendChild(item);
    });
  },

  /**
   * 创建照片项 DOM 元素
   * @param {Object} file - 文件对象
   * @param {number} index - 索引
   * @returns {HTMLElement}
   */
  createPhotoItem(file, index) {
    const item = document.createElement('div');
    item.className = 'photo-item';
    item.dataset.path = file.path;
    item.dataset.index = index;

    const isVideo = this.isVideo(file.name);

    // 复选框
    const checkbox = document.createElement('div');
    checkbox.className = 'photo-item-checkbox';
    checkbox.innerHTML = `
      <svg viewBox="0 0 24 24">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
      </svg>
    `;
    checkbox.addEventListener('click', (e) => {
      e.stopPropagation();
      this.toggleSelection(file.path);
    });
    item.appendChild(checkbox);

    // 缩略图 (暂时使用占位符)
    // TODO: 实际实现需要从设备拉取缩略图
    const thumbnail = document.createElement('div');
    thumbnail.style.cssText = 'width: 100%; height: 100%; background: var(--bg-secondary); display: flex; align-items: center; justify-content: center; color: var(--text-tertiary); font-size: 12px;';
    thumbnail.textContent = isVideo ? '视频' : '图片';
    item.appendChild(thumbnail);

    // 视频指示器
    if (isVideo) {
      const videoIndicator = document.createElement('div');
      videoIndicator.className = 'video-indicator';
      videoIndicator.innerHTML = `
        <svg viewBox="0 0 24 24">
          <path d="M8 5v14l11-7z"/>
        </svg>
        <span>视频</span>
      `;
      item.appendChild(videoIndicator);
    }

    // 点击预览
    item.addEventListener('click', () => {
      this.openPreview(index);
    });

    // 拖拽功能 - 拖出到本地文件系统
    // 注意: 在 Electron 中,拖拽文件到外部需要先将文件拉取到本地
    item.draggable = true;

    item.addEventListener('dragstart', async (e) => {
      e.stopPropagation();
      window.rLog(`开始拖拽: ${file.name}`);

      try {
        // 先拉取文件到临时目录
        const { ipcRenderer } = window.AppGlobals;
        const tempDir = await ipcRenderer.invoke('get-temp-dir');
        const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
        const localPath = path.join(tempDir, file.name);

        // 检查文件是否已存在
        const fs = window.nodeRequire ? window.nodeRequire('fs') : require('fs');
        if (!fs.existsSync(localPath)) {
          window.rLog(`拉取文件到临时目录: ${localPath}`);

          const pullResult = await window.api.tkeFilePull({
            remote: file.path,
            local: tempDir,
            deviceId: this.currentDeviceId
          });

          if (!pullResult.success) {
            throw new Error(pullResult.error || '拉取文件失败');
          }
        }

        // 使用 IPC 启动拖拽
        await ipcRenderer.invoke('start-drag', {
          filePath: localPath,
          iconPath: localPath // 可以使用文件本身作为图标
        });

        window.rLog(`拖拽已启动: ${localPath}`);
      } catch (error) {
        window.rError('拖拽失败:', error);
        window.AppNotifications?.error('拖拽失败: ' + error.message);
      }
    });

    return item;
  },

  /**
   * 切换选择状态
   * @param {string} path - 文件路径
   */
  toggleSelection(path) {
    const item = this.photosGrid.querySelector(`[data-path="${path}"]`);

    if (this.selectedMedia.has(path)) {
      this.selectedMedia.delete(path);
      item?.classList.remove('selected');
    } else {
      this.selectedMedia.add(path);
      item?.classList.add('selected');
    }

    this.updateDownloadButton();
  },

  /**
   * 全选
   */
  selectAll() {
    const allSelected = this.selectedMedia.size === this.mediaFiles.length;

    if (allSelected) {
      // 取消全选
      this.selectedMedia.clear();
      this.photosGrid.querySelectorAll('.photo-item').forEach(item => {
        item.classList.remove('selected');
      });
    } else {
      // 全选
      this.mediaFiles.forEach(file => {
        this.selectedMedia.add(file.path);
      });
      this.photosGrid.querySelectorAll('.photo-item').forEach(item => {
        item.classList.add('selected');
      });
    }

    this.updateDownloadButton();
  },

  /**
   * 更新下载按钮状态
   */
  updateDownloadButton() {
    if (this.downloadSelectedBtn) {
      this.downloadSelectedBtn.disabled = this.selectedMedia.size === 0;
      this.downloadSelectedBtn.textContent = this.selectedMedia.size > 0
        ? `下载选中 (${this.selectedMedia.size})`
        : '下载选中';
    }

    if (this.selectAllBtn) {
      const allSelected = this.selectedMedia.size === this.mediaFiles.length && this.mediaFiles.length > 0;
      this.selectAllBtn.textContent = allSelected ? '取消全选' : '全选';
    }
  },

  /**
   * 下载选中的媒体文件
   */
  async downloadSelected() {
    const paths = Array.from(this.selectedMedia);
    if (paths.length === 0) return;

    window.rLog(`准备下载 ${paths.length} 个媒体文件`);

    // 使用 FileOperations 的批量下载功能
    const result = await window.FileOperations.pullMultipleFiles(paths, this.currentDeviceId);

    if (result.success) {
      // 清空选择
      this.selectedMedia.clear();
      this.photosGrid.querySelectorAll('.photo-item').forEach(item => {
        item.classList.remove('selected');
      });
      this.updateDownloadButton();
    }
  },

  /**
   * 打开预览
   * @param {number} index - 文件索引
   */
  async openPreview(index) {
    this.currentPreviewIndex = index;
    const file = this.mediaFiles[index];

    window.rLog(`打开预览: ${file.name}`);

    // 显示预览模态框
    const modal = document.getElementById('mediaPreviewModal');
    const filename = document.getElementById('previewFilename');
    const content = document.getElementById('previewContent');

    if (filename) {
      filename.textContent = file.name;
    }

    if (content) {
      content.innerHTML = '<p style="color: white;">加载中...</p>';
    }

    if (modal) {
      modal.style.display = 'flex';
    }

    try {
      // 创建临时目录来存放预览文件
      const { ipcRenderer } = window.AppGlobals;
      const tempDir = await ipcRenderer.invoke('get-temp-dir');
      const isVideo = this.isVideo(file.name);

      window.rLog(`正在从设备拉取文件: ${file.path}`);

      // 从设备拉取文件到临时目录
      const pullResult = await window.api.tkeFilePull({
        remote: file.path,
        local: tempDir,
        deviceId: this.currentDeviceId
      });

      if (!pullResult.success) {
        throw new Error(pullResult.error || '拉取文件失败');
      }

      // 构建本地文件路径
      const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
      const localPath = path.join(tempDir, file.name);

      window.rLog(`文件已拉取到: ${localPath}`);

      // 显示预览
      if (content) {
        if (isVideo) {
          content.innerHTML = `
            <video controls autoplay style="max-width: 100%; max-height: 100%; object-fit: contain;">
              <source src="${localPath}" type="video/mp4">
              您的浏览器不支持视频播放
            </video>
          `;
        } else {
          content.innerHTML = `
            <img src="${localPath}" style="max-width: 100%; max-height: 100%; object-fit: contain;" alt="${file.name}">
          `;
        }
      }
    } catch (error) {
      window.rError('预览失败:', error);
      if (content) {
        content.innerHTML = `<p style="color: var(--accent-danger);">预览失败: ${error.message}</p>`;
      }
    }
  },

  /**
   * 关闭预览
   */
  closePreview() {
    const modal = document.getElementById('mediaPreviewModal');
    if (modal) {
      modal.style.display = 'none';
    }
    this.currentPreviewIndex = -1;
  },

  /**
   * 上一个媒体
   */
  prevMedia() {
    if (this.currentPreviewIndex > 0) {
      this.openPreview(this.currentPreviewIndex - 1);
    }
  },

  /**
   * 下一个媒体
   */
  nextMedia() {
    if (this.currentPreviewIndex < this.mediaFiles.length - 1) {
      this.openPreview(this.currentPreviewIndex + 1);
    }
  },

  /**
   * 下载当前预览的媒体
   */
  async downloadCurrentMedia() {
    if (this.currentPreviewIndex < 0) return;

    const file = this.mediaFiles[this.currentPreviewIndex];
    await window.FileOperations.pullSingleFile(file.path, file.name, this.currentDeviceId);
  },

  /**
   * 显示错误信息
   * @param {string} message - 错误信息
   */
  showError(message) {
    this.photosGrid.innerHTML = `
      <div class="empty-state" style="grid-column: 1 / -1;">
        <p style="color: var(--accent-danger);">${message}</p>
      </div>
    `;
  }
};
