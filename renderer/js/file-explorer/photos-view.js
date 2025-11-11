// Photos View Module - 相册视图模块
// 负责加载和显示相机照片、截图,以及缩略图生成

window.PhotosView = {
  currentFolder: 'camera', // 'camera' 或 'screenshots'
  currentDeviceId: null,
  mediaFiles: [], // 当前文件夹的媒体文件列表
  selectedMedia: new Set(), // 选中的媒体文件路径
  currentPreviewIndex: -1, // 当前预览的文件索引
  availableFolders: {}, // 实际可用的文件夹路径

  // 文件夹路径映射(候选路径)
  folderPathCandidates: {
    camera: [
      '/sdcard/DCIM/Camera/',
      '/sdcard/DCIM/',
      '/storage/emulated/0/DCIM/Camera/',
      '/storage/emulated/0/DCIM/'
    ],
    screenshots: [
      '/sdcard/DCIM/Screenshots/',
      '/sdcard/Pictures/Screenshots/',
      '/sdcard/Screenshots/',
      '/storage/emulated/0/Pictures/Screenshots/',
      '/storage/emulated/0/Screenshots/'
    ]
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
   * 检测可用的文件夹路径
   * @param {string} folderType - 文件夹类型 ('camera' 或 'screenshots')
   * @returns {Promise<string|null>} 第一个存在的路径,或 null
   */
  async detectAvailableFolder(folderType) {
    const candidates = this.folderPathCandidates[folderType];
    if (!candidates) return null;

    for (const path of candidates) {
      try {
        const result = await window.api.tkeFileLs({
          path: path,
          deviceId: this.currentDeviceId
        });

        if (result.success) {
          window.rLog(`✅ 找到可用路径: ${path}`);
          return path;
        }
      } catch (error) {
        // 静默处理，继续尝试下一个路径（不输出错误日志）
        continue;
      }
    }

    return null;
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

    // 检测可用的文件夹路径 (如果还没有检测过)
    if (!this.availableFolders[this.currentFolder]) {
      window.rLog(`正在检测 ${this.currentFolder} 的可用路径...`);
      const detectedPath = await this.detectAvailableFolder(this.currentFolder);

      if (!detectedPath) {
        // 未找到路径，显示空状态（不报错）
        window.rLog(`未找到 ${this.currentFolder} 的可用路径，显示空状态`);
        this.showEmptyState();
        return;
      }

      this.availableFolders[this.currentFolder] = detectedPath;
    }

    const path = this.availableFolders[this.currentFolder];
    window.rLog(`加载媒体文件: ${path}`);

    try {
      // 使用 tke file ls 获取详细文件列表(包括日期时间)
      const result = await window.api.tkeFileLs({
        path: path,
        deviceId: this.currentDeviceId
      });

      if (!result.success) {
        throw new Error(result.error || '加载失败');
      }

      // 解析文件列表
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
      const errorMsg = error?.message || '未知错误';
      this.showError('加载失败: ' + errorMsg);
    }
  },

  /**
   * 解析媒体文件列表(使用 ls -l 获取详细信息包括日期)
   * @param {string} output - ls 命令输出
   * @param {string} basePath - 基础路径
   * @returns {Array} 文件列表
   */
  parseMediaFiles(output, basePath) {
    if (!output) return [];

    const lines = output.trim().split('\n');
    const files = [];

    // ls -l 输出格式: -rw-rw---- 1 u0_a123 u0_a123 1234567 2024-01-15 10:30 IMG_20240115_103045.jpg
    for (let line of lines) {
      line = line.trim();
      if (!line || line.startsWith('total')) continue;

      const parts = line.split(/\s+/);
      if (parts.length < 8) continue;

      const permissions = parts[0];
      const isDir = permissions.startsWith('d');

      // 跳过目录和 . ..
      if (isDir) continue;

      const size = parts[4];
      const date = parts[5]; // 日期 YYYY-MM-DD
      const time = parts[6]; // 时间 HH:MM
      const name = parts.slice(7).join(' ');

      // 跳过 . 和 ..
      if (name === '.' || name === '..') continue;

      files.push({
        name: name,
        size: size,
        date: date,
        time: time,
        timestamp: this.parseTimestamp(date, time),
        isDir: false,
        path: `${basePath.replace(/\/$/, '')}/${name}`
      });
    }

    return files;
  },

  /**
   * 解析时间戳
   * @param {string} date - 日期字符串 YYYY-MM-DD
   * @param {string} time - 时间字符串 HH:MM
   * @returns {number} Unix 时间戳
   */
  parseTimestamp(date, time) {
    try {
      const dateTime = `${date} ${time}:00`;
      return new Date(dateTime).getTime();
    } catch {
      return 0;
    }
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
   * 渲染照片网格(按日期分组)
   */
  renderGrid() {
    if (this.mediaFiles.length === 0) {
      this.photosGrid.innerHTML = `
        <div class="empty-state" style="grid-column: 1 / -1;">
          <svg viewBox="0 0 24 24" width="64" height="64" fill="white">
            <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
          </svg>
          <p>No media files found</p>
        </div>
      `;
      return;
    }

    this.photosGrid.innerHTML = '';

    // 按日期分组
    const groupedByDate = this.groupFilesByDate(this.mediaFiles);

    // 渲染每个日期组
    groupedByDate.forEach(group => {
      // 创建日期分隔符
      const dateSeparator = document.createElement('div');
      dateSeparator.className = 'date-separator';
      dateSeparator.textContent = this.formatDateLabel(group.date);
      this.photosGrid.appendChild(dateSeparator);

      // 渲染该日期的所有文件
      group.files.forEach((file, indexInGroup) => {
        const globalIndex = this.mediaFiles.indexOf(file);
        const item = this.createPhotoItem(file, globalIndex);
        this.photosGrid.appendChild(item);
      });
    });
  },

  /**
   * 按日期分组文件
   * @param {Array} files - 文件列表
   * @returns {Array} 分组后的数组 [{date, files}, ...]
   */
  groupFilesByDate(files) {
    const groups = new Map();

    files.forEach(file => {
      const date = file.date || 'Unknown';
      if (!groups.has(date)) {
        groups.set(date, []);
      }
      groups.get(date).push(file);
    });

    // 转换为数组并按日期倒序排序
    const result = Array.from(groups.entries()).map(([date, files]) => ({
      date,
      files
    }));

    result.sort((a, b) => b.date.localeCompare(a.date));

    return result;
  },

  /**
   * 格式化日期标签
   * @param {string} date - 日期字符串 YYYY-MM-DD
   * @returns {string} 格式化后的日期
   */
  formatDateLabel(date) {
    if (!date || date === 'Unknown') return 'Unknown Date';

    try {
      const d = new Date(date);
      const today = new Date();
      const yesterday = new Date(today);
      yesterday.setDate(yesterday.getDate() - 1);

      // 判断是否是今天
      if (d.toDateString() === today.toDateString()) {
        return '今天';
      }

      // 判断是否是昨天
      if (d.toDateString() === yesterday.toDateString()) {
        return '昨天';
      }

      // 其他日期显示具体日期
      const year = d.getFullYear();
      const month = d.getMonth() + 1;
      const day = d.getDate();

      // 如果是今年,不显示年份
      if (year === today.getFullYear()) {
        return `${month}月${day}日`;
      }

      return `${year}年${month}月${day}日`;
    } catch {
      return date;
    }
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

    // 缩略图容器
    const thumbnail = document.createElement('div');
    thumbnail.className = 'photo-thumbnail';
    thumbnail.style.cssText = 'width: 100%; height: 100%; background: var(--bg-secondary); display: flex; align-items: center; justify-content: center; position: relative; overflow: hidden;';

    // 占位符 - 使用 media.svg 图标
    const placeholder = document.createElement('div');
    placeholder.className = 'thumbnail-placeholder';
    placeholder.innerHTML = `
      <svg viewBox="0 0 24 24" width="32" height="32" fill="var(--text-tertiary)">
        <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
      </svg>
    `;
    thumbnail.appendChild(placeholder);

    item.appendChild(thumbnail);

    // 使用 Intersection Observer 懒加载缩略图
    this.observeThumbnail(item, file, thumbnail, placeholder);

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
    item.addEventListener('click', (e) => {
      // 如果点击的是复选框，不触发预览
      if (e.target.closest('.photo-item-checkbox')) {
        return;
      }
      this.openPreview(index);
    });

    // 右键菜单
    item.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      e.stopPropagation();

      // 显示右键菜单
      if (window.ContextMenuManager) {
        window.ContextMenuManager.show(e.clientX, e.clientY, {
          paths: [file.path],
          isDir: false,
          isMedia: true,
          fileName: file.name
        });
      }
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
   * 使用 Intersection Observer 懒加载缩略图
   * @param {HTMLElement} item - 照片项元素
   * @param {Object} file - 文件对象
   * @param {HTMLElement} thumbnail - 缩略图容器
   * @param {HTMLElement} placeholder - 占位符元素
   */
  observeThumbnail(item, file, thumbnail, placeholder) {
    // 创建 Intersection Observer (如果还没有)
    if (!this.thumbnailObserver) {
      this.thumbnailObserver = new IntersectionObserver(
        (entries) => {
          entries.forEach(entry => {
            if (entry.isIntersecting) {
              const photoItem = entry.target;
              const filePath = photoItem.dataset.path;
              const fileObj = this.mediaFiles.find(f => f.path === filePath);

              if (fileObj && !photoItem.dataset.loaded) {
                photoItem.dataset.loaded = 'true';
                this.loadThumbnail(fileObj, photoItem);
                this.thumbnailObserver.unobserve(photoItem);
              }
            }
          });
        },
        {
          root: this.photosGrid,
          rootMargin: '200px', // 提前加载视口外200px的图片
          threshold: 0.01
        }
      );
    }

    this.thumbnailObserver.observe(item);
  },

  /**
   * 加载缩略图 (使用 MediaCache 管理缩略图缓存)
   * @param {Object} file - 文件对象
   * @param {HTMLElement} photoItem - 照片项元素
   */
  async loadThumbnail(file, photoItem) {
    try {
      const thumbnail = photoItem.querySelector('.photo-thumbnail');
      const placeholder = photoItem.querySelector('.thumbnail-placeholder');

      if (!thumbnail) return;

      // 显示加载状态 - 使用旋转的 spinner
      if (placeholder) {
        placeholder.innerHTML = `
          <div class="photo-loading-spinner"></div>
        `;
      }

      const isVideo = this.isVideo(file.name);
      const fileType = isVideo ? 'video' : 'image';

      // 使用 MediaCache 获取缩略图 URL（只缓存缩略图）
      const thumbPath = await window.MediaCache.getThumbnailUrl(
        file.path,
        this.currentDeviceId,
        fileType
      );

      // 移除占位符
      if (placeholder) {
        placeholder.remove();
      }

      // 显示缩略图
      const img = document.createElement('img');
      img.src = thumbPath;
      img.style.cssText = 'width: 100%; height: 100%; object-fit: cover;';
      img.alt = file.name;
      thumbnail.appendChild(img);

    } catch (error) {
      window.rError(`加载缩略图失败 ${file.name}:`, error);
      const placeholder = photoItem.querySelector('.thumbnail-placeholder');
      if (placeholder) {
        // 显示错误状态的 media.svg 图标
        placeholder.innerHTML = `
          <svg viewBox="0 0 24 24" width="32" height="32" fill="var(--accent-danger)">
            <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
          </svg>
        `;
      }
    }
  },

  /**
   * 格式化文件大小
   * @param {number} bytes - 字节数
   * @returns {string} 格式化后的大小
   */
  formatFileSize(bytes) {
    if (bytes >= 1024 * 1024 * 1024) {
      return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
    } else if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
    } else if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(1)}KB`;
    }
    return `${bytes}B`;
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
   * 打开预览（加载完整图片/视频）
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
      content.innerHTML = `
        <div style="display: flex; align-items: center; justify-content: center; height: 100%;">
          <div class="photo-loading-spinner"></div>
        </div>
      `;
    }

    if (modal) {
      modal.style.display = 'flex';
    }

    try {
      const isVideo = this.isVideo(file.name);

      window.rLog(`正在加载完整媒体文件: ${file.path}`);

      // 使用 MediaCache 获取完整媒体文件（不缓存）
      const localPath = await window.MediaCache.getFullMediaUrl(
        file.path,
        this.currentDeviceId
      );

      window.rLog(`完整媒体文件已加载: ${localPath}`);

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
    const content = document.getElementById('previewContent');

    // 停止视频播放
    if (content) {
      const video = content.querySelector('video');
      if (video) {
        video.pause();
        video.currentTime = 0;
        video.src = ''; // 清空视频源
      }
      // 清空内容
      content.innerHTML = '';
    }

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
   * 显示空状态
   */
  showEmptyState() {
    this.photosGrid.innerHTML = `
      <div class="empty-state" style="grid-column: 1 / -1;">
        <svg viewBox="0 0 24 24" width="64" height="64" fill="white">
          <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
        </svg>
        <p>No media files found</p>
      </div>
    `;
  },

  /**
   * 显示错误信息
   * @param {string} message - 错误信息
   */
  showError(message) {
    this.photosGrid.innerHTML = `
      <div class="empty-state" style="grid-column: 1 / -1;">
        <svg viewBox="0 0 24 24" width="64" height="64" fill="white">
          <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
        </svg>
        <p style="color: var(--text-secondary);">${message}</p>
      </div>
    `;
  }
};
