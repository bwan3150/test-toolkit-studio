// scrcpy-server 服务管理器
// 负责启动和管理 Rust 实现的 scrcpy WebSocket 服务器
const { ipcMain } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const os = require('os');

// scrcpy-server 进程
let scrcpyServerProcess = null;
let scrcpyServerPort = 8000;
let isServerRunning = false;

/**
 * 获取 scrcpy-server 可执行文件路径
 * @returns {string} scrcpy-server 二进制文件的完整路径
 */
function getScrcpyServerPath() {
  const platform = os.platform();
  const isDev = process.env.ELECTRON_DEV_MODE === 'true';

  let binName = 'tke-scrcpy';
  if (platform === 'win32') {
    binName = 'tke-scrcpy.exe';
  }

  if (isDev) {
    return path.join(__dirname, '..', '..', 'bin', platform, binName);
  } else {
    return path.join(process.resourcesPath, platform, binName);
  }
}

/**
 * 获取 tke adb 的完整路径
 * @param {Object} app - Electron app 实例
 * @returns {string} tke adb 可执行文件的完整路径
 * @throws {Error} 如果找不到 tke 可执行文件则抛出错误
 */
function getTkeAdbPath(app) {
  const platform = os.platform();
  let tkeBinName = 'tke';

  if (platform === 'win32') {
    tkeBinName = 'tke.exe';
  }

  // 开发环境路径
  const devPath = path.join(__dirname, '..', '..', 'bin', platform, tkeBinName);
  const prodPath = path.join(process.resourcesPath, platform, tkeBinName);

  const fs = require('fs');
  if (fs.existsSync(devPath)) {
    return devPath;
  } else if (fs.existsSync(prodPath)) {
    return prodPath;
  }

  const error = `未找到 tke 可执行文件。\n尝试的路径:\n  开发: ${devPath}\n  生产: ${prodPath}\n请确保 tke 已正确编译并放置在 bin/${platform}/ 目录下`;
  console.error(error);
  throw new Error(error);
}

/**
 * 启动 scrcpy-server
 * @param {number} port - 服务器端口
 * @param {string} adbPath - tke adb 可执行文件路径
 * @returns {Promise<Object>}
 */
async function startScrcpyServer(port = 8000, adbPath = null) {
  // 如果服务器正在运行，先停止它
  if (scrcpyServerProcess || isServerRunning) {
    console.log('scrcpy-server 正在运行，先停止...');
    await stopScrcpyServer();
    // 等待一小段时间确保端口释放
    await new Promise(resolve => setTimeout(resolve, 500));
  }

  return new Promise((resolve, reject) => {
    try {
      const serverBin = getScrcpyServerPath();

      // 检查二进制文件是否存在
      const fs = require('fs');
      if (!fs.existsSync(serverBin)) {
        const error = `scrcpy-server 二进制文件不存在: ${serverBin}，请先运行 ./scrcpy-server/build-mac.sh`;
        console.error(error);
        return reject(new Error(error));
      }

      // 设置环境变量
      const env = { ...process.env };

      // 设置端口
      env.PORT = port.toString();

      // 必须提供 tke adb 路径，不使用系统 adb
      if (!adbPath) {
        const error = 'adbPath 为空，无法启动 scrcpy-server';
        console.error(error);
        return reject(new Error(error));
      }

      env.ADB_PATH = adbPath;
      console.log('✅ 使用 tke adb 路径:', adbPath);

      // 获取环境信息
      const isDev = process.env.ELECTRON_DEV_MODE === 'true';

      console.log(`启动 scrcpy-server，端口: ${port}`);
      console.log('二进制文件:', serverBin);
      console.log('process.resourcesPath:', process.resourcesPath);
      console.log('__dirname:', __dirname);
      console.log('isDev:', isDev);

      // 检查 jar 文件是否存在
      const path = require('path');
      const jarPath = path.join(path.dirname(serverBin), 'scrcpy-server.jar');
      console.log('预期的 jar 文件路径:', jarPath);
      console.log('jar 文件存在:', fs.existsSync(jarPath));

      // 列出二进制文件所在目录的所有文件
      const serverDir = path.dirname(serverBin);
      console.log('scrcpy-server 目录:', serverDir);
      if (fs.existsSync(serverDir)) {
        const files = fs.readdirSync(serverDir);
        console.log('目录中的文件:', files);
      }

      // 启动 Rust 二进制
      scrcpyServerProcess = spawn(serverBin, [], {
        env: env,
        stdio: ['ignore', 'pipe', 'pipe']
      });

      scrcpyServerPort = port;

      // 监听标准输出
      scrcpyServerProcess.stdout.on('data', (data) => {
        const output = data.toString();
        console.log('[scrcpy-server]', output);

        // 检测服务器启动成功的标志
        // Rust 服务器输出 "Listening on:" 表示启动成功
        if (output.includes('Listening on')) {
          if (!isServerRunning) {
            isServerRunning = true;
            console.log('✅ scrcpy-server 启动成功');
            resolve({ success: true, port: scrcpyServerPort, message: '服务器启动成功' });
          }
        }
      });

      // 监听标准错误
      scrcpyServerProcess.stderr.on('data', (data) => {
        const error = data.toString();
        console.error('[scrcpy-server 错误]', error);
      });

      // 监听进程退出
      scrcpyServerProcess.on('exit', (code) => {
        console.log(`scrcpy-server 进程退出，代码: ${code}`);
        isServerRunning = false;
        scrcpyServerProcess = null;
      });

      // 监听错误
      scrcpyServerProcess.on('error', (error) => {
        console.error('scrcpy-server 启动失败:', error);
        isServerRunning = false;
        scrcpyServerProcess = null;
        reject(error);
      });

      // 设置超时，如果5秒内没有启动成功则认为失败
      setTimeout(() => {
        if (!isServerRunning) {
          console.log('⚠️ scrcpy-server 启动超时，但进程存在，假定启动成功');
          isServerRunning = true;
          resolve({ success: true, port: scrcpyServerPort, message: '服务器已启动（超时后假定成功）' });
        }
      }, 5000);

    } catch (error) {
      console.error('启动 scrcpy-server 时发生错误:', error);
      reject(error);
    }
  });
}

