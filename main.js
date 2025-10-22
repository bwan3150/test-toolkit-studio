// Test Toolkit Studio - 主进程
const { app, BrowserWindow, ipcMain, Menu, Tray, dialog, shell } = require('electron');
const path = require('path');
const fs = require('fs');
const { exec, spawn } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// 导入处理器模块
// TKE 集成模块 - 所有与 TKE（Toolkit Engine）交互的底层功能
const { registerAdbHandlers, getBuiltInScrcpyPath, getBuiltInStbPath, getTkePath, buildTkeAdbCommand } = require('./handlers/tke-integration/adb-handlers');
const { registerAaptHandlers } = require('./handlers/tke-integration/aapt-handlers');
const { registerLogcatHandlers } = require('./handlers/tke-integration/logcat-handlers');
const { registerIosHandlers, cleanupIosProcesses } = require('./handlers/tke-integration/ios-handlers');
const { registerDeviceHandlers } = require('./handlers/tke-integration/device-handlers');
const { registerControllerHandlers } = require('./handlers/tke-integration/controller-handlers');
const { registerFetcherHandlers } = require('./handlers/tke-integration/fetcher-handlers');
const { registerRecognizerHandlers } = require('./handlers/tke-integration/recognizer-handlers');
const { registerRunnerHandlers } = require('./handlers/tke-integration/runner-handlers');
const { registerOcrHandlers } = require('./handlers/tke-integration/ocr-handlers');

// Electron 核心模块 - Electron 应用的核心功能
const { registerWindowHandlers } = require('./handlers/electron-core/window-handlers');
const { registerStoreHandlers } = require('./handlers/electron-core/store-handlers');
const { registerLogHandlers } = require('./handlers/electron-core/log-handlers');
const { registerFilesystemHandlers } = require('./handlers/electron-core/filesystem-handlers');
const { registerSystemHandlers } = require('./handlers/electron-core/system-handlers');

// 项目管理模块
const { registerProjectHandlers } = require('./handlers/project/project-handlers');

// API 代理模块 - 与外部 API 服务交互
const { registerAuthHandlers } = require('./handlers/api-proxy/toolkit-gateway');
const { registerBugAnalysisProxyHandlers } = require('./handlers/api-proxy/bug-analysis');

// 自动更新模块
const { initAutoUpdater, registerUpdateHandlers } = require('./handlers/updater/auto-updater');

// 全局变量
let mainWindow;
let scrcpyProcess = null;
let stbProcess = null;
let tray = null;

