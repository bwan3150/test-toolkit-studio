// Store相关的IPC处理器 - 用于保存和读取配置
const { ipcMain } = require('electron');
const Store = require('electron-store');

// 初始化electron-store
const store = new Store();

// 注册存储相关的IPC处理器
function registerStoreHandlers(app) {
  // 获取存储值
  ipcMain.handle('store-get', async (event, key) => {
    return store.get(key);
  });

  // 设置存储值
  ipcMain.handle('store-set', async (event, key, value) => {
    store.set(key, value);
    return true;
  });

  // 删除存储值
  ipcMain.handle('store-delete', async (event, key) => {
    store.delete(key);
    return true;
  });

  // 获取所有存储键
  ipcMain.handle('store-keys', async () => {
    return Object.keys(store.store);
  });

  // 清空存储
  ipcMain.handle('store-clear', async () => {
    store.clear();
    return true;
  });

  // 检查键是否存在
  ipcMain.handle('store-has', async (event, key) => {
    return store.has(key);
  });

  // 获取存储大小
  ipcMain.handle('store-size', async () => {
    return store.size;
  });

  // 获取存储路径
  ipcMain.handle('store-path', async () => {
    return store.path;
  });
}

module.exports = {
  registerStoreHandlers
};