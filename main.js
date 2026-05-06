// Test Toolkit Studio - 主进程
// 必须在最开始初始化 Sentry

// 检测是否为开发环境
const isDev = process.env.ELECTRON_DEV_MODE === 'true';
const environment = isDev ? 'development' : 'production';

const Sentry = require("@sentry/electron/main");

// 只在生产环境启用 Sentry
if (!isDev) {
  Sentry.init({
    dsn: "https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488",
    environment: environment,
    release: require('./package.json').version,
    // 启用日志功能
    enableLogs: true,
    // 设置采样率
    tracesSampleRate: 1.0,
    // 过滤日志：只发送 warn 和 error 到 Logs
    beforeSendLog: (log) => {
      // 过滤掉 info 和 debug 级别的日志
      if (log.level === 'info' || log.level === 'debug' || log.level === 'trace') {
        return null;
      }
      return log;
    },
  });
  console.log('✅ Sentry 已启用 (生产环境)');
} else {
  console.log('ℹ️  Sentry 已禁用 (开发环境)');
}

const { app, BrowserWindow, ipcMain, dialog, shell } = require('electron');
const path = require('path');
const fs = require('fs');
const { exec, spawn } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// 导入处理器模块
// TKE 集成模块 - 所有与 TKE（Toolkit Engine）交互的底层功能
const { registerAdbHandlers, getTkePath, buildTkeAdbCommand } = require('./handlers/tke-integration/adb-handlers');
const { registerAaptHandlers } = require('./handlers/tke-integration/aapt-handlers');
const { registerLogcatHandlers } = require('./handlers/tke-integration/logcat-handlers');
const { registerIosHandlers, cleanupIosProcesses } = require('./handlers/tke-integration/ios-handlers');
const { registerDeviceHandlers } = require('./handlers/tke-integration/device-handlers');
const { registerControllerHandlers } = require('./handlers/tke-integration/controller-handlers');
const { registerFetcherHandlers } = require('./handlers/tke-integration/fetcher-handlers');
const { registerRecognizerHandlers } = require('./handlers/tke-integration/recognizer-handlers');
const { registerRunnerHandlers } = require('./handlers/tke-integration/runner-handlers');
const { registerOcrHandlers } = require('./handlers/tke-integration/ocr-handlers');
const { registerFileHandlers } = require('./handlers/tke-integration/file-handlers');
const { registerAppHandlers } = require('./handlers/tke-integration/app-handlers');
const { registerTkeDeviceHandlers } = require('./handlers/tke-integration/tke-device-handlers');
const { registerFfmpegHandlers } = require('./handlers/dependencies/ffmpeg-handler');

// Electron 核心模块 - Electron 应用的核心功能
const { registerWindowHandlers } = require('./handlers/electron-core/window-handlers');
const { registerStoreHandlers } = require('./handlers/electron-core/store-handlers');
const { registerLogHandlers } = require('./handlers/electron-core/log-handlers');
const { registerFilesystemHandlers } = require('./handlers/electron-core/filesystem-handlers');
const { registerSystemHandlers } = require('./handlers/electron-core/system-handlers');

// 项目管理模块
const { registerProjectHandlers } = require('./handlers/project/project-handlers');

// 数据库模块
const { registerDatabaseHandlers, closeAllDatabases } = require('./handlers/database');

// 属性管理模块 - 用户配置数据管理
const { registerDeviceConfigHandlers } = require('./handlers/property-manage/device-config-handlers');

// API 代理模块 - 与外部 API 服务交互
const { registerAuthHandlers, initializeSentryUser } = require('./handlers/api-proxy/toolkit-gateway');
const { registerBugAnalysisProxyHandlers } = require('./handlers/api-proxy/bug-analysis');
const { registerReleaseNotesHandlers } = require('./handlers/api-proxy/release-notes-handler');

// 自动更新模块
const { initAutoUpdater, registerUpdateHandlers } = require('./handlers/updater/auto-updater');

// LocalSend 模块
const { registerLocalSendHandlers } = require('./handlers/localsend/localsend-handler');

// ws-scrcpy 模块
const { registerScrcpyHandlers, cleanupScrcpyServer } = require('./handlers/ws-scrcpy/scrcpy-server-handler');
const { registerStreamCaptureHandlers } = require('./handlers/ws-scrcpy/stream-capture-handler');

// 全局变量
let mainWindow;

