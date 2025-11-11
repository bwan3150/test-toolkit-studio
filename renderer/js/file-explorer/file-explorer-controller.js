// File Explorer Controller - 文件浏览器主控制器
// 负责协调各个模块,处理用户交互和业务逻辑流程

class FileExplorerController {
  constructor() {
    this.currentPath = '/sdcard/';
    this.currentDevice = null;
    this.selectedFiles = new Set();

    // 历史记录
    this.history = [];
    this.historyIndex = -1;

    this.initElements();
    this.initModules();
    this.initEventListeners();
    this.loadDevices();
  }

  // ============ 初始化相关 ============

  initElements() {
    // 设备选择器
    this.deviceSelect = document.getElementById('fileExplorerDeviceSelect');
    this.refreshDeviceBtn = document.getElementById('refreshFileExplorerDeviceBtn');

    // Browser Tabs
    this.browserTabs = document.querySelectorAll('.browser-tab');
    this.fileExplorerContainer = document.querySelector('.file-explorer-container');
    this.photosContainer = document.getElementById('photosContainer');
    this.currentView = 'filesystem'; // 'filesystem' 或 'photos'

    // 路径导航
    this.pathBreadcrumb = document.getElementById('pathBreadcrumb');

    // 工具栏按钮
    this.backBtn = document.getElementById('backBtn');
    this.forwardBtn = document.getElementById('forwardBtn');
    this.refreshBtn = document.getElementById('refreshBtn');
    this.newFolderBtn = document.getElementById('newFolderBtn');
    this.searchInput = document.getElementById('searchInput');
    this.searchBtn = document.getElementById('searchBtn');

    // 调试: 检查按钮是否存在
    if (!this.newFolderBtn) {
      window.rError('newFolderBtn 未找到!');
    }

    // 文件列表
    this.fileListContent = document.getElementById('fileListContent');

    // 右键菜单
    this.contextMenu = document.getElementById('fileContextMenu');

    // 目录统计
    this.directoryStats = document.getElementById('directoryStats');

    // Photos 视图元素
    this.photosGrid = document.getElementById('photosGrid');
    this.selectAllPhotosBtn = document.getElementById('selectAllPhotosBtn');
    this.downloadSelectedPhotosBtn = document.getElementById('downloadSelectedPhotosBtn');
    this.photosTabs = document.querySelectorAll('.photos-tab');

    // Media Preview 模态框
    this.mediaPreviewModal = document.getElementById('mediaPreviewModal');
    this.closePreviewBtn = document.getElementById('closePreviewBtn');
    this.prevMediaBtn = document.getElementById('prevMediaBtn');
    this.nextMediaBtn = document.getElementById('nextMediaBtn');
    this.downloadPreviewMediaBtn = document.getElementById('downloadPreviewMediaBtn');
  }

