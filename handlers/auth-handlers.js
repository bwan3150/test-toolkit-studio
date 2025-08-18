// 用户认证相关的IPC处理器
const { ipcMain } = require('electron');

// 注册认证相关的IPC处理器
function registerAuthHandlers() {
  // 用户登录
  ipcMain.handle('login', async (event, credentials) => {
    try {
      // 模拟登录逻辑
      console.log('Login attempt:', credentials.username);
      return {
        success: true,
        token: 'mock-token',
        user: {
          username: credentials.username,
          email: credentials.email || `${credentials.username}@example.com`
        }
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 刷新token
  ipcMain.handle('refresh-token', async () => {
    try {
      return {
        success: true,
        token: 'refreshed-mock-token'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 用户登出
  ipcMain.handle('logout', async () => {
    try {
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerAuthHandlers
};