// Store相关的IPC处理器 - 用于保存和读取配置
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

// 简单的内存存储
const store = new Map();

// 获取存储文件路径
function getStorePath(app) {
  return path.join(app.getPath('userData'), 'store.json');
}

// 从文件加载存储
function loadStore(app) {
  try {
    const storePath = getStorePath(app);
    if (fs.existsSync(storePath)) {
      const data = fs.readFileSync(storePath, 'utf8');
      const jsonData = JSON.parse(data);
      Object.entries(jsonData).forEach(([key, value]) => {
        store.set(key, value);
      });
    }
  } catch (error) {
    console.error('加载存储失败:', error);
  }
}

// 保存存储到文件
function saveStore(app) {
  try {
    const storePath = getStorePath(app);
    const jsonData = {};
    store.forEach((value, key) => {
      jsonData[key] = value;
    });
    fs.writeFileSync(storePath, JSON.stringify(jsonData, null, 2));
  } catch (error) {
    console.error('保存存储失败:', error);
  }
}

// 注册存储相关的IPC处理器
function registerStoreHandlers(app) {
  // 初始化时加载存储
  loadStore(app);

  // 获取存储值
  ipcMain.handle('store-get', (event, key, defaultValue = null) => {
    if (store.has(key)) {
      return store.get(key);
    }
    return defaultValue;
  });

  // 设置存储值
  ipcMain.handle('store-set', (event, key, value) => {
    store.set(key, value);
    saveStore(app);
    return { success: true };
  });

  // 删除存储值
  ipcMain.handle('store-delete', (event, key) => {
    const result = store.delete(key);
    if (result) {
      saveStore(app);
    }
    return { success: result };
  });

  // 获取所有存储键
  ipcMain.handle('store-keys', () => {
    return Array.from(store.keys());
  });

  // 清空存储
  ipcMain.handle('store-clear', () => {
    store.clear();
    saveStore(app);
    return { success: true };
  });

  // 检查键是否存在
  ipcMain.handle('store-has', (event, key) => {
    return store.has(key);
  });
}

module.exports = {
  registerStoreHandlers
};