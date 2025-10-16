// 系统工具检查相关的IPC处理器
// 负责检查应用版本、TKE、ADB、AAPT等系统工具的状态
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { promisify } = require('util');
const { exec } = require('child_process');
const execPromise = promisify(exec);
const { execTkeAdbCommand } = require('../tke-integration/adb-handlers');

// 注册系统工具检查相关的IPC处理器
function registerSystemHandlers(app) {
  // 获取应用版本信息
  ipcMain.handle('get-app-version', async () => {
    try {
      const packageJson = require('../../../package.json');
      return packageJson.version || 'unknown';
    } catch (error) {
      return 'unknown';
    }
  });

  // 检查TKE状态
  ipcMain.handle('check-tke-status', async () => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const tkeBinaryName = process.platform === 'win32' ? 'tke.exe' : 'tke';
      let tkePath;

      if (app.isPackaged) {
        tkePath = path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinaryName);
      } else {
        tkePath = path.join(__dirname, '..', '..', '..', 'resources', platform, 'toolkit-engine', tkeBinaryName);
      }

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'TKE binary not found' };
      }

      // 测试TKE版本
      const { stdout, stderr } = await execPromise(`"${tkePath}" --version`);

      if (stderr) {
        console.error('TKE stderr:', stderr);
      }

      // 解析版本信息
      const versionMatch = stdout.match(/tke\s+(\d+\.\d+\.\d+)/);
      const version = versionMatch ? versionMatch[1] : stdout.trim();

      return {
        success: true,
        version: version,
        path: tkePath
      };

    } catch (error) {
      console.error('检查TKE状态失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 检查ADB版本（通过TKE内嵌ADB）
  ipcMain.handle('check-adb-version', async () => {
    try {
      const { stdout } = await execTkeAdbCommand(app, null, ['version']);
      const versionMatch = stdout.match(/Version (\d+\.\d+\.\d+)/);
      const version = versionMatch ? versionMatch[1] : 'Unknown';

      return {
        success: true,
        version: version,
        path: 'TKE内嵌ADB'
      };

    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 检查AAPT状态
  ipcMain.handle('check-aapt-status', async () => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const aaptName = process.platform === 'win32' ? 'aapt.exe' : 'aapt';
      let aaptPath;

      if (app.isPackaged) {
        // 对于不同平台使用不同的路径结构
        if (platform === 'darwin') {
          aaptPath = path.join(process.resourcesPath, platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
        } else {
          aaptPath = path.join(process.resourcesPath, platform, 'android-sdk', 'build-tools', 'aapt', aaptName);
        }
      } else {
        // 对于不同平台使用不同的路径结构
        if (platform === 'darwin') {
          aaptPath = path.join(__dirname, '..', '..', '..', 'resources', platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
        } else {
          aaptPath = path.join(__dirname, '..', '..', '..', 'resources', platform, 'android-sdk', 'build-tools', 'aapt', aaptName);
        }
      }

      if (!fs.existsSync(aaptPath)) {
        return { success: false, error: 'AAPT binary not found' };
      }

      // 测试AAPT
      const { stdout, stderr } = await execPromise(`"${aaptPath}" version 2>&1 || echo "Available"`);

      // 从输出中提取版本号，例如：Android Asset Packaging Tool, v0.2-9420752
      let version = 'Available';
      const versionMatch = stdout.match(/v([\d\.\-\w]+)/);
      if (versionMatch) {
        version = versionMatch[1]; // 提取v后面的版本号
      }

      return {
        success: true,
        version: version,
        path: aaptPath
      };

    } catch (error) {
      console.error('检查AAPT状态失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerSystemHandlers
};