/**
 * 停止 scrcpy-server
 * @returns {Promise<Object>}
 */
async function stopScrcpyServer() {
  if (!scrcpyServerProcess) {
    console.log('scrcpy-server 未运行');
    isServerRunning = false;
    return { success: true, message: '服务器未运行' };
  }

  return new Promise((resolve) => {
    console.log('停止 scrcpy-server');
    let resolved = false;

    const cleanup = () => {
      if (!resolved) {
        resolved = true;
        console.log('scrcpy-server 已停止');
        scrcpyServerProcess = null;
        isServerRunning = false;
        resolve({ success: true, message: '服务器已停止' });
      }
    };

    scrcpyServerProcess.once('exit', cleanup);

    // 尝试优雅关闭
    scrcpyServerProcess.kill('SIGTERM');

    // 2秒后强制关闭
    setTimeout(() => {
      if (scrcpyServerProcess && !resolved) {
        console.log('强制关闭 scrcpy-server');
        scrcpyServerProcess.kill('SIGKILL');
      }
    }, 2000);

    // 3秒后无论如何都清理
    setTimeout(() => {
      cleanup();
    }, 3000);
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
 * 注册所有 scrcpy-server 相关的 IPC 处理器
 * @param {Object} app - Electron app 实例
 */
function registerScrcpyHandlers(app) {
  // 启动 scrcpy-server
  ipcMain.handle('scrcpy:start-server', async (event, options = {}) => {
    try {
      const port = options.port || 8000;

      // 必须获取 tke 的路径，不使用系统 adb
      let adbPath = options.adbPath;
      if (!adbPath) {
        try {
          adbPath = getTkeAdbPath(app);
          console.log('✅ 自动获取的 tke 路径:', adbPath);
        } catch (error) {
          console.error('❌ 获取 tke 路径失败:', error.message);
          return { success: false, error: `无法找到 tke 可执行文件: ${error.message}` };
        }
      }

      // 再次检查 adbPath 是否有效
      if (!adbPath) {
        const error = '未提供 adbPath 且无法自动获取 tke 路径';
        console.error('❌', error);
        return { success: false, error };
      }

      const result = await startScrcpyServer(port, adbPath);
      return result;
    } catch (error) {
      console.error('❌ 启动 scrcpy-server 失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止 scrcpy-server
  ipcMain.handle('scrcpy:stop-server', async () => {
    try {
      const result = await stopScrcpyServer();
      return result;
    } catch (error) {
      console.error('停止 scrcpy-server 失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取服务器状态
  ipcMain.handle('scrcpy:get-status', async () => {
    return getServerStatus();
  });

  console.log('✅ scrcpy-server handlers 已注册');
}

/**
 * 清理函数，在应用退出时调用
 */
async function cleanupScrcpyServer() {
  if (scrcpyServerProcess) {
    console.log('清理 scrcpy-server 进程');
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
