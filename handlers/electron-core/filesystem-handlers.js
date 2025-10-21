// 文件系统相关的IPC处理器
// 负责文件/目录操作
const { ipcMain, shell } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const fsSync = require('fs');

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
        exists: fsSync.existsSync(filePath),
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

      if (!fsSync.existsSync(dirPath)) {
        return { success: false, error: '目录不存在' };
      }

      const items = fsSync.readdirSync(dirPath);
      const contents = items.map(item => {
        const itemPath = path.join(dirPath, item);
        const stats = fsSync.statSync(itemPath);
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

  // 创建新文件
  ipcMain.handle('fs-create-file', async (event, filePath, content = '') => {
    try {
      // 检查文件是否已存在
      if (fsSync.existsSync(filePath)) {
        return { success: false, error: '文件已存在' };
      }

      // 确保父目录存在
      const parentDir = path.dirname(filePath);
      if (!fsSync.existsSync(parentDir)) {
        await fs.mkdir(parentDir, { recursive: true });
      }

      // 创建文件
      await fs.writeFile(filePath, content, 'utf8');

      return {
        success: true,
        path: filePath,
        message: '文件创建成功'
      };
    } catch (error) {
      console.error('创建文件失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 重命名文件或目录
  ipcMain.handle('fs-rename', async (event, oldPath, newPath) => {
    try {
      if (!fsSync.existsSync(oldPath)) {
        return { success: false, error: '源文件/目录不存在' };
      }

      if (fsSync.existsSync(newPath)) {
        return { success: false, error: '目标文件/目录已存在' };
      }

      await fs.rename(oldPath, newPath);

      return {
        success: true,
        oldPath,
        newPath,
        message: '重命名成功'
      };
    } catch (error) {
      console.error('重命名失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 删除文件
  ipcMain.handle('fs-delete-file', async (event, filePath) => {
    try {
      if (!fsSync.existsSync(filePath)) {
        return { success: false, error: '文件不存在' };
      }

      const stats = await fs.stat(filePath);
      if (stats.isDirectory()) {
        return { success: false, error: '请使用删除目录接口' };
      }

      await fs.unlink(filePath);

      return {
        success: true,
        path: filePath,
        message: '文件删除成功'
      };
    } catch (error) {
      console.error('删除文件失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 删除目录（递归）
  ipcMain.handle('fs-delete-directory', async (event, dirPath) => {
    try {
      if (!fsSync.existsSync(dirPath)) {
        return { success: false, error: '目录不存在' };
      }

      const stats = await fs.stat(dirPath);
      if (!stats.isDirectory()) {
        return { success: false, error: '请使用删除文件接口' };
      }

      await fs.rm(dirPath, { recursive: true, force: true });

      return {
        success: true,
        path: dirPath,
        message: '目录删除成功'
      };
    } catch (error) {
      console.error('删除目录失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 复制文件
  ipcMain.handle('fs-copy-file', async (event, sourcePath, targetPath) => {
    try {
      if (!fsSync.existsSync(sourcePath)) {
        return { success: false, error: '源文件不存在' };
      }

      if (fsSync.existsSync(targetPath)) {
        return { success: false, error: '目标文件已存在' };
      }

      // 确保目标目录存在
      const targetDir = path.dirname(targetPath);
      if (!fsSync.existsSync(targetDir)) {
        await fs.mkdir(targetDir, { recursive: true });
      }

      await fs.copyFile(sourcePath, targetPath);

      return {
        success: true,
        sourcePath,
        targetPath,
        message: '文件复制成功'
      };
    } catch (error) {
      console.error('复制文件失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 在文件管理器中显示文件/目录
  ipcMain.handle('fs-show-in-folder', async (event, targetPath) => {
    try {
      if (!fsSync.existsSync(targetPath)) {
        return { success: false, error: '文件/目录不存在' };
      }

      // 使用 shell.showItemInFolder 在文件管理器中显示并选中文件
      shell.showItemInFolder(targetPath);

      return {
        success: true,
        path: targetPath,
        message: '已在文件管理器中打开'
      };
    } catch (error) {
      console.error('在文件管理器中显示失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 创建目录
  ipcMain.handle('fs-create-directory', async (event, dirPath) => {
    try {
      if (fsSync.existsSync(dirPath)) {
        return { success: false, error: '目录已存在' };
      }

      await fs.mkdir(dirPath, { recursive: true });

      return {
        success: true,
        path: dirPath,
        message: '目录创建成功'
      };
    } catch (error) {
      console.error('创建目录失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerFilesystemHandlers
};
