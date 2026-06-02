// 系统工具检查相关的IPC处理器
// 负责检查应用版本和内置工具的版本信息
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { promisify } = require('util');
const { exec } = require('child_process');
const execPromise = promisify(exec);

/**
 * 获取二进制文件的路径，所有工具平铺在 bin/[platform]/ 下
 * @param {string} toolName - 工具名称，如 'tke', 'tke-opencv', 'tester-ai'
 * @param {Electron.App} app - Electron app 实例
 * @returns {string} 二进制文件路径
 */
function getBinaryPath(toolName, app) {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const binaryName = process.platform === 'win32' ? `${toolName}.exe` : toolName;

  const isDevMode = process.env.ELECTRON_DEV_MODE === 'true';

  if (isDevMode) {
    const projectRoot = process.env.ELECTRON_PROJECT_ROOT || process.cwd();
    return path.join(projectRoot, 'bin', platform, binaryName);
  } else if (app.isPackaged) {
    return path.join(process.resourcesPath, platform, binaryName);
  } else {
    return path.join(__dirname, '..', '..', '..', 'bin', platform, binaryName);
  }
}

// 注册系统工具检查相关的IPC处理器
function registerSystemHandlers(app) {
  // 获取应用版本信息
  ipcMain.handle('get-app-version', async () => {
    try {
      // 使用 Electron app.getVersion() 方法，它会自动从 package.json 读取
      // 这种方式在开发和打包环境下都能正常工作
      const version = app.getVersion();
      return version || 'unknown';
    } catch (error) {
      console.error('获取应用版本失败:', error);
      return 'unknown';
    }
  });

  // 获取 TKE 引擎版本
  ipcMain.handle('get-tke-version', async () => {
    try {
      const tkePath = getBinaryPath('tke', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" --version`);
      // 输出格式: "tke 0.6.5-beta"
      // 提取版本号部分
      const match = stdout.trim().match(/tke\s+(.+)/);
      const version = match ? match[1].trim() : stdout.trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 TKE 版本失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取 TKE 内嵌 ADB 版本
  ipcMain.handle('get-tke-adb-version', async () => {
    try {
      const tkePath = getBinaryPath('tke', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" adb --version`);
      // 输出格式: "Android Debug Bridge version 1.0.41\nVersion 36.0.0-13206524\n..."
      // 提取版本号部分
      const match = stdout.trim().match(/Android Debug Bridge version\s+(.+)/);
      const version = match ? match[1].trim() : stdout.trim().split('\n')[0].trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 TKE ADB 版本失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取 TKE 内嵌 AAPT 版本
  ipcMain.handle('get-tke-aapt-version', async () => {
    try {
      const tkePath = getBinaryPath('tke', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" aapt version`);
      // 输出格式: "Android Asset Packaging Tool, v0.2-9420752"
      // 提取版本号部分
      const match = stdout.trim().match(/,\s*v(.+)/);
      const version = match ? match[1].trim() : stdout.trim().split('\n')[0].trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 TKE AAPT 版本失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取 TKE-OpenCV 版本
  ipcMain.handle('get-tke-opencv-version', async () => {
    try {
      const opencvPath = getBinaryPath('tke-opencv', app);

      if (!fs.existsSync(opencvPath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${opencvPath}" --version`);
      // 输出格式: "tke-opencv 0.6.5-beta"
      // 提取版本号部分
      const match = stdout.trim().match(/tke-opencv\s+(.+)/);
      const version = match ? match[1].trim() : stdout.trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 TKE-OpenCV 版本失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取 Tester-AI 版本
  ipcMain.handle('get-tester-ai-version', async () => {
    try {
      const testerAiPath = getBinaryPath('tester-ai', app);

      if (!fs.existsSync(testerAiPath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${testerAiPath}" --version`);
      // 输出格式: "tester-ai 0.6.5-beta"
      // 提取版本号部分
      const match = stdout.trim().match(/tester-ai\s+(.+)/);
      const version = match ? match[1].trim() : stdout.trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 Tester-AI 版本失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取 TKE-Scrcpy 版本
  ipcMain.handle('get-tke-scrcpy-version', async () => {
    try {
      const scrcpyPath = getBinaryPath('tke-scrcpy', app);

      if (!fs.existsSync(scrcpyPath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${scrcpyPath}" --version`);
      // 输出格式: "tke-scrcpy 0.6.5-beta"
      // 提取版本号部分
      const match = stdout.trim().match(/tke-scrcpy\s+(.+)/);
      const version = match ? match[1].trim() : stdout.trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 TKE-Scrcpy 版本失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerSystemHandlers
};
