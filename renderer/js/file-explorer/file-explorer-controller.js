// File Explorer Controller - 设备文件系统浏览器主控制器

class FileExplorerController {
  constructor() {
    this.currentPath = '/sdcard/';
    this.currentDevice = null;
    this.selectedFiles = new Set();
    this.fileHistory = ['/sdcard/'];
    this.historyIndex = 0;

    this.initElements();
    this.initEventListeners();
    this.loadDevices();
  }

  initElements() {
    // 设备选择器
    this.deviceSelect = document.getElementById('fileExplorerDeviceSelect');
    this.refreshDeviceBtn = document.getElementById('refreshFileExplorerDeviceBtn');

    // 路径导航
    this.pathBreadcrumb = document.getElementById('pathBreadcrumb');

    // 工具栏按钮
    this.backBtn = document.getElementById('backBtn');
    this.refreshBtn = document.getElementById('refreshBtn');
    this.newFolderBtn = document.getElementById('newFolderBtn');
    this.uploadBtn = document.getElementById('uploadBtn');
    this.downloadBtn = document.getElementById('downloadBtn');
    this.deleteBtn = document.getElementById('deleteBtn');
    this.searchInput = document.getElementById('searchInput');

    // 文件列表
    this.fileListContent = document.getElementById('fileListContent');

    // 右键菜单
    this.contextMenu = document.getElementById('fileContextMenu');
  }

  initEventListeners() {
    // 设备选择
    this.deviceSelect.addEventListener('change', async () => {
      this.currentDevice = this.deviceSelect.value;
      if (this.currentDevice) {
        // 保存选择的设备
        const { ipcRenderer } = window.AppGlobals;
        await ipcRenderer.invoke('store-set', 'selected_device', this.currentDevice);
        // 加载目录
        this.loadDirectory(this.currentPath);
      } else {
        // 清空显示
        this.showEmptyState('Select a device to browse files');
      }
    });

    this.refreshDeviceBtn.addEventListener('click', () => this.loadDevices());

    // 工具栏按钮
    this.backBtn.addEventListener('click', () => this.navigateUp());
    this.refreshBtn.addEventListener('click', () => this.loadDirectory(this.currentPath));
    this.newFolderBtn.addEventListener('click', () => this.createFolder());
    this.uploadBtn.addEventListener('click', () => this.uploadFile());
    this.downloadBtn.addEventListener('click', () => this.downloadSelected());
    this.deleteBtn.addEventListener('click', () => this.deleteSelected());

    // 搜索
    this.searchInput.addEventListener('input', (e) => this.handleSearch(e.target.value));

    // 右键菜单
    document.addEventListener('click', () => this.hideContextMenu());
    this.contextMenu.addEventListener('click', (e) => {
      if (e.target.classList.contains('context-menu-item')) {
        const action = e.target.dataset.action;
        this.handleContextMenuAction(action);
      }
    });

    // 全局键盘快捷键
    document.addEventListener('keydown', (e) => {
      if (document.getElementById('fileExplorerPage').classList.contains('active')) {
        this.handleKeyboard(e);
      }
    });
  }

