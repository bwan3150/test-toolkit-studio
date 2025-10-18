// 自动更新模块
// 使用 electron-updater 实现后台静默下载 + 一键重启更新

const { autoUpdater } = require('electron-updater');
const { ipcMain, dialog } = require('electron');
const log = require('electron-log');

// 配置日志
log.transports.file.level = 'info';
autoUpdater.logger = log;

// 禁用自动下载（我们手动控制）
autoUpdater.autoDownload = false;
// 禁用自动安装（等待用户确认）
autoUpdater.autoInstallOnAppQuit = true;

let mainWindow = null;
let updateDownloaded = false;

/**
 * 初始化自动更新
 * @param {BrowserWindow} win - 主窗口实例
 */
function initAutoUpdater(win) {
  mainWindow = win;

  // 监听更新事件
  setupUpdateListeners();

  // 检查是否模拟更新（用于测试）
  if (process.env.ELECTRON_SIMULATE_UPDATE === 'true') {
    log.info('🧪 模拟更新模式：将在 3 秒后显示更新弹窗');
    setTimeout(() => {
      simulateUpdate();
    }, 3000);
    return;
  }

  // 应用启动后延迟 3 秒检查更新（避免影响启动速度）
  setTimeout(() => {
    checkForUpdates();
  }, 3000);
}

/**
 * 检查更新
 */
function checkForUpdates() {
  // 开发模式不检查更新
  if (process.env.ELECTRON_DEV_MODE === 'true') {
    log.info('开发模式，跳过更新检查');
    return;
  }

  log.info('开始检查更新...');
  autoUpdater.checkForUpdates();
}

/**
 * 设置更新事件监听器
 */
function setupUpdateListeners() {
  // 检查更新时触发
  autoUpdater.on('checking-for-update', () => {
    log.info('正在检查更新...');
    sendStatusToWindow('checking-for-update');
  });

  // 发现可用更新
  autoUpdater.on('update-available', (info) => {
    log.info('发现新版本:', info.version);
    sendStatusToWindow('update-available', info);

    // 自动开始后台下载
    log.info('开始后台下载更新...');
    autoUpdater.downloadUpdate();
  });

  // 没有可用更新
  autoUpdater.on('update-not-available', (info) => {
    log.info('当前已是最新版本:', info.version);
    sendStatusToWindow('update-not-available', info);
  });

  // 下载进度
  autoUpdater.on('download-progress', (progressObj) => {
    const logMessage = `下载速度: ${progressObj.bytesPerSecond} - 已下载: ${progressObj.percent}% (${progressObj.transferred}/${progressObj.total})`;
    log.info(logMessage);
    sendStatusToWindow('download-progress', progressObj);
  });

  // 下载完成
  autoUpdater.on('update-downloaded', (info) => {
    log.info('更新下载完成，版本:', info.version);
    updateDownloaded = true;
    sendStatusToWindow('update-downloaded', info);

    // 通知渲染进程显示更新提示弹窗
    if (mainWindow && !mainWindow.isDestroyed()) {
      mainWindow.webContents.send('update-ready', {
        version: info.version,
        releaseNotes: info.releaseNotes,
        releaseDate: info.releaseDate
      });
    }
  });

  // 更新错误
  autoUpdater.on('error', (err) => {
    log.error('更新错误:', err);
    sendStatusToWindow('update-error', err);
  });
}

/**
 * 发送更新状态到渲染进程
 */
function sendStatusToWindow(event, data) {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('update-status', { event, data });
  }
}

/**
 * 注册 IPC 处理器
 */
function registerUpdateHandlers() {
  // 手动检查更新
  ipcMain.handle('check-for-updates', async () => {
    try {
      const result = await autoUpdater.checkForUpdates();
      return { success: true, updateInfo: result.updateInfo };
    } catch (error) {
      log.error('检查更新失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 立即安装更新并重启
  ipcMain.handle('install-update-now', () => {
    if (updateDownloaded) {
      // 检查是否为模拟模式
      if (process.env.ELECTRON_SIMULATE_UPDATE === 'true') {
        log.info('🧪 模拟模式：用户点击了安装按钮（不会真的重启）');
        return { success: true, simulated: true };
      }

      log.info('用户确认安装更新，准备重启应用...');
      // quitAndInstall 会关闭应用并安装更新
      setImmediate(() => {
        autoUpdater.quitAndInstall(false, true);
      });
      return { success: true };
    } else {
      return { success: false, error: '更新尚未下载完成' };
    }
  });

  // 获取当前更新状态
  ipcMain.handle('get-update-status', () => {
    return {
      updateDownloaded,
      currentVersion: require('electron').app.getVersion()
    };
  });
}

/**
 * 模拟更新（用于测试）
 */
function simulateUpdate() {
  log.info('🧪 模拟更新：触发 update-ready 事件');

  // 获取当前版本
  const currentVersion = require('electron').app.getVersion();
  const parts = currentVersion.split('.');
  const nextVersion = `${parts[0]}.${parseInt(parts[1]) + 1}.0`;

  // 模拟更新信息
  const mockUpdateInfo = {
    version: nextVersion,
    releaseNotes: `
**New Features:**
- Added automatic update functionality
- Improved UI performance
- Bug fixes and stability improvements

**Technical Changes:**
- Updated to Electron 38
- Enhanced error handling
    `.trim(),
    releaseDate: new Date().toISOString()
  };

  // 标记更新已下载（模拟）
  updateDownloaded = true;

  // 发送到渲染进程
  if (mainWindow && !mainWindow.isDestroyed()) {
    log.info(`🧪 模拟新版本: ${currentVersion} → ${nextVersion}`);
    mainWindow.webContents.send('update-ready', mockUpdateInfo);
  }
}

module.exports = {
  initAutoUpdater,
  checkForUpdates,
  registerUpdateHandlers
};