  initModules() {
    // 初始化右键菜单模块
    window.ContextMenuManager.init(this.contextMenu);

    // 注册右键菜单操作处理器
    window.ContextMenuManager.registerHandler('open', (selection) => {
      const path = Array.from(this.selectedFiles)[0];
      if (path) {
        this.openFile(path);
      }
    });

    window.ContextMenuManager.registerHandler('rename', (selection) => {
      const path = Array.from(this.selectedFiles)[0];
      if (path) {
        // 找到对应的文件项元素,启动内联重命名
        const fileItem = document.querySelector(`.file-item[data-path="${path}"]`);
        if (fileItem) {
          this.startInlineRename(fileItem, path);
        }
      }
    });

    window.ContextMenuManager.registerHandler('newfolder', async (selection) => {
      const result = await window.FileOperations.createFolder(this.currentPath, this.currentDevice);
      if (result.success) {
        // 重新加载目录
        await this.loadDirectory(this.currentPath);
        // 找到新创建的文件夹并进入重命名模式
        const newFolderPath = result.newPath;
        const fileItem = document.querySelector(`.file-item[data-path="${newFolderPath}"]`);
        if (fileItem) {
          this.startInlineRename(fileItem, newFolderPath);
        }
      }
    });

    window.ContextMenuManager.registerHandler('download', async (selection) => {
      const paths = Array.from(this.selectedFiles);
      if (paths.length > 0) {
        await window.FileOperations.pullMultipleFiles(paths, this.currentDevice);
        this.clearSelection();
      }
    });

    window.ContextMenuManager.registerHandler('delete', async (selection) => {
      const paths = Array.from(this.selectedFiles);
      if (paths.length > 0) {
        const result = await window.FileOperations.deleteFiles(paths, this.currentDevice);
        if (result.success) {
          this.clearSelection();
          this.loadDirectory(this.currentPath);
        }
      }
    });

    // 初始化拖拽上传模块
    window.DragUploadManager.init(this.fileListContent, async (filePaths) => {
      const result = await window.FileOperations.pushFilesFromPaths(
        filePaths,
        this.currentPath,
        this.currentDevice
      );
      if (result.success) {
        this.loadDirectory(this.currentPath);
      }
    });

    // 空白处右键菜单
    this.fileListContent.addEventListener('contextmenu', (e) => {
      // 如果点击的不是文件项,显示空白处菜单
      if (!e.target.closest('.file-item')) {
        e.preventDefault();
        window.ContextMenuManager.show(e.clientX, e.clientY, {
          paths: [],
          isDir: false
        });
      }
    });

    // 初始化 Photos 视图模块
    window.PhotosView.init({
      photosGrid: this.photosGrid,
      selectAllBtn: this.selectAllPhotosBtn,
      downloadSelectedBtn: this.downloadSelectedPhotosBtn,
      photosTabs: this.photosTabs
    });
  }

