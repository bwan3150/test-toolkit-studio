// File Icons Module - 文件图标模块
// 使用Iconify的vscode-icons图标集

window.FileIcons = {
  /**
   * 获取文件图标
   * @param {string} fileName - 文件名
   * @param {boolean} isDir - 是否是目录
   * @returns {string} iconify-icon元素的HTML或本地SVG
   */
  getIcon(fileName, isDir) {
    if (isDir) {
      return '<iconify-icon icon="vscode-icons:default-folder" width="18" height="18"></iconify-icon>';
    }

    const ext = this.getFileExtension(fileName);

    // APK使用本地Android图标 (绿色)
    if (ext === 'apk') {
      return `<img src="../../assets/icons/device-page/android.svg" width="18" height="18" style="flex-shrink: 0; filter: brightness(0) saturate(100%) invert(64%) sepia(98%) saturate(391%) hue-rotate(66deg) brightness(91%) contrast(87%);" />`;
    }

    // IPA使用本地Apple图标 (白色)
    if (ext === 'ipa') {
      return `<img src="../../assets/icons/device-page/apple.svg" width="18" height="18" style="flex-shrink: 0; filter: brightness(0) saturate(100%) invert(100%);" />`;
    }

    const iconName = this.getIconName(ext, fileName);
    return `<iconify-icon icon="${iconName}" width="18" height="18" onload="this.style.display='inline-block'" onerror="this.outerHTML='${this.getDefaultFileIcon()}'"></iconify-icon>`;
  },

  /**
   * 获取默认文件图标(SVG保底)
   */
  getDefaultFileIcon() {
    return '<svg viewBox="0 0 24 24" width="18" height="18" style="flex-shrink: 0;"><path fill="#cccccc" d="M6 2c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6H6zm7 7V3.5L18.5 9H13z"/></svg>';
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
   * 根据扩展名获取图标名称
   * @param {string} ext - 文件扩展名
   * @param {string} fileName - 完整文件名
   * @returns {string} Iconify图标名称
   */
  getIconName(ext, fileName) {
    // 特殊文件名
    const fileNameLower = fileName.toLowerCase();
    if (fileNameLower === 'readme.md') return 'vscode-icons:file-type-readme';
    if (fileNameLower === 'package.json') return 'vscode-icons:file-type-node';
    if (fileNameLower === '.gitignore') return 'vscode-icons:file-type-git';

    // 根据扩展名匹配
    const iconMap = {
      // 图片
      'jpg': 'vscode-icons:file-type-image',
      'jpeg': 'vscode-icons:file-type-image',
      'png': 'vscode-icons:file-type-image',
      'gif': 'vscode-icons:file-type-image',
      'bmp': 'vscode-icons:file-type-image',
      'webp': 'vscode-icons:file-type-image',
      'svg': 'vscode-icons:file-type-svg',
      'ico': 'vscode-icons:file-type-image',

      // 视频
      'mp4': 'vscode-icons:file-type-video',
      'avi': 'vscode-icons:file-type-video',
      'mkv': 'vscode-icons:file-type-video',
      'mov': 'vscode-icons:file-type-video',
      'wmv': 'vscode-icons:file-type-video',
      'flv': 'vscode-icons:file-type-video',
      'webm': 'vscode-icons:file-type-video',
      'm4v': 'vscode-icons:file-type-video',

      // 音频
      'mp3': 'vscode-icons:file-type-audio',
      'wav': 'vscode-icons:file-type-audio',
      'flac': 'vscode-icons:file-type-audio',
      'aac': 'vscode-icons:file-type-audio',
      'ogg': 'vscode-icons:file-type-audio',
      'm4a': 'vscode-icons:file-type-audio',
      'wma': 'vscode-icons:file-type-audio',

      // 压缩文件
      'zip': 'vscode-icons:file-type-zip',
      'rar': 'vscode-icons:file-type-zip',
      '7z': 'vscode-icons:file-type-zip',
      'tar': 'vscode-icons:file-type-zip',
      'gz': 'vscode-icons:file-type-zip',
      'bz2': 'vscode-icons:file-type-zip',
      'xz': 'vscode-icons:file-type-zip',

      // 文档
      'pdf': 'vscode-icons:file-type-pdf2',
      'doc': 'vscode-icons:file-type-word',
      'docx': 'vscode-icons:file-type-word',
      'xls': 'vscode-icons:file-type-excel',
      'xlsx': 'vscode-icons:file-type-excel',
      'ppt': 'vscode-icons:file-type-powerpoint',
      'pptx': 'vscode-icons:file-type-powerpoint',
      'txt': 'vscode-icons:file-type-text',
      'md': 'vscode-icons:file-type-markdown',
      'rtf': 'vscode-icons:file-type-text',

      // 代码文件
      'js': 'vscode-icons:file-type-js-official',
      'ts': 'vscode-icons:file-type-typescript-official',
      'jsx': 'vscode-icons:file-type-reactjs',
      'tsx': 'vscode-icons:file-type-reactts',
      'html': 'vscode-icons:file-type-html',
      'css': 'vscode-icons:file-type-css',
      'scss': 'vscode-icons:file-type-scss',
      'sass': 'vscode-icons:file-type-sass',
      'json': 'vscode-icons:file-type-json',
      'xml': 'vscode-icons:file-type-xml',
      'java': 'vscode-icons:file-type-java',
      'py': 'vscode-icons:file-type-python',
      'cpp': 'vscode-icons:file-type-cpp3',
      'c': 'vscode-icons:file-type-c3',
      'h': 'vscode-icons:file-type-c3',
      'go': 'vscode-icons:file-type-go',
      'rs': 'vscode-icons:file-type-rust',
      'php': 'vscode-icons:file-type-php3',
      'rb': 'vscode-icons:file-type-ruby',
      'swift': 'vscode-icons:file-type-swift',
      'kt': 'vscode-icons:file-type-kotlin',
      'sh': 'vscode-icons:file-type-shell',
      'yaml': 'vscode-icons:file-type-yaml',
      'yml': 'vscode-icons:file-type-yaml',
      'sql': 'vscode-icons:file-type-sql',
    };

    return iconMap[ext] || 'vscode-icons:default-file';
  }
};
