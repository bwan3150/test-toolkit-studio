// Toolkit Gateway API 代理处理器
// 负责与 Toolkit 网关服务的通信，包括用户认证、token管理等
const { ipcMain } = require('electron');
const Store = require('electron-store');
const Sentry = require('@sentry/electron/main');

// 初始化store
const store = new Store();
let isAuthenticated = false;
let tokenCheckInterval = null;

// 注册认证相关的IPC处理器
function registerAuthHandlers() {
  // 用户登录
  ipcMain.handle('login', async (event, credentials) => {
    try {
      const axios = require('axios');
      const response = await axios.post(`${credentials.baseUrl}/api/auth/login`, 
        `email=${encodeURIComponent(credentials.email)}&password=${encodeURIComponent(credentials.password)}`,
        {
          headers: {
            'Content-Type': 'application/x-www-form-urlencoded'
          }
        }
      );
      
      const data = response.data;

      // 存储tokens
      store.set('access_token', data.access_token);
      store.set('refresh_token', data.refresh_token);
      store.set('base_url', credentials.baseUrl);
      store.set('token_expiry', new Date(Date.now() + data.expires_in * 1000).toISOString());

      isAuthenticated = true;
      // 启动定期token检查
      startTokenCheck();

      // 设置 Sentry 用户上下文
      await setSentryUser();

      return { success: true, data };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 刷新token
  ipcMain.handle('refresh-token', async () => {
    return await refreshTokenInternal();
  });

  // 用户登出
  ipcMain.handle('logout', async () => {
    store.delete('access_token');
    store.delete('refresh_token');
    store.delete('token_expiry');
    isAuthenticated = false;
    // 停止定期token检查
    stopTokenCheck();

    // 清除 Sentry 用户上下文
    clearSentryUser();

    return { success: true };
  });

  // 获取用户信息
  ipcMain.handle('get-user-info', async () => {
    try {
      const axios = require('axios');
      const token = store.get('access_token');
      const baseUrl = store.get('base_url');
      
      if (!token || !baseUrl) {
        return { success: false, error: '未找到认证token或base URL' };
      }
      
      const response = await axios.get(`${baseUrl}/api/users/me`, {
        headers: {
          'Authorization': `Bearer ${token}`
        }
      });
      
      return { success: true, data: response.data };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });
}

// 内部token刷新函数（避免重复代码）
async function refreshTokenInternal() {
  try {
    const axios = require('axios');
    const refreshToken = store.get('refresh_token');
    const baseUrl = store.get('base_url');
    
    if (!refreshToken || !baseUrl) {
      return { success: false, error: 'Missing refresh token or base URL' };
    }
    
    const response = await axios.post(`${baseUrl}/api/auth/refresh`, 
      { refresh_token: refreshToken },
      {
        headers: {
          'Content-Type': 'application/json'
        }
      }
    );
    
    const data = response.data;
    
    // 更新tokens
    store.set('access_token', data.access_token);
    store.set('refresh_token', data.refresh_token);
    store.set('token_expiry', new Date(Date.now() + data.expires_in * 1000).toISOString());
    
    return { success: true, data };
  } catch (error) {
    console.error('Failed to refresh token:', error.message);
    return { success: false, error: error.message };
  }
}

// 启动定期token检查
function startTokenCheck() {
  // 清除之前的检查间隔
  if (tokenCheckInterval) {
    clearInterval(tokenCheckInterval);
  }
  
  // 每10分钟检查一次token状态
  tokenCheckInterval = setInterval(async () => {
    try {
      if (!isAuthenticated) {
        return;
      }
      
      const tokenExpiry = store.get('token_expiry');
      if (!tokenExpiry) {
        console.log('No token expiry time, stopping periodic check');
        stopTokenCheck();
        return;
      }
      
      const now = new Date();
      const expiry = new Date(tokenExpiry);
      const bufferTime = 5 * 60 * 1000; // 5分钟缓冲
      
      // 如果token将在5分钟内过期，尝试刷新
      if (now.getTime() > (expiry.getTime() - bufferTime)) {
        console.log('Periodic check found token is about to expire, refreshing...');
        
        const result = await refreshTokenInternal();
        if (result.success) {
          console.log('Periodic token refresh succeeded');
        } else {
          console.log('Periodic token refresh failed, redirecting to login page');
          isAuthenticated = false;
          stopTokenCheck();
        }
      }
    } catch (error) {
      console.error('Periodic token check exception:', error);
    }
  }, 10 * 60 * 1000); // 10分钟
  
  console.log('Started periodic token check (every 10 minutes)');
}

// 停止定期token检查
function stopTokenCheck() {
  if (tokenCheckInterval) {
    clearInterval(tokenCheckInterval);
    tokenCheckInterval = null;
    console.log('Stopped periodic token check');
  }
}

// 设置 Sentry 用户上下文
async function setSentryUser() {
  try {
    // 检查是否在开发环境
    const isDev = process.env.ELECTRON_DEV_MODE === 'true';
    if (isDev) {
      return; // 开发环境不设置 Sentry
    }

    const axios = require('axios');
    const token = store.get('access_token');
    const baseUrl = store.get('base_url');

    if (!token || !baseUrl) {
      console.log('No token or baseUrl, skipping Sentry user context');
      return;
    }

    const response = await axios.get(`${baseUrl}/api/users/me`, {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });

    const userData = response.data;

    // 设置 Sentry 用户上下文
    if (Sentry && Sentry.setUser) {
      Sentry.setUser({
        id: userData.id || userData.user_id,
        email: userData.email,
        username: userData.username || userData.name,
      });
      console.log('Sentry 用户上下文已设置:', userData.email);
    }
  } catch (error) {
    console.error('设置 Sentry 用户上下文失败:', error.message);
  }
}

// 清除 Sentry 用户上下文
function clearSentryUser() {
  try {
    // 检查是否在开发环境
    const isDev = process.env.ELECTRON_DEV_MODE === 'true';
    if (isDev) {
      return; // 开发环境不清除 Sentry
    }

    if (Sentry && Sentry.setUser) {
      Sentry.setUser(null);
      console.log('Sentry 用户上下文已清除');
    }
  } catch (error) {
    console.error('清除 Sentry 用户上下文失败:', error.message);
  }
}

// 初始化时检查并设置用户上下文
async function initializeSentryUser() {
  try {
    const token = store.get('access_token');
    if (token) {
      await setSentryUser();
    }
  } catch (error) {
    console.error('初始化 Sentry 用户上下文失败:', error.message);
  }
}

module.exports = {
  registerAuthHandlers,
  initializeSentryUser
};