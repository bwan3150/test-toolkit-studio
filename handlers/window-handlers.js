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
    mainWindow.loadFile('renderer/login.html');
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
}

module.exports = {
  registerWindowHandlers
};