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

    // 路径导航
    this.pathBreadcrumb = document.getElementById('pathBreadcrumb');

    // 工具栏按钮
    this.backBtn = document.getElementById('backBtn');
    this.forwardBtn = document.getElementById('forwardBtn');
    this.refreshBtn = document.getElementById('refreshBtn');
    this.newFolderBtn = document.getElementById('newFolderBtn');
    this.searchInput = document.getElementById('searchInput');
    this.searchBtn = document.getElementById('searchBtn');

    // 文件列表
    this.fileListContent = document.getElementById('fileListContent');

    // 右键菜单
    this.contextMenu = document.getElementById('fileContextMenu');

    // 目录统计
    this.directoryStats = document.getElementById('directoryStats');
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

    window.ContextMenuManager.registerHandler('rename', async (selection) => {
      const path = Array.from(this.selectedFiles)[0];
      if (path) {
        const result = await window.FileOperations.renameFile(path, this.currentDevice);
        if (result.success) {
          this.clearSelection();
          this.loadDirectory(this.currentPath);
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
  }

  initEventListeners() {
    // 设备选择
    this.deviceSelect.addEventListener('change', async () => {
      this.currentDevice = this.deviceSelect.value;
      if (this.currentDevice) {
        // 保存选择的设备
        const { ipcRenderer } = window.AppGlobals;
        await ipcRenderer.invoke('store-set', 'selected_device', this.currentDevice);

        // 更新拖拽模块的设备ID
        window.DragUploadManager.setDeviceId(this.currentDevice);

        // 加载目录
        this.loadDirectory(this.currentPath);
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
      this.newFolderBtn.addEventListener('click', () => this.createFolder());
    }

    // 搜索 - 回车或点击按钮触发
    this.searchInput.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') {
        this.handleSearch();
      }
    });
    this.searchBtn.addEventListener('click', () => this.handleSearch());
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

      // 更新UI - 禁用path breadcrumb的点击
      window.FileRenderer.updatePathBreadcrumb(this.pathBreadcrumb, path, null);
      this.updateNavigationButtons();

      // 显示加载状态
      window.FileRenderer.showEmptyState(this.fileListContent, 'Loading...');

      const result = await window.api.tkeFileLs({
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
    const result = await window.FileOperations.createFolder(this.currentPath, this.currentDevice);
    if (result.success) {
      this.loadDirectory(this.currentPath);
    }
  }

  openFile(filePath) {
    // TODO: 实现文件预览功能
    window.rLog(`打开文件: ${filePath}`);
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
      const result = await window.api.tkeFileLs({
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