// 创建主窗口
function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 1200,
    minHeight: 800,
    frame: false,  // 所有平台都使用自定义标题栏
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
      webSecurity: false,
      devTools: false,  // 彻底禁用开发者工具
      allowRunningInsecureContent: true  // 允许不安全的内容（HTTP请求）
    },
    icon: path.join(__dirname, 'assets', 'icon.png')
  });

  // 禁用CSP（Content Security Policy）以允许外部API请求
  mainWindow.webContents.session.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': ['default-src * \'unsafe-inline\' \'unsafe-eval\' data: blob:;']
      }
    });
  });

  // 设置代理（如果需要的话）- 确保可以访问外部API
  // 允许所有HTTP/HTTPS请求
  mainWindow.webContents.session.webRequest.onBeforeSendHeaders(
    { urls: ['http://*/*', 'https://*/*'] },
    (details, callback) => {
      callback({ requestHeaders: details.requestHeaders });
    }
  );

  // 加载主页面
  mainWindow.loadFile('renderer/html/index.html');

  // 监听窗口关闭事件
  mainWindow.on('closed', () => {
    mainWindow = null;
    cleanupProcesses();
  });

  // 监听窗口最大化状态变化
  mainWindow.on('maximize', () => {
    mainWindow.webContents.send('window-maximized', true);
  });

  mainWindow.on('unmaximize', () => {
    mainWindow.webContents.send('window-maximized', false);
  });

  // 多重防护：禁用开发者工具的所有访问方式
  mainWindow.webContents.on('before-input-event', (event, input) => {
    // 禁用所有可能打开开发者工具的快捷键
    const key = input.key.toLowerCase();
    
    // Windows/Linux: Ctrl+Shift+I
    if (input.control && input.shift && key === 'i') {
      event.preventDefault();
      console.log('Dev tools access attempt blocked: Ctrl+Shift+I');
      return;
    }
    
    // macOS: Cmd+Option+I (meta + alt + i)
    if (input.meta && input.alt && key === 'i') {
      event.preventDefault();
      console.log('Dev tools access attempt blocked: Cmd+Option+I');
      return;
    }
    
    // F12 key
    if (key === 'f12') {
      event.preventDefault();
      console.log('Dev tools access attempt blocked: F12');
      return;
    }
    
    // Additional combinations that might open dev tools
    if ((input.control && input.shift && key === 'j') || // Ctrl+Shift+J (Console)
        (input.meta && input.alt && key === 'j') || // Cmd+Option+J (Console)
        (input.control && input.shift && key === 'c') || // Ctrl+Shift+C (Element inspector)
        (input.meta && input.shift && key === 'c')) { // Cmd+Shift+C (Element inspector)
      event.preventDefault();
      console.log(`Dev tools access attempt blocked: ${input.meta ? 'Cmd' : 'Ctrl'}+${input.shift ? 'Shift' : ''}+${input.alt ? 'Alt' : ''}+${key.toUpperCase()}`);
      return;
    }
  });
  
  // 禁用右键菜单（防止通过右键菜单打开开发者工具）
  mainWindow.webContents.on('context-menu', (event) => {
    event.preventDefault();
    console.log('Right-click context menu blocked');
  });
  
  // 最后防线：即使开发者工具被意外打开，也立即关闭
  mainWindow.webContents.on('devtools-opened', () => {
    console.log('Dev tools opened, force closing...');
    mainWindow.webContents.closeDevTools();
  });
  
  // 监听并阻止开发者工具相关的事件
  mainWindow.webContents.on('devtools-focus', () => {
    console.log('Dev tools focus blocked');
    mainWindow.focus();
  });
}

// 清理子进程
function cleanupProcesses() {
  // 停止scrcpy进程
  if (scrcpyProcess) {
    try {
      if (process.platform === 'win32') {
        exec(`taskkill /F /T /PID ${scrcpyProcess.pid}`);
      } else {
        scrcpyProcess.kill('SIGTERM');
      }
    } catch (e) {
      console.error('停止scrcpy进程失败:', e);
    }
    scrcpyProcess = null;
  }

  // 停止STB进程
  if (stbProcess) {
    try {
      if (process.platform === 'win32') {
        exec(`taskkill /F /T /PID ${stbProcess.pid}`);
      } else {
        stbProcess.kill('SIGTERM');
      }
    } catch (e) {
      console.error('停止STB进程失败:', e);
    }
    stbProcess = null;
  }
  
  // 清理 iOS 相关进程
  try {
    cleanupIosProcesses();
  } catch (e) {
    console.error('清理iOS进程失败:', e);
  }
}

// 创建系统托盘
function createTray() {
  const iconPath = path.join(__dirname, 'assets', 'tray-icon.png');
  
  // 检查图标文件是否存在
  if (!fs.existsSync(iconPath)) {
    console.log('托盘图标不存在，跳过托盘创建:', iconPath);
    return;
  }
  
  try {
    tray = new Tray(iconPath);
  
  const contextMenu = Menu.buildFromTemplate([
    {
      label: '显示主窗口',
      click: () => {
        if (mainWindow) {
          mainWindow.show();
          mainWindow.focus();
        }
      }
    },
    {
      label: '退出',
      click: () => {
        app.quit();
      }
    }
  ]);
  
  tray.setToolTip('Test Toolkit Studio');
  tray.setContextMenu(contextMenu);
  
  tray.on('click', () => {
    if (mainWindow) {
      if (mainWindow.isVisible()) {
        mainWindow.hide();
      } else {
        mainWindow.show();
        mainWindow.focus();
      }
    }
  });
  } catch (error) {
    console.error('创建托盘失败:', error);
  }
}

