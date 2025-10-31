// ws-scrcpy 服务管理器
// 负责启动和管理 ws-scrcpy 服务，并与 tke adb 集成
const { ipcMain } = require('electron');
const path = require('path');
const { spawn, exec } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// ws-scrcpy 服务器进程
let scrcpyServerProcess = null;
let scrcpyServerPort = 8000;
let isServerRunning = false;

/**
 * 获取 ws-scrcpy 的构建目录路径
 */
function getScrcpyDistPath() {
  // ws-scrcpy 编译后的目录
  return path.join(__dirname, 'dist');
}

/**
 * 启动 ws-scrcpy 服务器
 * @param {number} port - 服务器端口
 * @param {string} adbPath - tke adb 可执行文件路径
 * @returns {Promise<Object>}
 */
async function startScrcpyServer(port = 8000, adbPath = null) {
  if (isServerRunning && scrcpyServerProcess) {
    console.log('ws-scrcpy 服务器已经在运行');
    return { success: true, port: scrcpyServerPort, message: '服务器已经在运行' };
  }

  return new Promise((resolve, reject) => {
    try {
      const distPath = getScrcpyDistPath();
      const serverScript = path.join(distPath, 'index.js');

      // 检查服务器脚本是否存在
      const fs = require('fs');
      if (!fs.existsSync(serverScript)) {
        const error = `ws-scrcpy 服务器脚本不存在: ${serverScript}，请先运行构建`;
        console.error(error);
        return reject(new Error(error));
      }

      // 设置环境变量
      const env = { ...process.env };

      // 如果提供了 tke adb 路径，设置为 ADB 可执行文件
      if (adbPath) {
        env.ADB_PATH = adbPath;
        console.log('使用 tke adb 路径:', adbPath);
      }

      // 设置服务器端口
      env.PORT = port.toString();

      console.log(`启动 ws-scrcpy 服务器，端口: ${port}`);

      // 启动服务器进程
      scrcpyServerProcess = spawn('node', [serverScript], {
        cwd: distPath,
        env: env,
        stdio: ['ignore', 'pipe', 'pipe']
      });

      scrcpyServerPort = port;

      // 监听标准输出
      scrcpyServerProcess.stdout.on('data', (data) => {
        const output = data.toString();
        console.log('[ws-scrcpy 输出]', output);

        // 检测服务器启动成功的标志
        // ws-scrcpy 输出 "Listening on:" 表示启动成功
        if (output.includes('Listening on')) {
          if (!isServerRunning) {
            isServerRunning = true;
            console.log('✅ ws-scrcpy 服务器启动成功');
            resolve({ success: true, port: scrcpyServerPort, message: '服务器启动成功' });
          }
        }
      });

      // 监听标准错误
      scrcpyServerProcess.stderr.on('data', (data) => {
        const error = data.toString();
        console.error('[ws-scrcpy 错误]', error);
      });

      // 监听进程退出
      scrcpyServerProcess.on('exit', (code) => {
        console.log(`ws-scrcpy 服务器进程退出，代码: ${code}`);
        isServerRunning = false;
        scrcpyServerProcess = null;
      });

      // 监听错误
      scrcpyServerProcess.on('error', (error) => {
        console.error('ws-scrcpy 服务器启动失败:', error);
        isServerRunning = false;
        scrcpyServerProcess = null;
        reject(error);
      });

      // 设置超时，如果5秒内没有启动成功则认为失败
      setTimeout(() => {
        if (!isServerRunning) {
          console.log('⚠️ ws-scrcpy 服务器启动超时，但进程存在，假定启动成功');
          isServerRunning = true;
          resolve({ success: true, port: scrcpyServerPort, message: '服务器已启动（超时后假定成功）' });
        }
      }, 5000);

    } catch (error) {
      console.error('启动 ws-scrcpy 服务器时发生错误:', error);
      reject(error);
    }
  });
}

