// 系统工具检查相关的IPC处理器
// 负责检查应用版本和内置工具的版本信息
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { promisify } = require('util');
const { exec } = require('child_process');
const execPromise = promisify(exec);

/**
 * 获取二进制文件的路径
 * @param {string} toolName - 工具名称，如 'tke', 'tke-opencv', 'tester-ai'
 * @param {string} subdir - 子目录，如 'toolkit-engine', 'tester-ai'
 * @param {Electron.App} app - Electron app 实例
 * @returns {string} 二进制文件路径
 */
function getBinaryPath(toolName, subdir, app) {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const binaryName = process.platform === 'win32' ? `${toolName}.exe` : toolName;

  // 检查是否在开发模式
  const isDevMode = process.env.ELECTRON_DEV_MODE === 'true';

  if (isDevMode) {
    // 开发模式：从项目根目录的 resources 获取
    const projectRoot = process.env.ELECTRON_PROJECT_ROOT || process.cwd();
    return path.join(projectRoot, 'resources', platform, subdir, binaryName);
  } else if (app.isPackaged) {
    // 打包模式：从 app resources 获取
    return path.join(process.resourcesPath, platform, subdir, binaryName);
  } else {
    // 其他情况（未打包但非开发模式）
    return path.join(__dirname, '..', '..', '..', 'resources', platform, subdir, binaryName);
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
      const tkePath = getBinaryPath('tke', 'toolkit-engine', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" --version`);
      const version = stdout.trim().split('\n')[0].trim();

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
      const tkePath = getBinaryPath('tke', 'toolkit-engine', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" adb --version`);
      const version = stdout.trim().split('\n')[0].trim();

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
      const tkePath = getBinaryPath('tke', 'toolkit-engine', app);

      if (!fs.existsSync(tkePath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${tkePath}" aapt version`);
      const version = stdout.trim().split('\n')[0].trim();

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
      const opencvPath = getBinaryPath('tke-opencv', 'toolkit-engine', app);

      if (!fs.existsSync(opencvPath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${opencvPath}" --version`);
      const version = stdout.trim().split('\n')[0].trim();

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
      const testerAiPath = getBinaryPath('tester-ai', 'tester-ai', app);

      if (!fs.existsSync(testerAiPath)) {
        return { success: false, error: '可执行文件不存在' };
      }

      const { stdout } = await execPromise(`"${testerAiPath}" --version`);
      const version = stdout.trim().split('\n')[0].trim();

      return {
        success: true,
        version: version
      };
    } catch (error) {
      console.error('获取 Tester-AI 版本失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerSystemHandlers
};