// 注册所有IPC处理器
function registerAllHandlers() {
  try {
    console.log('开始注册IPC处理器...');

    // Electron 核心模块
    console.log('注册Store处理器...');
    registerStoreHandlers(app);

    console.log('注册窗口处理器...');
    registerWindowHandlers(mainWindow);

    console.log('注册日志处理器...');
    registerLogHandlers(app);

    console.log('注册文件系统处理器...');
    registerFilesystemHandlers(app);

    console.log('注册系统工具检查处理器...');
    registerSystemHandlers(app);

    // TKE 集成模块
    console.log('注册ADB处理器...');
    registerAdbHandlers(app);

    console.log('注册AAPT处理器...');
    registerAaptHandlers(app);

    console.log('注册Logcat处理器...');
    registerLogcatHandlers(app, mainWindow);

    console.log('注册设备管理处理器...');
    registerDeviceHandlers(app);

    console.log('注册iOS处理器...');
    registerIosHandlers(app);

    console.log('注册TKE Controller处理器...');
    registerControllerHandlers(app);

    console.log('注册TKE Fetcher处理器...');
    registerFetcherHandlers(app);

    console.log('注册TKE Recognizer处理器...');
    registerRecognizerHandlers(app);

    console.log('注册TKE Runner处理器...');
    registerRunnerHandlers(app);

    console.log('注册TKE OCR处理器...');
    registerOcrHandlers(app);

    // 项目管理模块
    console.log('注册项目处理器...');
    registerProjectHandlers();

    // API 代理模块
    console.log('注册认证处理器...');
    registerAuthHandlers();

    console.log('注册Bug Analysis API代理处理器...');
    registerBugAnalysisProxyHandlers();

    // 自动更新模块
    console.log('注册自动更新处理器...');
    registerUpdateHandlers();

    // 注册其他IPC处理器（scrcpy, STB, screenshot等）
    registerOtherHandlers();

    console.log('所有IPC处理器注册完成');
  } catch (error) {
    console.error('注册IPC处理器失败:', error);
  }
}

// 初始化应用
app.whenReady().then(() => {
  createWindow();
  createTray();

  // 注册IPC处理器
  registerAllHandlers();

  // 初始化自动更新（窗口创建后）
  initAutoUpdater(mainWindow);

  // 初始化环境
  initializeEnvironment();
});