  // 加载设备列表
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
        // 自动加载默认目录
        this.loadDirectory(this.currentPath);
      }
    } catch (error) {
      window.rError('加载设备列表失败:', error);
      this.showError('Failed to load devices: ' + error.message);
    }
  }

  // 加载目录内容
  async loadDirectory(path, level = 1) {
    if (!this.currentDevice) {
      this.showEmptyState('Select a device to browse files');
      return;
    }

    try {
      window.rLog(`加载目录: ${path}`);
      this.currentPath = path;
      this.updatePathBreadcrumb();
      this.updateBackButton();

      // 显示加载状态
      this.fileListContent.innerHTML = `
        <div class="empty-state">
          <p>Loading...</p>
        </div>
      `;

      const result = await window.api.tkeFileLs({
        path: path,
        level: level,
        deviceId: this.currentDevice
      });

      if (!result.success) {
        throw new Error(result.error || '加载目录失败');
      }

      this.parseAndRenderDirectory(result.output, path);
    } catch (error) {
      window.rError('加载目录失败:', error);
      this.showError('Failed to load directory: ' + error.message);
    }
  }

  // 解析并渲染目录内容
  parseAndRenderDirectory(output, basePath) {
    const lines = output.trim().split('\n');
    const files = [];

    // 解析 tree 输出
    for (let i = 1; i < lines.length; i++) {
      const line = lines[i];
      if (!line || line.includes('directories,') || line.includes('files')) {
        continue;
      }

      // 解析树形结构
      const match = line.match(/[├└]── (.+?)(?:\s+\((.+?)\))?$/);
      if (match) {
        const name = match[1].trim();
        const size = match[2] || '';
        const isDir = !size; // 如果没有大小信息,通常是目录

        files.push({
          name: name,
          size: size,
          isDir: isDir,
          path: `${basePath.replace(/\/$/, '')}/${name}`
        });
      }
    }

    this.renderFileList(files);
  }

  // 渲染文件列表
  renderFileList(files) {
    if (files.length === 0) {
      this.showEmptyState('Empty directory');
      return;
    }

    // 排序：目录在前,文件在后
    files.sort((a, b) => {
      if (a.isDir && !b.isDir) return -1;
      if (!a.isDir && b.isDir) return 1;
      return a.name.localeCompare(b.name);
    });

    this.fileListContent.innerHTML = '';
    files.forEach(file => {
      const fileItem = this.createFileItem(file);
      this.fileListContent.appendChild(fileItem);
    });
  }

  // 创建文件项 DOM
  createFileItem(file) {
    const item = document.createElement('div');
    item.className = 'file-item';
    item.dataset.path = file.path;
    item.dataset.isDir = file.isDir;

    const icon = file.isDir
      ? '<svg viewBox="0 0 24 24"><path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>'
      : '<svg viewBox="0 0 24 24"><path d="M6 2c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6H6zm7 7V3.5L18.5 9H13z"/></svg>';

    item.innerHTML = `
      <div class="file-item-name ${file.isDir ? 'is-folder' : ''}">
        ${icon}
        <span>${file.name}</span>
      </div>
      <div class="file-item-size">${file.size}</div>
      <div class="file-item-actions">
        <button class="btn btn-icon btn-sm" title="Download">
          <svg viewBox="0 0 24 24" width="14" height="14">
            <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/>
          </svg>
        </button>
      </div>
    `;

    // 双击事件
    item.addEventListener('dblclick', () => {
      if (file.isDir) {
        this.navigateToDirectory(file.path);
      } else {
        this.openFile(file);
      }
    });

    // 单击选择
    item.addEventListener('click', (e) => {
      if (!e.ctrlKey && !e.metaKey) {
        this.clearSelection();
      }
      this.toggleSelection(item);
    });

    // 右键菜单
    item.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      if (!item.classList.contains('selected')) {
        this.clearSelection();
        this.toggleSelection(item);
      }
      this.showContextMenu(e.clientX, e.clientY);
    });

    return item;
  }

  // 导航到目录
  navigateToDirectory(path) {
    this.loadDirectory(path);
  }

  // 返回上一级
  navigateUp() {
    const parts = this.currentPath.split('/').filter(p => p);
    if (parts.length > 1) {
      parts.pop();
      const newPath = '/' + parts.join('/') + '/';
      this.loadDirectory(newPath);
    }
  }

  // 更新路径面包屑
  updatePathBreadcrumb() {
    const parts = this.currentPath.split('/').filter(p => p);
    this.pathBreadcrumb.innerHTML = '';

    let currentPath = '/';
    parts.forEach((part, index) => {
      const pathItem = document.createElement('span');
      pathItem.className = 'path-item';
      currentPath += part + '/';
      pathItem.dataset.path = currentPath;
      pathItem.textContent = part;

      pathItem.addEventListener('click', () => {
        this.loadDirectory(pathItem.dataset.path);
      });

      this.pathBreadcrumb.appendChild(pathItem);
    });
  }

  // 更新返回按钮状态
  updateBackButton() {
    const parts = this.currentPath.split('/').filter(p => p);
    this.backBtn.disabled = parts.length <= 1;
  }

  // 文件选择相关
  toggleSelection(item) {
    item.classList.toggle('selected');
    const path = item.dataset.path;

    if (item.classList.contains('selected')) {
      this.selectedFiles.add(path);
    } else {
      this.selectedFiles.delete(path);
    }

    this.updateToolbarButtons();
  }

  clearSelection() {
    document.querySelectorAll('.file-item.selected').forEach(item => {
      item.classList.remove('selected');
    });
    this.selectedFiles.clear();
    this.updateToolbarButtons();
  }

  updateToolbarButtons() {
    const hasSelection = this.selectedFiles.size > 0;
    this.downloadBtn.disabled = !hasSelection;
    this.deleteBtn.disabled = !hasSelection;
  }

  // 右键菜单
  showContextMenu(x, y) {
    this.contextMenu.style.display = 'block';
    this.contextMenu.style.left = x + 'px';
    this.contextMenu.style.top = y + 'px';
  }

  hideContextMenu() {
    this.contextMenu.style.display = 'none';
  }

  async handleContextMenuAction(action) {
    this.hideContextMenu();

    switch (action) {
      case 'open':
        // TODO: 实现打开文件
        break;
      case 'rename':
        await this.renameSelected();
        break;
      case 'copy':
        // TODO: 实现复制
        break;
      case 'download':
        await this.downloadSelected();
        break;
      case 'delete':
        await this.deleteSelected();
        break;
    }
  }

  // 文件操作
  async createFolder() {
    const name = prompt('Enter folder name:');
    if (!name) return;

    try {
      const newPath = `${this.currentPath}${name}`;
      const result = await window.api.tkeFileMkdir({
        path: newPath,
        deviceId: this.currentDevice
      });

      if (result.success) {
        window.rLog(`创建文件夹成功: ${newPath}`);
        this.loadDirectory(this.currentPath);
      } else {
        throw new Error(result.error);
      }
    } catch (error) {
      window.rError('创建文件夹失败:', error);
      alert('Failed to create folder: ' + error.message);
    }
  }

  async deleteSelected() {
    if (this.selectedFiles.size === 0) return;

    if (!confirm(`Delete ${this.selectedFiles.size} item(s)?`)) {
      return;
    }

    try {
      for (const filePath of this.selectedFiles) {
        const result = await window.api.tkeFileRm({
          path: filePath,
          deviceId: this.currentDevice
        });

        if (!result.success) {
          throw new Error(result.error);
        }
      }

      window.rLog(`删除 ${this.selectedFiles.size} 个文件`);
      this.clearSelection();
      this.loadDirectory(this.currentPath);
    } catch (error) {
      window.rError('删除文件失败:', error);
      alert('Failed to delete: ' + error.message);
    }
  }

  async renameSelected() {
    if (this.selectedFiles.size !== 1) {
      alert('Please select exactly one item to rename');
      return;
    }

    const oldPath = Array.from(this.selectedFiles)[0];
    const oldName = oldPath.split('/').pop();
    const newName = prompt('Enter new name:', oldName);

    if (!newName || newName === oldName) return;

    try {
      const newPath = oldPath.replace(oldName, newName);
      const result = await window.api.tkeFileMv({
        source: oldPath,
        dest: newPath,
        deviceId: this.currentDevice
      });

      if (result.success) {
        window.rLog(`重命名成功: ${oldPath} -> ${newPath}`);
        this.clearSelection();
        this.loadDirectory(this.currentPath);
      } else {
        throw new Error(result.error);
      }
    } catch (error) {
      window.rError('重命名失败:', error);
      alert('Failed to rename: ' + error.message);
    }
  }

  async downloadSelected() {
    if (this.selectedFiles.size === 0) return;

    // TODO: 实现文件下载功能
    window.rLog('下载功能待实现');
    alert('Download feature coming soon');
  }

  async uploadFile() {
    // TODO: 实现文件上传功能
    window.rLog('上传功能待实现');
    alert('Upload feature coming soon');
  }

  async openFile(file) {
    // TODO: 实现文件预览功能
    window.rLog(`打开文件: ${file.path}`);
  }

  // 搜索
  async handleSearch(query) {
    if (!query || !this.currentDevice) return;

    try {
      const result = await window.api.tkeFileFind({
        path: this.currentPath,
        pattern: query,
        deviceId: this.currentDevice
      });

      if (result.success) {
        const files = result.output.trim().split('\n')
          .filter(line => line)
          .map(path => ({
            name: path.split('/').pop(),
            path: path,
            size: '',
            isDir: false
          }));

        this.renderFileList(files);
      }
    } catch (error) {
      window.rError('搜索失败:', error);
    }
  }

  // 键盘快捷键
  handleKeyboard(e) {
    // Ctrl/Cmd + A: 全选
    if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
      e.preventDefault();
      document.querySelectorAll('.file-item').forEach(item => {
        item.classList.add('selected');
        this.selectedFiles.add(item.dataset.path);
      });
      this.updateToolbarButtons();
    }

    // Delete: 删除选中
    if (e.key === 'Delete' && this.selectedFiles.size > 0) {
      this.deleteSelected();
    }

    // Backspace: 返回上一级
    if (e.key === 'Backspace' && !this.backBtn.disabled) {
      e.preventDefault();
      this.navigateUp();
    }
  }

  // UI 辅助函数
  showEmptyState(message) {
    this.fileListContent.innerHTML = `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" width="64" height="64">
          <path d="M20 6h-8l-2-2H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2zm0 12H4V8h16v10z"/>
        </svg>
        <p>${message}</p>
      </div>
    `;
  }

  showError(message) {
    this.fileListContent.innerHTML = `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" width="64" height="64">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
        </svg>
        <p style="color: var(--error-color);">${message}</p>
      </div>
    `;
  }
}

// 导出控制器
window.FileExplorerController = FileExplorerController;
