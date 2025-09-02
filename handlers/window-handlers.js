// 窗口相关的IPC处理器
const { ipcMain, BrowserWindow, dialog } = require('electron');

// 注册所有窗口相关的IPC处理器
function registerWindowHandlers(mainWindow) {
  // 窗口最小化
  ipcMain.handle('window-minimize', () => {
    if (mainWindow) {
      mainWindow.minimize();
    }
  });

  // 窗口最大化/还原
  ipcMain.handle('window-maximize', () => {
    if (mainWindow) {
      if (mainWindow.isMaximized()) {
        mainWindow.unmaximize();
      } else {
        mainWindow.maximize();
      }
    }
    return mainWindow ? mainWindow.isMaximized() : false;
  });

  // 窗口关闭
  ipcMain.handle('window-close', () => {
    if (mainWindow) {
      mainWindow.close();
    }
  });

  // 获取窗口最大化状态
  ipcMain.handle('window-is-maximized', () => {
    return mainWindow ? mainWindow.isMaximized() : false;
  });

  // 导航到主应用
  ipcMain.handle('navigate-to-app', async () => {
    mainWindow.loadFile('renderer/index.html');
    return { success: true };
  });

  // 导航到登录页
  ipcMain.handle('navigate-to-login', async () => {
    mainWindow.loadFile('renderer/html/login.html');
    return { success: true };
  });

  // 显示保存文件对话框
  ipcMain.handle('show-save-dialog', async (event, options) => {
    try {
      const result = await dialog.showSaveDialog(mainWindow, options);
      return result;
    } catch (error) {
      console.error('Failed to show save dialog:', error);
      return { canceled: true };
    }
  });

  // 选择目录对话框
  ipcMain.handle('select-directory', async () => {
    try {
      const result = await dialog.showOpenDialog(mainWindow, {
        properties: ['openDirectory', 'createDirectory'],
        title: '选择项目目录',
        buttonLabel: '选择'
      });
      
      if (!result.canceled && result.filePaths.length > 0) {
        return result.filePaths[0];
      }
    } catch (error) {
      console.error('选择目录出错:', error);
    }
    return null;
  });

  // 选择文件对话框
  ipcMain.handle('select-file', async (event, filters) => {
    const result = await dialog.showOpenDialog(mainWindow, {
      properties: ['openFile'],
      filters: filters || [{ name: '所有文件', extensions: ['*'] }]
    });
    
    if (!result.canceled) {
      return result.filePaths[0];
    }
    return null;
  });

  // 读取文件
  ipcMain.handle('read-file', async (event, filepath) => {
    try {
      const content = await require('fs').promises.readFile(filepath, 'utf-8');
      return { success: true, content };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 写入文件
  ipcMain.handle('write-file', async (event, filepath, content) => {
    try {
      await require('fs').promises.writeFile(filepath, content, 'utf-8');
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerWindowHandlers
};