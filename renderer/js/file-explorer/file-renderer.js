// File Renderer Module - 文件列表UI渲染模块
// 负责解析目录输出、渲染文件列表、创建文件项DOM

window.FileRenderer = {
  /**
   * 解析tree命令输出为文件列表
   * @param {string} output - tree命令的输出
   * @param {string} basePath - 基础路径
   * @returns {Array} 文件列表数组
   */
  parseTreeOutput(output, basePath) {
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

    return files;
  },

  /**
   * 渲染文件列表
   * @param {Array} files - 文件列表
   * @param {HTMLElement} container - 容器元素
   * @param {Function} onFileItemCreated - 文件项创建后的回调
   * @returns {Object} 返回统计信息 {dirCount, fileCount}
   */
  renderFileList(files, container, onFileItemCreated) {
    if (files.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <svg viewBox="0 0 24 24" width="64" height="64">
            <path d="M20 6h-8l-2-2H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2zm0 12H4V8h16v10z"/>
          </svg>
          <p>Empty directory</p>
        </div>
      `;
      return { dirCount: 0, fileCount: 0 };
    }

    // 排序：目录在前,文件在后
    files.sort((a, b) => {
      if (a.isDir && !b.isDir) return -1;
      if (!a.isDir && b.isDir) return 1;
      return a.name.localeCompare(b.name);
    });

    container.innerHTML = '';
    files.forEach(file => {
      const fileItem = this.createFileItem(file);
      container.appendChild(fileItem);

      // 回调通知文件项已创建,用于绑定事件
      if (onFileItemCreated) {
        onFileItemCreated(fileItem, file);
      }
    });

    // 返回统计信息
    const dirCount = files.filter(f => f.isDir).length;
    const fileCount = files.filter(f => !f.isDir).length;
    return { dirCount, fileCount };
  },

  /**
   * 创建文件项DOM元素
   * @param {Object} file - 文件对象
   * @returns {HTMLElement} 文件项DOM
   */
  createFileItem(file) {
    const item = document.createElement('div');
    item.className = 'file-item';
    item.dataset.path = file.path;
    item.dataset.isDir = file.isDir;
    item.dataset.fileName = file.name;

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
        ${!file.isDir ? `
          <button class="btn btn-icon btn-sm file-download-btn" title="取出此文件">
            <svg viewBox="0 0 24 24" width="14" height="14">
              <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/>
            </svg>
          </button>
        ` : ''}
      </div>
    `;

    return item;
  },

  /**
   * 更新目录统计信息
   * @param {HTMLElement} statsElement - 统计信息元素
   * @param {number} dirCount - 目录数量
   * @param {number} fileCount - 文件数量
   */
  updateDirectoryStats(statsElement, dirCount, fileCount) {
    if (!statsElement) return;

    const statsText = statsElement.querySelector('.stats-text');
    if (statsText) {
      const parts = [];
      if (dirCount > 0) {
        parts.push(`${dirCount} ${dirCount === 1 ? 'folder' : 'folders'}`);
      }
      if (fileCount > 0) {
        parts.push(`${fileCount} ${fileCount === 1 ? 'file' : 'files'}`);
      }
      statsText.textContent = parts.length > 0 ? parts.join(', ') : 'Empty';
    }
  },

  /**
   * 更新路径面包屑
   * @param {HTMLElement} breadcrumbElement - 面包屑容器
   * @param {string} currentPath - 当前路径
   * @param {Function} onPathClick - 路径点击回调
   */
  updatePathBreadcrumb(breadcrumbElement, currentPath, onPathClick) {
    const parts = currentPath.split('/').filter(p => p);
    breadcrumbElement.innerHTML = '';

    let currentPath_build = '/';
    parts.forEach((part, index) => {
      currentPath_build += part + '/';
      const pathSpan = document.createElement('span');
      pathSpan.className = 'path-item';
      pathSpan.dataset.path = currentPath_build;
      pathSpan.textContent = part;

      if (onPathClick) {
        pathSpan.addEventListener('click', () => onPathClick(currentPath_build));
      }

      breadcrumbElement.appendChild(pathSpan);
    });
  },

  /**
   * 显示空状态
   * @param {HTMLElement} container - 容器元素
   * @param {string} message - 提示信息
   */
  showEmptyState(container, message) {
    container.innerHTML = `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" width="64" height="64">
          <path d="M20 6h-8l-2-2H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2zm0 12H4V8h16v10z"/>
        </svg>
        <p>${message}</p>
      </div>
    `;
  },

  /**
   * 显示错误信息
   * @param {HTMLElement} container - 容器元素
   * @param {string} message - 错误信息
   */
  showError(container, message) {
    container.innerHTML = `
      <div class="empty-state">
        <p style="color: var(--accent-danger);">${message}</p>
      </div>
    `;
  }
};