// 日志辅助函数 - 使用 Sentry 官方日志 API
const logger = {
  log: (message, context = {}) => {
    console.log(message);
    if (!isDev && Sentry.addBreadcrumb) {
      // info/log 只作为 breadcrumb，不单独发送
      Sentry.addBreadcrumb({
        message,
        level: 'info',
        data: context,
      });
    }
  },
  info: (message, context = {}) => {
    console.log(message);
    if (!isDev && Sentry.addBreadcrumb) {
      // info 只作为 breadcrumb，不单独发送
      Sentry.addBreadcrumb({
        message,
        level: 'info',
        data: context,
      });
    }
  },
  warn: (message, context = {}) => {
    console.warn(message);
    if (!isDev && Sentry.logger) {
      // warn 发送到 Logs
      Sentry.logger.warn(message, context);
    }
  },
  error: (message, error = null, context = {}) => {
    console.error(message, error);
    if (!isDev) {
      if (error instanceof Error && Sentry.captureException) {
        // 对于 Error 对象，使用 captureException 以获得完整堆栈
        Sentry.captureException(error, {
          contexts: { custom: { message, ...context } },
        });
      } else if (Sentry.logger) {
        // 纯文本错误发送到 Logs
        Sentry.logger.error(message, context);
      }
    }
  },
  debug: (message, context = {}) => {
    console.debug(message);
    if (!isDev && Sentry.addBreadcrumb) {
      // debug 只作为 breadcrumb，不单独发送
      Sentry.addBreadcrumb({
        message,
        level: 'debug',
        data: context,
      });
    }
  },
};

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
  // 但允许自定义右键菜单（通过检查目标元素）
  mainWindow.webContents.on('context-menu', (event, params) => {
    // 允许file-explorer的文件项右键菜单
    const allowedClasses = ['file-item', 'file-item-name', 'file-column'];
    const targetClass = params.selectionText || '';

    // 如果不是在允许的元素上右键,则阻止
    event.preventDefault();

    // 注意：自定义右键菜单由renderer进程处理，这里只是防止原生菜单
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
async function cleanupProcesses() {
  // 清理 iOS 相关进程
  try {
    cleanupIosProcesses();
  } catch (e) {
    logger.error('清理iOS进程失败', e);
  }

  // 清理 ws-scrcpy 服务器进程
  try {
    await cleanupScrcpyServer();
  } catch (e) {
    logger.error('清理ws-scrcpy进程失败', e);
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

    console.log('注册TKE File处理器...');
    registerFileHandlers(app);

    console.log('注册TKE App处理器...');
    registerAppHandlers(app);

    console.log('注册TKE Device处理器...');
    registerTkeDeviceHandlers(app);

    console.log('注册FFmpeg处理器...');
    registerFfmpegHandlers(app);

    // 项目管理模块
    console.log('注册项目处理器...');
    registerProjectHandlers();

    // 数据库模块
    console.log('注册数据库处理器...');
    registerDatabaseHandlers();

    // 属性管理模块
    console.log('注册设备配置处理器...');
    registerDeviceConfigHandlers(app);

    // API 代理模块
    console.log('注册认证处理器...');
    registerAuthHandlers();

    console.log('注册Bug Analysis API代理处理器...');
    registerBugAnalysisProxyHandlers();

    console.log('注册Release Notes处理器...');
    registerReleaseNotesHandlers();

    // 自动更新模块
    console.log('注册自动更新处理器...');
    registerUpdateHandlers();

    // LocalSend 模块
    console.log('注册LocalSend处理器...');
    registerLocalSendHandlers(mainWindow);

    // ws-scrcpy 模块
    console.log('注册ws-scrcpy处理器...');
    registerScrcpyHandlers(app);
    registerStreamCaptureHandlers(app);

    // 注册其他IPC处理器（scrcpy, STB, screenshot等）
    registerOtherHandlers();

    logger.log('所有IPC处理器注册完成');
  } catch (error) {
    logger.error('注册IPC处理器失败', error);
  }
}

// 初始化应用
app.whenReady().then(() => {
  createWindow();

  // 注册IPC处理器
  registerAllHandlers();

  // 初始化自动更新（窗口创建后）
  initAutoUpdater(mainWindow);

  // 初始化环境
  initializeEnvironment();

  // 初始化 Sentry 用户上下文（如果已登录）
  initializeSentryUser().catch(error => {
    logger.error('初始化 Sentry 用户上下文失败', error);
  });
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




  // 打开外部链接
  ipcMain.handle('open-external', async (event, url) => {
    try {
      await shell.openExternal(url);
      return { success: true };
    } catch (error) {
      logger.error('打开外部链接失败', error, { url });
      return { success: false, error: error.message };
    }
  });

  // 获取系统临时目录
  ipcMain.handle('get-temp-dir', async () => {
    const os = require('os');
    const path = require('path');

    // 创建应用专用的临时目录
    const appTempDir = path.join(os.tmpdir(), 'toolkit-app-media-preview');

    if (!fs.existsSync(appTempDir)) {
      fs.mkdirSync(appTempDir, { recursive: true });
    }

    return appTempDir;
  });

  // 启动文件拖拽到外部
  ipcMain.handle('start-drag', async (event, { filePath, iconPath }) => {
    try {
      // 检查文件是否存在
      if (!fs.existsSync(filePath)) {
        throw new Error('文件不存在: ' + filePath);
      }

      // 启动拖拽
      event.sender.startDrag({
        file: filePath,
        icon: iconPath || filePath // 使用文件本身或指定的图标
      });

      logger.log('拖拽已启动: ' + filePath);
      return { success: true };
    } catch (error) {
      logger.error('启动拖拽失败', error, { filePath, iconPath });
      return { success: false, error: error.message };
    }
  });

}

// 初始化环境
function initializeEnvironment() {
  // 输出TKE路径信息
  const tkePath = getTkePath(app);
  logger.log('TKE路径: ' + tkePath);

  // 检查TKE是否存在
  const tkeExists = fs.existsSync(tkePath);
  logger.log('TKE存在: ' + tkeExists);

  // 启动ADB服务器（通过TKE内置的ADB）
  if (tkeExists) {
    const startServerCommand = buildTkeAdbCommand(tkePath, null, ['start-server']);
    exec(startServerCommand, (error, stdout, stderr) => {
      if (error) {
        logger.error('启动ADB服务器失败', error);
      } else {
        logger.log('ADB服务器已启动');
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
  closeAllDatabases();
});

// 导出主窗口引用（供其他模块使用）
module.exports = {
  getMainWindow: () => mainWindow
};