// 注册其他IPC处理器
function registerOtherHandlers() {
  // 处理渲染进程的日志消息（旧版本，保留兼容性）
  ipcMain.on('log-message', (event, message) => {
    console.log('[Renderer Log]:', message);
  });
  
  // 新的渲染进程日志处理器
  ipcMain.on('renderer-log', (event, logData) => {
    const { level, message, timestamp } = logData;
    const levelColors = {
      log: '\x1b[0m',    // 默认
      info: '\x1b[36m',   // 青色
      warn: '\x1b[33m',   // 黄色
      error: '\x1b[31m',  // 红色
      debug: '\x1b[35m'   // 紫色
    };
    const color = levelColors[level] || levelColors.log;
    const reset = '\x1b[0m';

    console.log(`${color}[Renderer ${level.toUpperCase()}] ${timestamp}:${reset} ${message}`);
  });

  // 获取保存的设备列表
  ipcMain.handle('get-saved-devices', async () => {
    try {
      const savedDevicesPath = path.join(app.getPath('userData'), 'saved-devices.json');
      if (fs.existsSync(savedDevicesPath)) {
        const data = fs.readFileSync(savedDevicesPath, 'utf8');
        return { success: true, devices: JSON.parse(data) };
      }
      return { success: true, devices: [] };
    } catch (error) {
      console.error('获取保存的设备失败:', error);
      return { success: false, devices: [], error: error.message };
    }
  });

  // 保存设备列表
  ipcMain.handle('save-devices', async (event, devices) => {
    try {
      const savedDevicesPath = path.join(app.getPath('userData'), 'saved-devices.json');
      fs.writeFileSync(savedDevicesPath, JSON.stringify(devices, null, 2));
      return { success: true };
    } catch (error) {
      console.error('保存设备失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 启动scrcpy投屏
  ipcMain.handle('start-scrcpy', async (event, deviceId, options = {}) => {
    try {
      // 如果已有scrcpy进程在运行，先停止它
      if (scrcpyProcess) {
        if (process.platform === 'win32') {
          exec(`taskkill /F /T /PID ${scrcpyProcess.pid}`);
        } else {
          scrcpyProcess.kill('SIGTERM');
        }
        scrcpyProcess = null;
      }

      const scrcpyPath = getBuiltInScrcpyPath(app);
      
      if (!fs.existsSync(scrcpyPath)) {
        return { success: false, error: '内置scrcpy未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 构建scrcpy命令参数
      const args = ['-s', deviceId];
      
      // 添加选项参数
      if (options.maxSize) {
        args.push('--max-size', options.maxSize.toString());
      }
      if (options.bitRate) {
        args.push('--bit-rate', options.bitRate);
      }
      if (options.maxFps) {
        args.push('--max-fps', options.maxFps.toString());
      }
      if (options.stayAwake) {
        args.push('--stay-awake');
      }
      if (options.turnScreenOff) {
        args.push('--turn-screen-off');
      }
      if (options.showTouches) {
        args.push('--show-touches');
      }
      if (options.fullscreen) {
        args.push('--fullscreen');
      }
      if (options.alwaysOnTop) {
        args.push('--always-on-top');
      }
      if (options.noControl) {
        args.push('--no-control');
      }
      if (options.noAudio) {
        args.push('--no-audio');
      }

      console.log('启动scrcpy:', scrcpyPath, args.join(' '));

      // 启动scrcpy进程
      scrcpyProcess = spawn(scrcpyPath, args, {
        detached: false,
        stdio: ['ignore', 'pipe', 'pipe']
      });

      scrcpyProcess.stdout.on('data', (data) => {
        console.log('scrcpy输出:', data.toString());
      });

      scrcpyProcess.stderr.on('data', (data) => {
        console.error('scrcpy错误:', data.toString());
      });

      scrcpyProcess.on('close', (code) => {
        console.log('scrcpy进程退出，代码:', code);
        scrcpyProcess = null;
        mainWindow?.webContents.send('scrcpy-closed', { deviceId, code });
      });

      // 等待一小段时间确认进程启动
      await new Promise(resolve => setTimeout(resolve, 1000));

      if (scrcpyProcess && !scrcpyProcess.killed) {
        return { 
          success: true, 
          message: '投屏已启动',
          pid: scrcpyProcess.pid
        };
      } else {
        return { 
          success: false, 
          error: '投屏启动失败'
        };
      }

    } catch (error) {
      console.error('启动scrcpy失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止scrcpy投屏
  ipcMain.handle('stop-scrcpy', async () => {
    try {
      if (scrcpyProcess) {
        if (process.platform === 'win32') {
          exec(`taskkill /F /T /PID ${scrcpyProcess.pid}`);
        } else {
          scrcpyProcess.kill('SIGTERM');
        }
        scrcpyProcess = null;
        return { success: true, message: '投屏已停止' };
      }
      return { success: true, message: '没有运行中的投屏' };
    } catch (error) {
      console.error('停止scrcpy失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 启动STB远程控制
  ipcMain.handle('start-stb', async (event, deviceId) => {
    try {
      // 如果已有STB进程在运行，先停止它
      if (stbProcess) {
        if (process.platform === 'win32') {
          exec(`taskkill /F /T /PID ${stbProcess.pid}`);
        } else {
          stbProcess.kill('SIGTERM');
        }
        stbProcess = null;
      }

      const stbPath = getBuiltInStbPath(app);
      
      if (!fs.existsSync(stbPath)) {
        return { success: false, error: '内置STB未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      console.log('启动STB:', stbPath, deviceId);

      // 启动STB进程
      stbProcess = spawn(stbPath, [deviceId], {
        detached: false,
        stdio: ['ignore', 'pipe', 'pipe']
      });

      stbProcess.stdout.on('data', (data) => {
        console.log('STB输出:', data.toString());
        const output = data.toString();
        if (output.includes('http://')) {
          const urlMatch = output.match(/http:\/\/[^\s]+/);
          if (urlMatch) {
            mainWindow?.webContents.send('stb-url', { 
              deviceId, 
              url: urlMatch[0] 
            });
          }
        }
      });

      stbProcess.stderr.on('data', (data) => {
        console.error('STB错误:', data.toString());
      });

      stbProcess.on('close', (code) => {
        console.log('STB进程退出，代码:', code);
        stbProcess = null;
        mainWindow?.webContents.send('stb-closed', { deviceId, code });
      });

      // 等待STB启动并返回URL
      return new Promise((resolve) => {
        let timeout = setTimeout(() => {
          resolve({ 
            success: false, 
            error: 'STB启动超时'
          });
        }, 10000);

        const checkOutput = (data) => {
          const output = data.toString();
          if (output.includes('http://')) {
            clearTimeout(timeout);
            const urlMatch = output.match(/http:\/\/[^\s]+/);
            if (urlMatch) {
              resolve({ 
                success: true, 
                message: 'STB已启动',
                url: urlMatch[0],
                pid: stbProcess.pid
              });
            }
          }
        };

        if (stbProcess) {
          stbProcess.stdout.once('data', checkOutput);
        }
      });

    } catch (error) {
      console.error('启动STB失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止STB
  ipcMain.handle('stop-stb', async () => {
    try {
      if (stbProcess) {
        if (process.platform === 'win32') {
          exec(`taskkill /F /T /PID ${stbProcess.pid}`);
        } else {
          stbProcess.kill('SIGTERM');
        }
        stbProcess = null;
        return { success: true, message: 'STB已停止' };
      }
      return { success: true, message: '没有运行中的STB' };
    } catch (error) {
      console.error('停止STB失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 截图功能
  ipcMain.handle('take-screenshot', async (event, deviceId) => {
    try {
      const tkePath = getTkePath(app);
      
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'Toolkit Engine未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 创建截图保存目录
      const screenshotDir = path.join(app.getPath('userData'), 'screenshots');
      if (!fs.existsSync(screenshotDir)) {
        fs.mkdirSync(screenshotDir, { recursive: true });
      }

      // 生成文件名
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const filename = `screenshot_${deviceId}_${timestamp}.png`;
      const localPath = path.join(screenshotDir, filename);
      const remotePath = '/sdcard/screenshot.png';

      // 在设备上截图
      const screenshotCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', 'screencap', '-p', remotePath]);
      await execPromise(screenshotCommand);

      // 拉取截图到本地
      const pullCommand = buildTkeAdbCommand(tkePath, deviceId, ['pull', remotePath, `"${localPath}"`]);
      await execPromise(pullCommand);

      // 删除设备上的截图
      const deleteCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', 'rm', remotePath]);
      await execPromise(deleteCommand);

      return { 
        success: true, 
        message: '截图成功',
        path: localPath,
        filename: filename
      };

    } catch (error) {
      console.error('截图失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 录屏功能
  ipcMain.handle('start-recording', async (event, deviceId, duration = 180) => {
    try {
      const tkePath = getTkePath(app);
      
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'Toolkit Engine未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 创建录屏保存目录
      const recordingDir = path.join(app.getPath('userData'), 'recordings');
      if (!fs.existsSync(recordingDir)) {
        fs.mkdirSync(recordingDir, { recursive: true });
      }

      // 生成文件名
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const filename = `recording_${deviceId}_${timestamp}.mp4`;
      const localPath = path.join(recordingDir, filename);
      const remotePath = '/sdcard/recording.mp4';

      // 开始录屏
      const recordCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', 'screenrecord', '--time-limit', duration.toString(), remotePath]);
      
      // 异步执行录屏命令
      exec(recordCommand, async (error) => {
        if (!error) {
          // 录屏完成，拉取文件
          try {
            const pullCommand = buildTkeAdbCommand(tkePath, deviceId, ['pull', remotePath, `"${localPath}"`]);
            await execPromise(pullCommand);

            // 删除设备上的录屏文件
            const deleteCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', 'rm', remotePath]);
            await execPromise(deleteCommand);

            // 通知前端录屏完成
            mainWindow?.webContents.send('recording-completed', {
              deviceId,
              path: localPath,
              filename: filename
            });
          } catch (pullError) {
            console.error('拉取录屏文件失败:', pullError);
            mainWindow?.webContents.send('recording-failed', {
              deviceId,
              error: pullError.message
            });
          }
        } else {
          console.error('录屏失败:', error);
          mainWindow?.webContents.send('recording-failed', {
            deviceId,
            error: error.message
          });
        }
      });

      return { 
        success: true, 
        message: `录屏已开始，最长${duration}秒`,
        remotePath: remotePath
      };

    } catch (error) {
      console.error('开始录屏失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止录屏
  ipcMain.handle('stop-recording', async (event, deviceId) => {
    try {
      const tkePath = getTkePath(app);
      
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'Toolkit Engine未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 停止录屏（通过杀死screenrecord进程）
      const stopCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', 'pkill', '-2', 'screenrecord']);
      await execPromise(stopCommand);

      return { 
        success: true, 
        message: '录屏已停止'
      };

    } catch (error) {
      // pkill可能不存在，尝试其他方法
      try {
        const tkePath = getTkePath(app);
        const killCommand = buildTkeAdbCommand(tkePath, deviceId, ['shell', '"ps | grep screenrecord | awk \'{print $2}\' | xargs kill -2"']);
        await execPromise(killCommand);
        return { success: true, message: '录屏已停止' };
      } catch (killError) {
        console.error('停止录屏失败:', killError);
        return { success: false, error: '停止录屏失败，可能已经停止或未在录制' };
      }
    }
  });

  // 打开外部链接
  ipcMain.handle('open-external', async (event, url) => {
    try {
      await shell.openExternal(url);
      return { success: true };
    } catch (error) {
      console.error('打开外部链接失败:', error);
      return { success: false, error: error.message };
    }
  });

}

// 初始化环境
function initializeEnvironment() {
  // 输出各个工具的路径信息
  console.log('TKE路径:', getTkePath(app));
  console.log('Scrcpy路径:', getBuiltInScrcpyPath(app));
  console.log('STB路径:', getBuiltInStbPath(app));
  
  // 检查工具是否存在
  const tkeExists = fs.existsSync(getTkePath(app));
  const scrcpyExists = fs.existsSync(getBuiltInScrcpyPath(app));
  const stbExists = fs.existsSync(getBuiltInStbPath(app));
  
  console.log('TKE存在:', tkeExists);
  console.log('Scrcpy存在:', scrcpyExists);
  console.log('STB存在:', stbExists);
  
  // 启动ADB服务器
  if (tkeExists) {
    const tkePath = getTkePath(app);
    const startServerCommand = buildTkeAdbCommand(tkePath, null, ['start-server']);
    exec(startServerCommand, (error, stdout, stderr) => {
      if (error) {
        console.error('启动ADB服务器失败:', error);
      } else {
        console.log('ADB服务器已启动');
      }
    });
  }
}

// 应用事件处理
app.on('window-all-closed', () => {
  cleanupProcesses();
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  }
});

app.on('before-quit', () => {
  cleanupProcesses();
});

// 导出主窗口引用（供其他模块使用）
module.exports = {
  getMainWindow: () => mainWindow
};