// 文件系统相关的IPC处理器
// 负责文件/目录操作
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

// 注册文件系统相关的IPC处理器
function registerFilesystemHandlers(app) {
  // 获取用户数据路径
  ipcMain.handle('get-user-data-path', () => {
    return app.getPath('userData');
  });

  // 检查文件/目录是否存在
  ipcMain.handle('file-exists', async (event, filePath) => {
    try {
      return {
        success: true,
        exists: fs.existsSync(filePath),
        path: filePath
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 获取目录内容
  ipcMain.handle('read-directory', async (event, dirPath) => {
    try {
      if (!dirPath || typeof dirPath !== 'string') {
        return { success: false, error: '无效的目录路径' };
      }

      if (!fs.existsSync(dirPath)) {
        return { success: false, error: '目录不存在' };
      }

      const items = fs.readdirSync(dirPath);
      const contents = items.map(item => {
        const itemPath = path.join(dirPath, item);
        const stats = fs.statSync(itemPath);
        return {
          name: item,
          path: itemPath,
          isDirectory: stats.isDirectory(),
          isFile: stats.isFile(),
          size: stats.size,
          mtime: stats.mtime.toISOString()
        };
      });

      return {
        success: true,
        contents: contents,
        path: dirPath
      };

    } catch (error) {
      console.error('读取目录失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerFilesystemHandlers
};