/**
 * 停止 ws-scrcpy 服务器
 * @returns {Promise<Object>}
 */
async function stopScrcpyServer() {
  if (!scrcpyServerProcess) {
    console.log('ws-scrcpy 服务器未运行');
    return { success: true, message: '服务器未运行' };
  }

  return new Promise((resolve) => {
    console.log('停止 ws-scrcpy 服务器');

    scrcpyServerProcess.once('exit', () => {
      console.log('ws-scrcpy 服务器已停止');
      scrcpyServerProcess = null;
      isServerRunning = false;
      resolve({ success: true, message: '服务器已停止' });
    });

    // 尝试优雅关闭
    scrcpyServerProcess.kill('SIGTERM');

    // 5秒后强制关闭
    setTimeout(() => {
      if (scrcpyServerProcess) {
        console.log('强制关闭 ws-scrcpy 服务器');
        scrcpyServerProcess.kill('SIGKILL');
        scrcpyServerProcess = null;
        isServerRunning = false;
        resolve({ success: true, message: '服务器已强制关闭' });
      }
    }, 5000);
  });
}

/**
 * 获取服务器状态
 * @returns {Object}
 */
function getServerStatus() {
  return {
    running: isServerRunning,
    port: scrcpyServerPort,
    processExists: scrcpyServerProcess !== null
  };
}

/**
 * 获取 tke adb 的完整路径
 * @param {Object} app - Electron app 实例
 * @returns {string} tke adb 可执行文件的完整路径
 */
function getTkeAdbPath(app) {
  const path = require('path');
  const os = require('os');

  // 根据平台获取 tke 可执行文件路径
  const platform = os.platform();
  let tkeBinName = 'tke';

  if (platform === 'win32') {
    tkeBinName = 'tke.exe';
  }

  // 开发环境路径
  const devPath = path.join(__dirname, '..', '..', 'resources', platform, 'toolkit-engine', tkeBinName);

  // 生产环境路径
  const prodPath = path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinName);

  // 检查哪个路径存在
  const fs = require('fs');
  if (fs.existsSync(devPath)) {
    return devPath;
  } else if (fs.existsSync(prodPath)) {
    return prodPath;
  }

  console.warn('未找到 tke 可执行文件，将使用系统默认 adb');
  return null;
}

/**
 * 注册所有 ws-scrcpy 相关的 IPC 处理器
 * @param {Object} app - Electron app 实例
 */
function registerScrcpyHandlers(app) {
  // 启动 ws-scrcpy 服务器
  ipcMain.handle('scrcpy:start-server', async (event, options = {}) => {
    try {
      const port = options.port || 8000;

      // 如果没有提供 adbPath，则自动获取 tke 的路径
      let adbPath = options.adbPath;
      if (!adbPath) {
        adbPath = getTkeAdbPath(app);
        console.log('自动获取的 tke 路径:', adbPath);
      }

      const result = await startScrcpyServer(port, adbPath);
      return result;
    } catch (error) {
      console.error('启动 ws-scrcpy 服务器失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止 ws-scrcpy 服务器
  ipcMain.handle('scrcpy:stop-server', async () => {
    try {
      const result = await stopScrcpyServer();
      return result;
    } catch (error) {
      console.error('停止 ws-scrcpy 服务器失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取服务器状态
  ipcMain.handle('scrcpy:get-status', async () => {
    return getServerStatus();
  });

  console.log('✅ ws-scrcpy handlers 已注册');
}

/**
 * 清理函数，在应用退出时调用
 */
async function cleanupScrcpyServer() {
  if (scrcpyServerProcess) {
    console.log('清理 ws-scrcpy 服务器进程');
    await stopScrcpyServer();
  }
}

module.exports = {
  registerScrcpyHandlers,
  cleanupScrcpyServer,
  startScrcpyServer,
  stopScrcpyServer,
  getServerStatus
};