  initEventListeners() {
    // Browser Tabs 切换
    this.browserTabs.forEach(tab => {
      tab.addEventListener('click', (e) => {
        const view = e.currentTarget.dataset.view;
        this.switchView(view);
      });
    });

    // 设备选择
    this.deviceSelect.addEventListener('change', async () => {
      this.currentDevice = this.deviceSelect.value;
      if (this.currentDevice) {
        // 保存选择的设备
        const { ipcRenderer } = window.AppGlobals;
        await ipcRenderer.invoke('store-set', 'selected_device', this.currentDevice);

        // 更新拖拽模块的设备ID
        window.DragUploadManager.setDeviceId(this.currentDevice);

        // 根据当前视图加载内容
        if (this.currentView === 'filesystem') {
          this.loadDirectory(this.currentPath);
        } else if (this.currentView === 'photos') {
          window.PhotosView.loadMedia(this.currentDevice);
        }
      } else {
        window.FileRenderer.showEmptyState(this.fileListContent, 'Select a device to browse files');
      }
    });

    this.refreshDeviceBtn.addEventListener('click', () => this.loadDevices());

    // 工具栏按钮
    this.backBtn.addEventListener('click', () => this.navigateBack());
    this.forwardBtn.addEventListener('click', () => this.navigateForward());
    this.refreshBtn.addEventListener('click', () => this.loadDirectory(this.currentPath));
    if (this.newFolderBtn) {
      this.newFolderBtn.addEventListener('click', () => {
        window.rLog('新建文件夹按钮被点击');
        this.createFolder();
      });
    } else {
      window.rError('无法绑定新建文件夹按钮事件: 按钮不存在');
    }

    // 搜索 - 回车或点击按钮触发
    this.searchInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        this.handleSearch();
      }
    });
    this.searchBtn.addEventListener('click', () => this.handleSearch());

    // Media Preview 模态框事件
    this.closePreviewBtn?.addEventListener('click', () => {
      window.PhotosView.closePreview();
    });

    this.prevMediaBtn?.addEventListener('click', () => {
      window.PhotosView.prevMedia();
    });

    this.nextMediaBtn?.addEventListener('click', () => {
      window.PhotosView.nextMedia();
    });

    this.downloadPreviewMediaBtn?.addEventListener('click', () => {
      window.PhotosView.downloadCurrentMedia();
    });

    // 点击模态框背景关闭
    this.mediaPreviewModal?.addEventListener('click', (e) => {
      if (e.target === this.mediaPreviewModal) {
        window.PhotosView.closePreview();
      }
    });

    // ESC 键关闭预览
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.mediaPreviewModal?.style.display === 'flex') {
        window.PhotosView.closePreview();
      }
    });
  }

  // ============ 视图切换 ============

  /**
   * 切换 Browser 视图 (File System / Photos)
   * @param {string} view - 视图名称: 'filesystem' 或 'photos'
   */
  switchView(view) {
    if (this.currentView === view) return;

    window.rLog(`切换视图: ${this.currentView} -> ${view}`);
    this.currentView = view;

    // 更新 tab 激活状态
    this.browserTabs.forEach(tab => {
      if (tab.dataset.view === view) {
        tab.classList.add('active');
      } else {
        tab.classList.remove('active');
      }
    });

    // 切换容器显示
    if (view === 'filesystem') {
      this.fileExplorerContainer.style.display = 'flex';
      this.photosContainer.style.display = 'none';

      // 如果有设备,加载目录
      if (this.currentDevice) {
        this.loadDirectory(this.currentPath);
      }
    } else if (view === 'photos') {
      this.fileExplorerContainer.style.display = 'none';
      this.photosContainer.style.display = 'flex';

      // 如果有设备,加载照片
      if (this.currentDevice) {
        window.PhotosView.loadMedia(this.currentDevice);
      } else {
        window.PhotosView.showError('请先选择设备');
      }
    }
  }

  // ============ 设备管理 ============

  async loadDevices() {
    try {
      window.rLog('加载设备列表...');
      const { ipcRenderer } = window.AppGlobals;
      const result = await ipcRenderer.invoke('adb-devices');

      if (!result || !result.success) {
        throw new Error(result?.error || '获取设备列表失败');
      }

      const devices = result.devices || [];

      // 更新设备选择器
      this.deviceSelect.innerHTML = '<option value="">Select Device</option>';

      devices.forEach(device => {
        if (device.status === 'device') {
          const option = document.createElement('option');
          option.value = device.id;
          option.textContent = device.id;
          this.deviceSelect.appendChild(option);
        }
      });

      window.rLog(`找到 ${devices.length} 个设备`);

      // 恢复之前选择的设备
      const savedSelection = await ipcRenderer.invoke('store-get', 'selected_device');
      if (savedSelection && Array.from(this.deviceSelect.options).some(opt => opt.value === savedSelection)) {
        this.deviceSelect.value = savedSelection;
        this.currentDevice = savedSelection;
        window.DragUploadManager.setDeviceId(savedSelection);
        // 自动加载默认目录
        this.loadDirectory(this.currentPath);
      }
    } catch (error) {
      window.rError('加载设备列表失败:', error);
      window.FileRenderer.showError(this.fileListContent, 'Failed to load devices: ' + error.message);
    }
  }

  // ============ 目录导航 ============

  async loadDirectory(path, level = 1, addToHistory = true) {
    if (!this.currentDevice) {
      window.FileRenderer.showEmptyState(this.fileListContent, 'Select a device to browse files');
      return;
    }

    try {
      window.rLog(`加载目录: ${path}`);
      this.currentPath = path;

      // 添加到历史记录
      if (addToHistory) {
        // 如果不是在历史记录的末尾,清除后面的历史
        if (this.historyIndex < this.history.length - 1) {
          this.history = this.history.slice(0, this.historyIndex + 1);
        }
        this.history.push(path);
        this.historyIndex = this.history.length - 1;
      }

      // 更新UI - 启用path breadcrumb的点击跳转
      window.FileRenderer.updatePathBreadcrumb(this.pathBreadcrumb, path, (clickedPath) => {
        this.loadDirectory(clickedPath);
      });
      this.updateNavigationButtons();

      // 显示加载状态
      window.FileRenderer.showEmptyState(this.fileListContent, 'Loading...');

      const result = await window.api.tkeFileTree({
        path: path,
        level: level,
        deviceId: this.currentDevice
      });

      if (!result.success) {
        throw new Error(result.error || '加载目录失败');
      }

      // 解析并渲染文件列表
      const files = window.FileRenderer.parseTreeOutput(result.output, path);
      const stats = window.FileRenderer.renderFileList(
        files,
        this.fileListContent,
        (fileItem, file) => this.bindFileItemEvents(fileItem, file)
      );

      // 更新统计信息
      window.FileRenderer.updateDirectoryStats(this.directoryStats, stats.dirCount, stats.fileCount);

    } catch (error) {
      window.rError('加载目录失败:', error);
      window.FileRenderer.showError(this.fileListContent, 'Failed to load directory: ' + error.message);
    }
  }

  navigateBack() {
    if (this.historyIndex > 0) {
      this.historyIndex--;
      const path = this.history[this.historyIndex];
      this.loadDirectory(path, 1, false);
    }
  }

  navigateForward() {
    if (this.historyIndex < this.history.length - 1) {
      this.historyIndex++;
      const path = this.history[this.historyIndex];
      this.loadDirectory(path, 1, false);
    }
  }

  updateNavigationButtons() {
    // 更新后退按钮
    this.backBtn.disabled = this.historyIndex <= 0;
    // 更新前进按钮
    this.forwardBtn.disabled = this.historyIndex >= this.history.length - 1;
  }

  // ============ 文件项事件绑定 ============

  bindFileItemEvents(fileItem, file) {
    // 双击事件
    fileItem.addEventListener('dblclick', () => {
      if (file.isDir) {
        this.loadDirectory(file.path);
      } else {
        this.openFile(file.path);
      }
    });

    // 单击选择
    fileItem.addEventListener('click', (e) => {
      // 如果点击的是下载按钮,不处理选择
      if (e.target.closest('.file-download-btn')) {
        return;
      }

      if (!e.ctrlKey && !e.metaKey) {
        this.clearSelection();
      }
      this.toggleSelection(fileItem);
    });

    // 右键菜单
    fileItem.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      if (!fileItem.classList.contains('selected')) {
        this.clearSelection();
        this.toggleSelection(fileItem);
      }
      window.ContextMenuManager.show(e.clientX, e.clientY, {
        paths: Array.from(this.selectedFiles),
        isDir: file.isDir
      });
    });

    // 下载按钮点击事件 (只对文件)
    if (!file.isDir) {
      const downloadBtn = fileItem.querySelector('.file-download-btn');
      if (downloadBtn) {
        downloadBtn.addEventListener('click', async (e) => {
          e.stopPropagation();
          await window.FileOperations.pullSingleFile(file.path, file.name, this.currentDevice);
        });
      }
    }
  }

  // ============ 文件选择 ============

  toggleSelection(item) {
    item.classList.toggle('selected');
    const path = item.dataset.path;

    if (item.classList.contains('selected')) {
      this.selectedFiles.add(path);
    } else {
      this.selectedFiles.delete(path);
    }
  }

  clearSelection() {
    document.querySelectorAll('.file-item.selected').forEach(item => {
      item.classList.remove('selected');
    });
    this.selectedFiles.clear();
  }

  // ============ 文件操作 ============

  async createFolder() {
    window.rLog('createFolder 方法被调用');
    if (!this.currentDevice) {
      window.rWarn('请先选择设备');
      return;
    }
    const result = await window.FileOperations.createFolder(this.currentPath, this.currentDevice);
    if (result.success) {
      // 重新加载目录
      await this.loadDirectory(this.currentPath);
      // 找到新创建的文件夹并进入重命名模式
      const newFolderPath = result.newPath;
      const fileItem = document.querySelector(`.file-item[data-path="${newFolderPath}"]`);
      if (fileItem) {
        this.startInlineRename(fileItem, newFolderPath);
      }
    }
  }

  openFile(filePath) {
    // TODO: 实现文件预览功能
    window.rLog(`打开文件: ${filePath}`);
  }

  /**
   * 启动内联重命名
   * @param {HTMLElement} fileItem - 文件项元素
   * @param {string} filePath - 文件路径
   */
  startInlineRename(fileItem, filePath) {
    const fileName = filePath.split('/').filter(p => p).pop();
    const nameElement = fileItem.querySelector('.file-item-name span');

    if (!nameElement) {
      window.rError('未找到文件名元素');
      return;
    }

    // 创建输入框
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'file-name-input';
    input.value = fileName;
    input.style.cssText = `
      background: var(--input-bg, #3c3c3c);
      border: 1px solid var(--accent-primary, #f9a825);
      border-radius: 2px;
      color: var(--text-primary, #cccccc);
      font-size: 13px;
      padding: 2px 4px;
      outline: none;
      width: 100%;
      font-family: inherit;
    `;

    // 替换名称元素为输入框
    nameElement.style.display = 'none';
    nameElement.parentNode.insertBefore(input, nameElement);

    // 聚焦并选择文本
    input.focus();
    input.select();

    // 完成重命名
    const finishRename = async () => {
      const newName = input.value.trim();

      // 移除输入框,恢复名称显示
      input.remove();
      nameElement.style.display = '';

      if (!newName || newName === fileName) {
        window.rLog('用户取消重命名或未修改');
        return;
      }

      // 执行重命名
      const result = await window.FileOperations.renameFile(filePath, newName, this.currentDevice);
      if (result.success) {
        // 刷新目录
        this.clearSelection();
        this.loadDirectory(this.currentPath);
      } else {
        // 重命名失败,恢复原名称
        nameElement.textContent = fileName;
      }
    };

    // 取消重命名
    const cancelRename = () => {
      window.rLog('取消重命名');
      input.remove();
      nameElement.style.display = '';
    };

    // 回车确认
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        finishRename();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        cancelRename();
      }
    });

    // 失去焦点时确认
    input.addEventListener('blur', finishRename);
  }

  // ============ 搜索 ============

  async handleSearch() {
    const query = this.searchInput.value.trim();

    // 如果搜索框为空,重新加载当前目录
    if (!query) {
      this.loadDirectory(this.currentPath);
      return;
    }

    if (!this.currentDevice) {
      window.rWarn('请先选择设备');
      return;
    }

    try {
      window.rLog(`在当前目录 ${this.currentPath} 搜索: ${query}`);

      // 先加载当前目录的文件列表
      const result = await window.api.tkeFileTree({
        path: this.currentPath,
        level: 1,
        deviceId: this.currentDevice
      });

      if (!result.success) {
        throw new Error(result.error || '加载目录失败');
      }

      const files = window.FileRenderer.parseTreeOutput(result.output, this.currentPath);

      // 在当前目录文件中进行过滤(不区分大小写)
      const queryLower = query.toLowerCase();
      const filteredFiles = files.filter(file =>
        file.name.toLowerCase().includes(queryLower)
      );

      if (filteredFiles.length === 0) {
        window.FileRenderer.showEmptyState(
          this.fileListContent,
          `No results for "${query}" in current directory`
        );
        window.rLog(`在当前目录未找到匹配 "${query}" 的文件`);
        return;
      }

      window.rLog(`在当前目录找到 ${filteredFiles.length} 个匹配结果`);

      const stats = window.FileRenderer.renderFileList(
        filteredFiles,
        this.fileListContent,
        (fileItem, file) => this.bindFileItemEvents(fileItem, file)
      );

      // 更新统计信息
      window.FileRenderer.updateDirectoryStats(this.directoryStats, stats.dirCount, stats.fileCount);

    } catch (error) {
      window.rError('搜索失败:', error);
      window.FileRenderer.showError(this.fileListContent, 'Search failed: ' + error.message);
    }
  }
}

// 初始化控制器(在app.js中调用)
window.FileExplorerController = FileExplorerController;
