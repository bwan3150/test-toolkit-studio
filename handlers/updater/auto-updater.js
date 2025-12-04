// 自动更新模块
// 使用 electron-updater 实现后台静默下载 + 一键重启更新

const { autoUpdater } = require('electron-updater');
const { ipcMain, dialog } = require('electron');
const log = require('electron-log');
const Store = require('electron-store');
const https = require('https');

const store = new Store();

// 配置日志
log.transports.file.level = 'info';
autoUpdater.logger = log;

// 禁用自动下载（我们手动控制）
autoUpdater.autoDownload = false;
// 禁用自动安装（等待用户确认）
autoUpdater.autoInstallOnAppQuit = true;

// macOS 特殊配置：允许降级（用于测试）
if (process.platform === 'darwin') {
  autoUpdater.allowDowngrade = true;
  log.info('🍎 macOS: 已启用 allowDowngrade');
}

let mainWindow = null;
let updateDownloaded = false;

// 安全发送消息到渲染进程（避免"Object has been destroyed"错误）
function safeSend(channel, data) {
  try {
    if (mainWindow && !mainWindow.isDestroyed() && mainWindow.webContents) {
      mainWindow.webContents.send(channel, data);
      return true;
    }
  } catch (error) {
    // 窗口可能在检查和发送之间被销毁，忽略此错误
    log.warn('Failed to send to renderer (window may be destroyed):', error.message);
  }
  return false;
}

// 设置更新 channel（beta 或 latest）
function setUpdateChannel() {
  const receiveBetaUpdates = store.get('receive_beta_updates', false);
  autoUpdater.allowPrerelease = receiveBetaUpdates;
  log.info(`更新频道设置为: ${receiveBetaUpdates ? 'beta' : 'latest'}`);
}

// 从 S3 获取 release notes
function fetchReleaseNotes(version) {
  return new Promise((resolve) => {
    const url = `https://toolkit-studio-updates.s3.ap-southeast-2.amazonaws.com/release-notes/${version}.md`;

    log.info(`尝试获取 Release Notes: ${url}`);

    https.get(url, (res) => {
      if (res.statusCode !== 200) {
        log.warn(`Release Notes 不存在 (状态码: ${res.statusCode})`);
        resolve(null);
        return;
      }

      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });

      res.on('end', () => {
        log.info('✅ 成功获取 Release Notes');
        resolve(data);
      });
    }).on('error', (err) => {
      log.error('获取 Release Notes 失败:', err.message);
      resolve(null);
    });
  });
}

/**
 * 初始化自动更新
 * @param {BrowserWindow} win - 主窗口实例
 */
function initAutoUpdater(win) {
  mainWindow = win;

  // 设置更新频道
  setUpdateChannel();

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

  // 重新设置更新频道（以防用户改变了设置）
  setUpdateChannel();

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
  autoUpdater.on('update-downloaded', async (info) => {
    log.info('更新下载完成，版本:', info.version);
    updateDownloaded = true;
    sendStatusToWindow('update-downloaded', info);

    try {
      // 从 S3 获取 release notes
      log.info('开始获取 Release Notes...');
      const releaseNotes = await fetchReleaseNotes(info.version);

      if (releaseNotes) {
        log.info('✅ 成功获取 Release Notes，长度:', releaseNotes.length);
      } else {
        log.warn('⚠️  未获取到 Release Notes');
      }

      // 通知渲染进程显示更新提示弹窗
      safeSend('update-ready', {
        version: info.version,
        releaseNotes: releaseNotes,
        releaseDate: info.releaseDate
      });
    } catch (error) {
      log.error('处理更新下载完成事件时出错:', error);
      // 即使获取 release notes 失败，也要通知渲染进程
      safeSend('update-ready', {
        version: info.version,
        releaseNotes: null,
        releaseDate: info.releaseDate
      });
    }
  });

  // 更新错误
  autoUpdater.on('error', (err) => {
    log.error('❌ 更新错误:', err);
    log.error('错误详情:', {
      message: err.message,
      stack: err.stack,
      name: err.name,
      code: err.code
    });
    sendStatusToWindow('update-error', {
      message: err.message || '未知错误',
      code: err.code,
      name: err.name
    });
  });
}

/**
 * 发送更新状态到渲染进程
 */
function sendStatusToWindow(event, data) {
  safeSend('update-status', { event, data });
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

      log.info('⚡ 用户确认安装更新，即将重启应用...');

      try {
        // macOS 和 Windows 需要不同的参数
        const isMac = process.platform === 'darwin';

        if (isMac) {
          log.info('🍎 macOS: 使用 quitAndInstall(false, true)');
          // macOS: isSilent=false, isForceRunAfter=true
          setImmediate(() => {
            autoUpdater.quitAndInstall(false, true);
          });
        } else {
          log.info('🪟 Windows: 使用 quitAndInstall(true, true)');
          // Windows: isSilent=true, isForceRunAfter=true
          setImmediate(() => {
            autoUpdater.quitAndInstall(true, true);
          });
        }

        return { success: true };
      } catch (error) {
        log.error('❌ quitAndInstall 调用失败:', error);
        return { success: false, error: error.message };
      }
    } else {
      log.warn('⚠️  尝试安装更新但 updateDownloaded = false');
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

  // 设置更新频道（beta 或 latest）
  ipcMain.handle('set-update-channel', (event, channel) => {
    try {
      const isBeta = channel === 'beta';
      autoUpdater.allowPrerelease = isBeta;
      log.info(`更新频道已切换为: ${channel}`);
      return { success: true, channel };
    } catch (error) {
      log.error('设置更新频道失败:', error);
      return { success: false, error: error.message };
    }
  });
}

/**
 * 模拟更新（用于测试）
 */
async function simulateUpdate() {
  log.info('🧪 模拟更新：触发 update-ready 事件');

  // 获取当前版本
  const currentVersion = require('electron').app.getVersion();
  const parts = currentVersion.split('.');
  const nextVersion = `${parts[0]}.${parseInt(parts[1]) + 1}.0`;

  // 从 S3 获取 release notes（真实测试）
  log.info(`🧪 模拟版本: ${currentVersion} → ${nextVersion}`);
  const releaseNotes = await fetchReleaseNotes(nextVersion);

  if (releaseNotes) {
    log.info('🧪 成功从 S3 获取 Release Notes');
  } else {
    log.warn('🧪 未找到 Release Notes，将显示默认文本');
  }

  // 模拟更新信息
  const mockUpdateInfo = {
    version: nextVersion,
    releaseNotes: releaseNotes,
    releaseDate: new Date().toISOString()
  };

  // 标记更新已下载（模拟）
  updateDownloaded = true;

  // 发送到渲染进程
  safeSend('update-ready', mockUpdateInfo);
}

module.exports = {
  initAutoUpdater,
  checkForUpdates,
  registerUpdateHandlers
};
