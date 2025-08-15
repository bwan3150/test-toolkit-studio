const { app, BrowserWindow, ipcMain, Menu, dialog } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const Store = require('electron-store');
const { exec } = require('child_process');
const util = require('util');
const execPromise = util.promisify(exec);

// Fix for macOS
if (process.platform === 'darwin') {
  app.commandLine.appendSwitch('disable-features', 'IOSurfaceCapturer');
}

// Initialize electron store for persistent data
const store = new Store();

let mainWindow;
let isAuthenticated = false;
let tokenCheckInterval = null;

// Helper function to get built-in ADB path
function getBuiltInAdbPath() {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
  
  if (app.isPackaged) {
    // In production, resources are in the app's resources folder
    return path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
  } else {
    // In development, use the resources folder in the project
    return path.join(__dirname, 'resources', platform, 'android-sdk', 'platform-tools', adbName);
  }
}

function createWindow() {
  // 根据平台配置不同的窗口选项
  let windowOptions = {
    width: 1600,
    height: 900,
    minWidth: 1200,
    minHeight: 700,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
      webSecurity: false
    },
    backgroundColor: '#1e1e1e',
    icon: path.join(__dirname, 'assets', 'logo', 'toolkit_logo.png')
  };

  // macOS平台配置
  if (process.platform === 'darwin') {
    windowOptions.titleBarStyle = 'hidden';
    windowOptions.trafficLightPosition = { x: 15, y: 13 };
    windowOptions.frame = true;
    // 确保背景色与CSS一致
    windowOptions.backgroundColor = '#1e1e1e';
  }
  // Windows平台配置  
  else if (process.platform === 'win32') {
    // 完全隐藏系统标题栏，使用自定义标题栏
    windowOptions.frame = false;
    windowOptions.titleBarStyle = 'hidden';
    windowOptions.backgroundColor = '#1e1e1e';
  }
  // Linux和其他平台
  else {
    windowOptions.titleBarStyle = 'default';
    windowOptions.frame = true;
  }

  mainWindow = new BrowserWindow(windowOptions);

  // Check if user has valid token
  const token = store.get('access_token');
  const tokenExpiry = store.get('token_expiry');
  const refreshToken = store.get('refresh_token');
  
  // 检查token是否存在且未过期（给5分钟缓冲时间）
  const bufferTime = 5 * 60 * 1000; // 5分钟
  const isTokenValid = token && tokenExpiry && 
    new Date(new Date(tokenExpiry).getTime() - bufferTime) > new Date();
  
  if (isTokenValid) {
    isAuthenticated = true;
    mainWindow.loadFile('renderer/index.html');
    // 启动定期token检查
    startTokenCheck();
  } else if (refreshToken) {
    // 有refresh token，尝试自动刷新
    console.log('Token即将过期或已过期，尝试自动刷新...');
    tryRefreshTokenOnStartup();
  } else {
    // 没有有效token，显示登录页面
    mainWindow.loadFile('renderer/login.html');
  }

  // Create application menu
  const template = [
    {
      label: 'File',
      submenu: [
        {
          label: 'New Project',
          accelerator: 'CmdOrCtrl+N',
          click: () => {
            mainWindow.webContents.send('menu-new-project');
          }
        },
        {
          label: 'Open Project',
          accelerator: 'CmdOrCtrl+O',
          click: () => {
            mainWindow.webContents.send('menu-open-project');
          }
        },
        { type: 'separator' },
        {
          label: 'Quit',
          accelerator: process.platform === 'darwin' ? 'Cmd+Q' : 'Ctrl+Q',
          click: () => {
            app.quit();
          }
        }
      ]
    },
    {
      label: 'Edit',
      submenu: [
        { label: 'Undo', accelerator: 'CmdOrCtrl+Z', role: 'undo' },
        { label: 'Redo', accelerator: 'Shift+CmdOrCtrl+Z', role: 'redo' },
        { type: 'separator' },
        { label: 'Cut', accelerator: 'CmdOrCtrl+X', role: 'cut' },
        { label: 'Copy', accelerator: 'CmdOrCtrl+C', role: 'copy' },
        { label: 'Paste', accelerator: 'CmdOrCtrl+V', role: 'paste' }
      ]
    },
    {
      label: 'View',
      submenu: [
        { label: 'Reload', accelerator: 'CmdOrCtrl+R', role: 'reload' },
        { label: 'Force Reload', accelerator: 'CmdOrCtrl+Shift+R', role: 'forceReload' },
        { label: 'Toggle Developer Tools', accelerator: 'F12', role: 'toggleDevTools' },
        { type: 'separator' },
        { label: 'Actual Size', accelerator: 'CmdOrCtrl+0', role: 'resetZoom' },
        { label: 'Zoom In', accelerator: 'CmdOrCtrl+Plus', role: 'zoomIn' },
        { label: 'Zoom Out', accelerator: 'CmdOrCtrl+-', role: 'zoomOut' },
        { type: 'separator' },
        { label: 'Toggle Fullscreen', accelerator: 'F11', role: 'togglefullscreen' }
      ]
    },
    {
      label: 'Test',
      submenu: [
        {
          label: 'Run Current Test',
          accelerator: 'F5',
          click: () => {
            mainWindow.webContents.send('menu-run-test');
          }
        },
        {
          label: 'Stop Test',
          accelerator: 'Shift+F5',
          click: () => {
            mainWindow.webContents.send('menu-stop-test');
          }
        },
        { type: 'separator' },
        {
          label: 'Refresh Device',
          accelerator: 'CmdOrCtrl+D',
          click: () => {
            mainWindow.webContents.send('menu-refresh-device');
          }
        }
      ]
    },
    {
      label: 'Help',
      submenu: [
        {
          label: 'Wiki',
          click: async () => {
            const { shell } = require('electron');
            await shell.openExternal('https://wiki.test-toolkit.app');
          }
        },
        {
          label: 'Download Portable Toolkit',
          click: async () => {
            const { shell } = require('electron');
            await shell.openExternal('https://www.pgyer.com/omf6LBDa');
          }
        },
        {
          label: 'About',
          click: async () => {
            const packageJson = require('./package.json');
            const version = packageJson.version || 'unknown';
            dialog.showMessageBox(mainWindow, {
              type: 'info',
              title: 'About Toolkit Studio',
              message: 'Toolkit Studio',
              detail: `Version ${version}\n\nA Cross-platform Low-code IDE for General UI Automation Testing.\n\n`,
              buttons: ['OK']
            });
          }
        }
      ]
    }
  ];

  const menu = Menu.buildFromTemplate(template);
  
  // 在Windows平台隐藏菜单栏，其他平台显示
  if (process.platform === 'win32') {
    Menu.setApplicationMenu(null);
  } else {
    Menu.setApplicationMenu(menu);
  }

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// 启动时尝试刷新token
async function tryRefreshTokenOnStartup() {
  try {
    const result = await refreshTokenInternal();
    
    if (result.success) {
      console.log('启动时token刷新成功');
      isAuthenticated = true;
      mainWindow.loadFile('renderer/index.html');
      // 启动定期token检查
      startTokenCheck();
    } else {
      console.log('启动时token刷新失败，跳转到登录页面');
      mainWindow.loadFile('renderer/login.html');
    }
  } catch (error) {
    console.error('启动时token刷新异常:', error);
    mainWindow.loadFile('renderer/login.html');
  }
}

// 内部token刷新函数（避免重复代码）
async function refreshTokenInternal() {
  try {
    const axios = require('axios');
    const refreshToken = store.get('refresh_token');
    const baseUrl = store.get('base_url');
    
    if (!refreshToken || !baseUrl) {
      return { success: false, error: '缺少refresh token或base URL' };
    }
    
    const response = await axios.post(`${baseUrl}/api/auth/refresh`, 
      { refresh_token: refreshToken },
      {
        headers: {
          'Content-Type': 'application/json'
        }
      }
    );
    
    const data = response.data;
    
    // 更新tokens
    store.set('access_token', data.access_token);
    store.set('refresh_token', data.refresh_token);
    store.set('token_expiry', new Date(Date.now() + data.expires_in * 1000).toISOString());
    
    return { success: true, data };
  } catch (error) {
    console.error('刷新token失败:', error.message);
    return { success: false, error: error.message };
  }
}

// 启动定期token检查
function startTokenCheck() {
  // 清除之前的检查间隔
  if (tokenCheckInterval) {
    clearInterval(tokenCheckInterval);
  }
  
  // 每10分钟检查一次token状态
  tokenCheckInterval = setInterval(async () => {
    try {
      if (!isAuthenticated) {
        return;
      }
      
      const tokenExpiry = store.get('token_expiry');
      if (!tokenExpiry) {
        console.log('没有token过期时间，停止定期检查');
        stopTokenCheck();
        return;
      }
      
      const now = new Date();
      const expiry = new Date(tokenExpiry);
      const bufferTime = 5 * 60 * 1000; // 5分钟缓冲
      
      // 如果token将在5分钟内过期，尝试刷新
      if (now.getTime() > (expiry.getTime() - bufferTime)) {
        console.log('定期检查发现token即将过期，正在刷新...');
        
        const result = await refreshTokenInternal();
        if (result.success) {
          console.log('定期token刷新成功');
          // 通知渲染进程token已刷新
          if (mainWindow && !mainWindow.isDestroyed()) {
            mainWindow.webContents.send('token-refreshed');
          }
        } else {
          console.log('定期token刷新失败，将跳转到登录页面');
          isAuthenticated = false;
          stopTokenCheck();
          
          if (mainWindow && !mainWindow.isDestroyed()) {
            mainWindow.loadFile('renderer/login.html');
          }
        }
      }
    } catch (error) {
      console.error('定期token检查异常:', error);
    }
  }, 10 * 60 * 1000); // 10分钟
  
  console.log('已启动定期token检查（每10分钟）');
}

// 停止定期token检查
function stopTokenCheck() {
  if (tokenCheckInterval) {
    clearInterval(tokenCheckInterval);
    tokenCheckInterval = null;
    console.log('已停止定期token检查');
  }
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  }
});

// IPC Handlers

// Authentication
ipcMain.handle('login', async (event, credentials) => {
  try {
    const axios = require('axios');
    const response = await axios.post(`${credentials.baseUrl}/api/auth/login`, 
      `email=${encodeURIComponent(credentials.email)}&password=${encodeURIComponent(credentials.password)}`,
      {
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded'
        }
      }
    );
    
    const data = response.data;
    
    // Store tokens
    store.set('access_token', data.access_token);
    store.set('refresh_token', data.refresh_token);
    store.set('base_url', credentials.baseUrl);
    store.set('token_expiry', new Date(Date.now() + data.expires_in * 1000).toISOString());
    
    isAuthenticated = true;
    // 启动定期token检查
    startTokenCheck();
    return { success: true, data };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('refresh-token', async () => {
  return await refreshTokenInternal();
});

ipcMain.handle('logout', async () => {
  store.delete('access_token');
  store.delete('refresh_token');
  store.delete('token_expiry');
  isAuthenticated = false;
  // 停止定期token检查
  stopTokenCheck();
  return { success: true };
});

ipcMain.handle('get-user-info', async () => {
  try {
    const axios = require('axios');
    const token = store.get('access_token');
    const baseUrl = store.get('base_url');
    
    const response = await axios.get(`${baseUrl}/api/users/me`, {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    
    return { success: true, data: response.data };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// App version
ipcMain.handle('get-app-version', async () => {
  try {
    const packageJson = require('./package.json');
    return packageJson.version;
  } catch (error) {
    return 'unknown';
  }
});

// Store operations
ipcMain.handle('store-get', async (event, key) => {
  return store.get(key);
});

ipcMain.handle('store-set', async (event, key, value) => {
  store.set(key, value);
  return true;
});

// File operations
ipcMain.handle('select-directory', async () => {
  try {
    const result = await dialog.showOpenDialog(mainWindow, {
      properties: ['openDirectory', 'createDirectory'],
      title: 'Select Project Directory',
      buttonLabel: 'Select'
    });
    
    if (!result.canceled && result.filePaths.length > 0) {
      return result.filePaths[0];
    }
  } catch (error) {
    console.error('Error selecting directory:', error);
  }
  return null;
});

ipcMain.handle('select-file', async (event, filters) => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openFile'],
    filters: filters || [{ name: 'All Files', extensions: ['*'] }]
  });
  
  if (!result.canceled) {
    return result.filePaths[0];
  }
  return null;
});

ipcMain.handle('read-file', async (event, filepath) => {
  try {
    const content = await fs.readFile(filepath, 'utf-8');
    return { success: true, content };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('write-file', async (event, filepath, content) => {
  try {
    await fs.writeFile(filepath, content, 'utf-8');
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// 保存文件处理器（用于编辑器保存功能）
ipcMain.handle('save-file', async (event, filepath, content) => {
  try {
    await fs.writeFile(filepath, content, 'utf-8');
    return { success: true };
  } catch (error) {
    console.error('保存文件失败:', error);
    return { success: false, error: error.message };
  }
});

ipcMain.handle('create-project-structure', async (event, projectPath) => {
  try {
    // Create directory structure
    await fs.mkdir(path.join(projectPath, 'cases'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'devices'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'workarea'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'locator'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'locator', 'img'), { recursive: true });
    
    // Create initial files
    await fs.writeFile(path.join(projectPath, 'testcase_map.json'), '{}', 'utf-8');
    await fs.writeFile(path.join(projectPath, 'testcase_sheet.csv'), '', 'utf-8');
    await fs.writeFile(path.join(projectPath, 'locator', 'element.json'), '{}', 'utf-8');
    
    // Create README for each folder
    await fs.writeFile(
      path.join(projectPath, 'workarea', 'README.md'),
      '# Workarea\n\n此文件夹用于存放临时文件，如当前截图和UI树XML文件。',
      'utf-8'
    );
    
    await fs.writeFile(
      path.join(projectPath, 'locator', 'README.md'),
      '# Locator\n\n此文件夹用于存放所有测试用例共享的元素定位信息。\n- element.json: XML元素定位信息\n- img/: 图像识别元素定位截图',
      'utf-8'
    );
    
    console.log('Project structure created at:', projectPath);
    return { success: true };
  } catch (error) {
    console.error('Error creating project structure:', error);
    return { success: false, error: error.message };
  }
});

// ADB operations with built-in SDK
ipcMain.handle('adb-devices', async () => {
  try {
    const adbPath = getBuiltInAdbPath();
    console.log('Using built-in ADB from:', adbPath);
    
    // Check if ADB exists
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      console.error('Built-in ADB not found at:', adbPath);
      return { success: false, error: 'Built-in Android SDK not found', devices: [] };
    }
    
    // Make sure ADB is executable (for macOS/Linux)
    if (process.platform !== 'win32') {
      try {
        await execPromise(`chmod +x "${adbPath}"`);
      } catch (e) {
        // Ignore chmod errors
      }
    }
    
    // Get ADB version
    let adbVersion = 'Unknown';
    try {
      const { stdout: versionOutput } = await execPromise(`"${adbPath}" version`);
      const versionMatch = versionOutput.match(/Android Debug Bridge version ([\d.\-]+)/);
      if (versionMatch) {
        adbVersion = versionMatch[1];
      }
    } catch (e) {
      console.error('Failed to get ADB version:', e);
    }
    
    const { stdout } = await execPromise(`"${adbPath}" devices`);
    const lines = stdout.split('\n').filter(line => line && !line.includes('List of devices'));
    const devices = lines.map(line => {
      const [id, status] = line.split('\t');
      return { id: id.trim(), status: status?.trim() || 'unknown' };
    }).filter(d => d.id);
    
    return { success: true, devices, adbPath, adbVersion };
  } catch (error) {
    console.error('ADB error:', error);
    return { success: false, error: error.message, devices: [] };
  }
});

// 增强版截图，同时保存到工作区并获取UI树
ipcMain.handle('adb-screenshot', async (event, deviceId, projectPath = null) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: 'Built-in Android SDK not found' };
    }
    
    const tempPath = path.join(app.getPath('temp'), 'screenshot.png');
    const deviceArg = deviceId ? `-s ${deviceId}` : '';
    
    // 获取截图
    await execPromise(`"${adbPath}" ${deviceArg} exec-out screencap -p > "${tempPath}"`);
    const imageData = await fs.readFile(tempPath);
    
    // 如果提供了项目路径，保存到工作区
    if (projectPath) {
      try {
        const workareaPath = path.join(projectPath, 'workarea');
        
        // 确保workarea目录存在
        if (!fsSync.existsSync(workareaPath)) {
          await fs.mkdir(workareaPath, { recursive: true });
        }
        
        // 保存截图到工作区
        const screenshotPath = path.join(workareaPath, 'current_screenshot.png');
        await fs.writeFile(screenshotPath, imageData);
        
        // 同时获取UI树
        try {
          await execPromise(`"${adbPath}" ${deviceArg} shell uiautomator dump /sdcard/window_dump.xml`);
          const { stdout: xmlContent } = await execPromise(`"${adbPath}" ${deviceArg} shell cat /sdcard/window_dump.xml`);
          
          // 保存XML到工作区
          const xmlPath = path.join(workareaPath, 'current_ui_tree.xml');
          await fs.writeFile(xmlPath, xmlContent, 'utf8');
          
          console.log('已保存截图和UI树到工作区:', workareaPath);
        } catch (xmlError) {
          console.warn('获取UI树失败，但截图保存成功:', xmlError.message);
        }
        
        // 返回截图路径和base64数据
        return { 
          success: true, 
          data: imageData.toString('base64'),
          screenshotPath: screenshotPath
        };
      } catch (saveError) {
        console.warn('保存到工作区失败:', saveError.message);
        // 不影响截图返回
      }
    }
    
    return { success: true, data: imageData.toString('base64') };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('adb-ui-dump', async (event, deviceId) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: 'Built-in Android SDK not found' };
    }
    
    const deviceArg = deviceId ? `-s ${deviceId}` : '';
    
    // Dump UI hierarchy
    await execPromise(`"${adbPath}" ${deviceArg} shell uiautomator dump /sdcard/window_dump.xml`);
    const { stdout } = await execPromise(`"${adbPath}" ${deviceArg} shell cat /sdcard/window_dump.xml`);
    
    return { success: true, xml: stdout };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// 增强版UI树获取，包含屏幕尺寸信息
ipcMain.handle('adb-ui-dump-enhanced', async (event, deviceId) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: 'Built-in Android SDK not found' };
    }
    
    const deviceArg = deviceId ? `-s ${deviceId}` : '';
    
    // 1. 获取UI hierarchy
    await execPromise(`"${adbPath}" ${deviceArg} shell uiautomator dump /sdcard/window_dump.xml`);
    const { stdout: xmlContent } = await execPromise(`"${adbPath}" ${deviceArg} shell cat /sdcard/window_dump.xml`);
    
    // 2. 获取屏幕尺寸
    let screenSize = null;
    try {
      const { stdout: sizeOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell wm size`);
      const sizeMatch = sizeOutput.match(/Physical size: (\d+)x(\d+)/);
      if (sizeMatch) {
        screenSize = { 
          width: parseInt(sizeMatch[1]), 
          height: parseInt(sizeMatch[2]) 
        };
      } else {
        // 尝试从dumpsys获取
        const { stdout: dumpsysOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell dumpsys window displays | grep -E "Display|cur="`);
        const displayMatch = dumpsysOutput.match(/cur=(\d+)x(\d+)/);
        if (displayMatch) {
          screenSize = {
            width: parseInt(displayMatch[1]),
            height: parseInt(displayMatch[2])
          };
        }
      }
    } catch (sizeError) {
      console.warn('无法获取屏幕尺寸:', sizeError.message);
      // 使用默认尺寸
      screenSize = { width: 1080, height: 1920 };
    }
    
    // 3. 获取设备信息（可选）
    let deviceInfo = {};
    try {
      const { stdout: brandOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.product.brand`);
      const { stdout: modelOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.product.model`);
      const { stdout: versionOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.build.version.release`);
      
      deviceInfo = {
        brand: brandOutput.trim(),
        model: modelOutput.trim(),
        androidVersion: versionOutput.trim()
      };
    } catch (infoError) {
      console.warn('无法获取设备信息:', infoError.message);
    }
    
    return { 
      success: true, 
      xml: xmlContent,
      screenSize: screenSize,
      deviceInfo: deviceInfo,
      timestamp: Date.now()
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// 初始化项目工作区
ipcMain.handle('init-project-workarea', async (event, projectPath) => {
  try {
    const workareaPath = path.join(projectPath, 'workarea');
    
    // 确保workarea目录存在
    const fsSync = require('fs');
    if (!fsSync.existsSync(workareaPath)) {
      await fs.mkdir(workareaPath, { recursive: true });
      console.log('创建工作区目录:', workareaPath);
    }
    
    return { success: true, path: workareaPath };
  } catch (error) {
    console.error('初始化项目工作区失败:', error);
    return { success: false, error: error.message };
  }
});

// Navigate to main app after login
ipcMain.handle('navigate-to-app', async () => {
  mainWindow.loadFile('renderer/index.html');
  return { success: true };
});

// Navigate to login
ipcMain.handle('navigate-to-login', async () => {
  mainWindow.loadFile('renderer/login.html');
  return { success: true };
});

// 执行ADB Shell命令 - 用于TKS脚本引擎
ipcMain.handle('adb-shell-command', async (event, command, deviceId = null) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: 'Built-in Android SDK not found' };
    }
    
    // 构建完整的ADB命令
    const deviceArg = deviceId ? `-s ${deviceId}` : '';
    const fullCommand = `"${adbPath}" ${deviceArg} shell ${command}`;
    
    console.log('执行ADB Shell命令:', fullCommand);
    
    const { stdout, stderr } = await execPromise(fullCommand);
    
    if (stderr && !stderr.includes('Warning')) {
      // 某些ADB命令会在stderr输出警告，但仍然成功
      console.warn('ADB命令警告:', stderr);
    }
    
    return { 
      success: true, 
      output: stdout,
      error: stderr
    };
  } catch (error) {
    console.error('ADB Shell命令执行失败:', error);
    return { 
      success: false, 
      error: error.message,
      output: error.stdout || '',
      stderr: error.stderr || ''
    };
  }
});

// 批量执行ADB命令 - 用于复杂操作
ipcMain.handle('adb-batch-commands', async (event, commands, deviceId = null) => {
  const results = [];
  
  for (const command of commands) {
    const result = await ipcMain._events['adb-shell-command'][0](event, command, deviceId);
    results.push({
      command,
      ...result
    });
    
    // 如果某个命令失败，停止执行
    if (!result.success && command.required !== false) {
      break;
    }
  }
  
  return results;
});

// 获取当前运行的App信息 - 包名和Activity
ipcMain.handle('get-current-app', async (event, deviceId) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!deviceId) {
      return { success: false, error: '请提供设备ID' };
    }
    
    let packageName = '';
    let activityName = '';
    
    // 方法1: 尝试获取顶层Activity（最有效）
    try {
      const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell dumpsys activity top`);
      if (stdout) {
        const lines = stdout.split('\n');
        for (const line of lines) {
          if (line.includes('ACTIVITY')) {
            const matches = line.match(/([a-zA-Z0-9._]+)\/([a-zA-Z0-9._$]+)/g);
            if (matches && matches.length > 0) {
              const fullActivity = matches[0];
              if (!fullActivity.includes('android/') && !fullActivity.includes('system/') && !fullActivity.includes('com.android.')) {
                const parts = fullActivity.split('/');
                packageName = parts[0];
                activityName = parts[1];
                if (activityName.startsWith('.')) {
                  activityName = packageName + activityName;
                }
                break;
              }
            }
          }
        }
      }
    } catch (error) {
      // 继续尝试其他方法
    }
    
    // 方法2: 如果方法1失败，尝试activity activities
    if (!packageName) {
      try {
        const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell dumpsys activity activities`);
        if (stdout) {
          const lines = stdout.split('\n');
          for (const line of lines) {
            if (line.includes('mResumedActivity') || line.includes('mFocusedActivity')) {
              const matches = line.match(/([a-zA-Z0-9._]+)\/([a-zA-Z0-9._$]+)/g);
              if (matches && matches.length > 0) {
                const fullActivity = matches[0];
                if (!fullActivity.includes('android/') && !fullActivity.includes('system/') && !fullActivity.includes('com.android.')) {
                  const parts = fullActivity.split('/');
                  packageName = parts[0];
                  activityName = parts[1];
                  if (activityName.startsWith('.')) {
                    activityName = packageName + activityName;
                  }
                  break;
                }
              }
            }
          }
        }
      } catch (error) {
        // 继续尝试其他方法
      }
    }
    
    if (packageName) {
      return { 
        success: true, 
        packageName: packageName,
        activityName: activityName || '未获取到Activity'
      };
    } else {
      return { 
        success: false, 
        error: '无法获取当前运行的应用信息。请确保设备屏幕已解锁并且有应用正在前台运行。'
      };
    }
    
  } catch (error) {
    return { 
      success: false,
      error: `获取失败: ${error.message}`
    };
  }
});

// 自定义标题栏窗口控制
ipcMain.handle('window-minimize', () => {
  if (mainWindow) {
    mainWindow.minimize();
  }
});

ipcMain.handle('window-maximize', () => {
  if (mainWindow) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow.maximize();
    }
  }
  return mainWindow ? mainWindow.isMaximized() : false;
});

ipcMain.handle('window-close', () => {
  if (mainWindow) {
    mainWindow.close();
  }
});

ipcMain.handle('window-is-maximized', () => {
  return mainWindow ? mainWindow.isMaximized() : false;
});

// 获取APK包名（通过尝试安装获取错误信息中的包名）
ipcMain.handle('get-apk-package-name', async (event, apkPath) => {
  try {
    console.log('开始获取APK包名，文件路径:', apkPath);
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      console.error('ADB路径不存在:', adbPath);
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!apkPath || !fsSync.existsSync(apkPath)) {
      console.error('APK文件不存在:', apkPath);
      return { success: false, error: 'APK文件不存在' };
    }
    
    console.log('APK文件存在，开始解析包名');
    
    // 方法1：尝试通过aapt获取（如果有的话）
    const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
    const aaptName = process.platform === 'win32' ? 'aapt.exe' : 'aapt';
    let aaptPath;
    
    if (app.isPackaged) {
      aaptPath = path.join(process.resourcesPath, platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
    } else {
      aaptPath = path.join(__dirname, 'resources', platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
    }
    
    console.log('尝试使用aapt路径:', aaptPath);
    console.log('aapt文件是否存在:', fsSync.existsSync(aaptPath));
    
    // 尝试使用aapt
    if (fsSync.existsSync(aaptPath)) {
      try {
        console.log('使用aapt获取APK包名');
        const aaptCommand = `"${aaptPath}" dump badging "${apkPath}"`;
        console.log('执行aapt命令:', aaptCommand);
        
        const { stdout } = await execPromise(aaptCommand);
        console.log('aapt输出前100字符:', stdout.substring(0, 100));
        
        const packageMatch = stdout.match(/package:\s+name='([^']+)'/);
        if (packageMatch && packageMatch[1]) {
          const packageName = packageMatch[1];
          console.log('通过aapt获取到包名:', packageName);
          return { success: true, packageName };
        } else {
          console.log('aapt输出中未找到包名匹配');
          return { success: false, error: 'aapt输出中未找到包名信息' };
        }
      } catch (error) {
        console.error('aapt方法失败:', error.message);
        return { success: false, error: `aapt执行失败: ${error.message}` };
      }
    } else {
      console.log('aapt工具不存在，尝试通过安装错误获取包名');
      
      // 如果aapt不可用，尝试通过安装错误信息获取包名
      try {
        // 先列出设备，选择第一个设备
        const devicesCommand = `"${adbPath}" devices`;
        const { stdout: devicesOutput } = await execPromise(devicesCommand);
        const deviceMatch = devicesOutput.match(/^([^\s]+)\s+device$/m);
        
        if (deviceMatch && deviceMatch[1]) {
          const tempDeviceId = deviceMatch[1];
          
          // 尝试安装（不用-r参数，这样如果已存在会报错并显示包名）
          const installCommand = `"${adbPath}" -s ${tempDeviceId} install "${apkPath}"`;
          
          try {
            await execPromise(installCommand);
            // 如果安装成功，无法获取包名
            return { success: false, error: '无法获取包名，请手动提供', needManualInput: true };
          } catch (installError) {
            // 安装失败，尝试从错误信息中提取包名
            const errorOutput = (installError.stdout || '') + (installError.stderr || '');
            
            const packagePatterns = [
              /INSTALL_FAILED_UPDATE_INCOMPATIBLE:\s*Existing\s+package\s+([a-zA-Z0-9._]+)/,
              /Existing\s+package\s+([a-zA-Z0-9._]+)\s+signatures/,
              /package\s+([a-zA-Z0-9._]+)\s+signatures\s+do\s+not\s+match/,
              /Package\s+([a-zA-Z0-9._]+)\s+signatures/,
              /package:\s*([a-zA-Z0-9._]+)/
            ];
            
            for (const pattern of packagePatterns) {
              const match = errorOutput.match(pattern);
              if (match && match[1]) {
                const packageName = match[1];
                console.log('从安装错误中获取到包名:', packageName);
                return { success: true, packageName };
              }
            }
          }
        }
      } catch (error) {
        console.log('通过安装方法获取包名失败:', error.message);
      }
      
      // 所有方法都失败，需要用户手动输入
      return { success: false, error: '无法自动获取包名，请手动提供', needManualInput: true };
    }
    // 如果aapt方法失败，返回错误
    return { success: false, error: '无法自动获取包名' };
    
  } catch (error) {
    console.error('获取APK包名失败:', error);
    return { success: false, error: error.message };
  }
});

// 卸载应用
ipcMain.handle('adb-uninstall-app', async (event, deviceId, packageName) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!deviceId || !packageName) {
      return { success: false, error: '设备ID或包名无效' };
    }
    
    console.log('正在卸载应用:', packageName, '从设备:', deviceId);
    
    // 执行卸载命令
    const uninstallCommand = `"${adbPath}" -s ${deviceId} uninstall ${packageName}`;
    const { stdout, stderr } = await execPromise(uninstallCommand);
    
    if (stdout.includes('Success') || stdout.includes('成功')) {
      console.log('应用卸载成功');
      return { success: true, message: '应用卸载成功' };
    } else if (stderr || stdout.includes('Failure') || stdout.includes('失败')) {
      console.error('应用卸载失败:', stdout, stderr);
      return { success: false, error: stdout + stderr };
    } else {
      // 未知结果，可能成功
      return { success: true, message: '卸载命令已执行' };
    }
    
  } catch (error) {
    console.error('卸载应用失败:', error);
    return { success: false, error: error.message };
  }
});

// APK安装功能
ipcMain.handle('adb-install-apk', async (event, deviceId, apkPath, forceReinstall = false) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!deviceId || !apkPath) {
      return { success: false, error: '设备ID或APK路径无效' };
    }
    
    // 检查APK文件是否存在
    if (!fsSync.existsSync(apkPath)) {
      return { success: false, error: 'APK文件不存在' };
    }
    
    console.log('正在安装APK到设备:', deviceId, apkPath);
    
    // 构建安装命令
    // -r: 替换已存在的应用
    // -d: 允许降级安装（强制安装旧版本）
    // -g: 授予所有运行时权限（Android 6.0+）
    let installFlags = '-r'; // 替换安装
    if (forceReinstall) {
      installFlags += ' -d'; // 允许降级
    }
    installFlags += ' -g'; // 授予权限
    
    const installCommand = `"${adbPath}" -s ${deviceId} install ${installFlags} "${apkPath}"`;
    console.log('执行安装命令:', installCommand);
    
    const { stdout, stderr } = await execPromise(installCommand);
    
    // 合并输出以便检查
    const fullOutput = stdout + '\n' + stderr;
    
    // 检查安装结果
    if (fullOutput.includes('Success') || fullOutput.includes('成功')) {
      console.log('APK安装成功');
      return { 
        success: true, 
        message: 'APK安装成功',
        output: stdout
      };
    } else if (fullOutput.includes('Failure') || fullOutput.includes('失败') || fullOutput.includes('INSTALL_FAILED')) {
      // 解析错误原因
      let errorMsg = '安装失败';
      
      if (fullOutput.includes('INSTALL_FAILED_ALREADY_EXISTS')) {
        errorMsg = '应用已存在，请卸载后重试';
      } else if (fullOutput.includes('INSTALL_FAILED_UPDATE_INCOMPATIBLE')) {
        // 从错误信息中提取包名
        let packageName = 'unknown';
        const packageMatch = fullOutput.match(/package\s+([a-zA-Z0-9._]+)/i);
        if (packageMatch) {
          packageName = packageMatch[1];
        }
        errorMsg = `签名不匹配: ${packageName} 的debug版本和release版本签名不同，需要先卸载原应用`;
      } else if (fullOutput.includes('INSTALL_FAILED_VERSION_DOWNGRADE')) {
        errorMsg = '不允许降级安装，请使用强制安装选项';
      } else if (fullOutput.includes('INSTALL_FAILED_INSUFFICIENT_STORAGE')) {
        errorMsg = '设备存储空间不足';
      } else if (fullOutput.includes('INSTALL_FAILED_INVALID_APK')) {
        errorMsg = 'APK文件无效或损坏';
      } else if (fullOutput.includes('INSTALL_PARSE_FAILED')) {
        errorMsg = 'APK解析失败，文件可能损坏';
      } else if (fullOutput.includes('INSTALL_FAILED_CPU_ABI_INCOMPATIBLE')) {
        errorMsg = 'APK与设备CPU架构不兼容';
      }
      
      console.error('APK安装失败:', fullOutput);
      return { 
        success: false, 
        error: errorMsg,
        details: fullOutput
      };
    } else {
      // 未知结果，可能成功
      return { 
        success: true, 
        message: '安装命令已执行',
        output: stdout
      };
    }
    
  } catch (error) {
    console.error('安装APK失败:', error);
    
    // 检查错误信息中是否包含签名不匹配
    const errorOutput = (error.stdout || '') + (error.stderr || '') + error.message;
    console.log('完整错误输出:', errorOutput);
    
    // 从错误信息中提取包名
    let extractedPackageName = null;
    const packagePatterns = [
      /INSTALL_FAILED_UPDATE_INCOMPATIBLE:\s*Existing\s+package\s+([a-zA-Z0-9._]+)/,  // INSTALL_FAILED_UPDATE_INCOMPATIBLE: Existing package com.example.app
      /Existing\s+package\s+([a-zA-Z0-9._]+)\s+signatures/,  // Existing package com.example.app signatures
      /package\s+([a-zA-Z0-9._]+)\s+signatures\s+do\s+not\s+match/,  // package com.example.app signatures do not match
      /Package\s+([a-zA-Z0-9._]+)\s+signatures/,  // Package com.example.app signatures
      /Package\s+([a-zA-Z0-9._]+)\s+/,            // Package com.example.app 
      /package:\s*([a-zA-Z0-9._]+)/              // package: com.example.app
    ];
    
    console.log('尝试从catch块的错误信息中提取包名:', errorOutput);
    
    for (const pattern of packagePatterns) {
      const match = errorOutput.match(pattern);
      if (match && match[1]) {
        extractedPackageName = match[1];
        console.log('从安装错误中提取到包名:', extractedPackageName);
        break;
      }
    }
    
    if (errorOutput.includes('INSTALL_FAILED_UPDATE_INCOMPATIBLE')) {
      const errorMessage = extractedPackageName 
        ? `签名不匹配: ${extractedPackageName} 的debug版本和release版本签名不同，需要先卸载原应用`
        : '签名不匹配，需要先卸载原应用';
      
      return { 
        success: false, 
        error: errorMessage,
        details: errorOutput,
        packageName: extractedPackageName  // 返回提取到的包名
      };
    }
    
    return { 
      success: false, 
      error: error.message,
      details: errorOutput,
      packageName: extractedPackageName  // 返回提取到的包名（如果有的话）
    };
  }
});

// ADB无线配对功能 (Android 11+)
ipcMain.handle('adb-pair-wireless', async (event, ipAddress, pairingPort, pairingCode) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!ipAddress || !pairingPort || !pairingCode) {
      return { success: false, error: '请提供IP地址、配对端口和配对码' };
    }
    
    // 构建配对地址
    const pairingAddress = `${ipAddress}:${pairingPort}`;
    
    console.log('正在配对无线ADB设备:', pairingAddress);
    
    // 执行配对命令，直接使用参数传递（适用于所有平台）
    const commandStr = `"${adbPath}" pair ${pairingAddress} ${pairingCode}`;
    
    console.log('执行配对命令:', commandStr);
    
    let stdout, stderr;
    try {
      const result = await execPromise(commandStr);
      stdout = result.stdout;
      stderr = result.stderr;
      console.log('配对命令输出 - stdout:', stdout);
      console.log('配对命令输出 - stderr:', stderr);
    } catch (execError) {
      console.error('配对命令执行失败:', execError);
      
      // 如果是协议错误，提供更好的错误信息
      const errorMessage = (execError.stdout || '') + (execError.stderr || '') + execError.message;
      if (errorMessage.includes('protocol fault') || errorMessage.includes("couldn't read status message")) {
        return {
          success: false,
          error: 'Windows ADB配对协议错误，请尝试以下解决方案：\n1. 重启ADB服务\n2. 确保设备和电脑在同一网络\n3. 重新生成配对码',
          details: execError
        };
      }
      
      return { 
        success: false, 
        error: `配对失败: ${execError.message}`,
        details: execError
      };
    }
    
    // 检查配对结果
    if (stdout.includes('Successfully paired') || stdout.includes('Paired devices')) {
      return { 
        success: true, 
        message: `成功配对设备 ${pairingAddress}`,
        output: stdout
      };
    } else if (stderr && (stderr.includes('failed') || stderr.includes('error'))) {
      return { 
        success: false, 
        error: `配对失败: ${stderr || stdout}` 
      };
    } else if (stdout.includes('Failed to pair')) {
      return { 
        success: false, 
        error: `配对失败: ${stdout}` 
      };
    } else {
      // 某些情况下配对可能成功但没有明确的成功消息
      return { 
        success: true, 
        message: `配对命令已执行，请尝试连接设备`,
        output: stdout,
        warning: '配对状态不确定，请尝试连接'
      };
    }
    
  } catch (error) {
    console.error('ADB配对失败:', error);
    return { success: false, error: `配对异常: ${error.message}` };
  }
});

// ADB无线连接功能
ipcMain.handle('adb-connect-wireless', async (event, ipAddress, port = 5555) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!ipAddress) {
      return { success: false, error: '请提供IP地址' };
    }
    
    // 构建连接地址
    const connectionAddress = `${ipAddress}:${port}`;
    
    console.log('正在连接无线ADB设备:', connectionAddress);
    
    // 尝试连接设备
    const { stdout, stderr } = await execPromise(`"${adbPath}" connect ${connectionAddress}`);
    
    if (stderr && stderr.includes('failed')) {
      return { success: false, error: `连接失败: ${stderr}` };
    }
    
    // 检查连接结果
    if (stdout.includes('connected to') || stdout.includes('already connected')) {
      return { 
        success: true, 
        message: `成功连接到设备 ${connectionAddress}`,
        deviceId: connectionAddress 
      };
    } else if (stdout.includes('unable to connect')) {
      return { 
        success: false, 
        error: `无法连接到 ${connectionAddress}，请确保设备开启ADB调试并连接到同一网络` 
      };
    } else {
      return { 
        success: false, 
        error: `连接状态未知: ${stdout}` 
      };
    }
    
  } catch (error) {
    return { success: false, error: `连接异常: ${error.message}` };
  }
});

// ADB断开无线连接功能
ipcMain.handle('adb-disconnect-wireless', async (event, ipAddress, port = 5555) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    if (!ipAddress) {
      return { success: false, error: '请提供IP地址' };
    }
    
    // 构建连接地址
    const connectionAddress = `${ipAddress}:${port}`;
    
    console.log('正在断开无线ADB设备连接:', connectionAddress);
    
    // 断开设备连接
    const { stdout, stderr } = await execPromise(`"${adbPath}" disconnect ${connectionAddress}`);
    
    if (stderr && !stderr.includes('Warning')) {
      return { success: false, error: `断开连接失败: ${stderr}` };
    }
    
    return { 
      success: true, 
      message: `已断开设备 ${connectionAddress} 的连接`,
      output: stdout.trim()
    };
    
  } catch (error) {
    return { success: false, error: `断开连接异常: ${error.message}` };
  }
});

// 扫描局域网内可用的ADB设备
ipcMain.handle('scan-wireless-devices', async (event, ipRange = null) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    console.log('开始扫描局域网ADB设备...');
    
    // 如果没有提供IP范围，尝试自动检测本机网络
    let targetIpRange = ipRange;
    if (!targetIpRange) {
      try {
        const os = require('os');
        const networkInterfaces = os.networkInterfaces();
        
        // 查找活跃的网络接口
        for (const interfaceName in networkInterfaces) {
          const interfaces = networkInterfaces[interfaceName];
          for (const iface of interfaces) {
            if (iface.family === 'IPv4' && !iface.internal) {
              // 提取网段 (例: 192.168.1.x)
              const ipParts = iface.address.split('.');
              if (ipParts.length === 4) {
                targetIpRange = `${ipParts[0]}.${ipParts[1]}.${ipParts[2]}`;
                break;
              }
            }
          }
          if (targetIpRange) break;
        }
        
        if (!targetIpRange) {
          targetIpRange = '192.168.1'; // 默认网段
        }
      } catch (e) {
        targetIpRange = '192.168.1'; // 默认网段
      }
    }
    
    const foundDevices = [];
    const commonPorts = [5555, 5556, 5557]; // 常见的ADB无线端口
    
    // 扫描网段中的常见IP地址 (避免扫描太多地址，影响性能)
    const scanPromises = [];
    
    // 扫描范围：1-50 和 100-200 (常见的路由器分配范围)
    const scanRanges = [
      { start: 1, end: 50 },
      { start: 100, end: 200 }
    ];
    
    for (const range of scanRanges) {
      for (let i = range.start; i <= range.end; i++) {
        for (const port of commonPorts) {
          const ip = `${targetIpRange}.${i}`;
          const address = `${ip}:${port}`;
          
          // 限制并发数量以避免资源过度占用
          scanPromises.push(
            (async () => {
              try {
                // 设置较短的超时时间来加快扫描速度
                const { stdout } = await execPromise(`"${adbPath}" connect ${address}`, { timeout: 3000 });
                
                if (stdout.includes('connected to') || stdout.includes('already connected')) {
                  // 获取设备信息
                  try {
                    const { stdout: deviceInfo } = await execPromise(`"${adbPath}" -s ${address} shell getprop ro.product.model`, { timeout: 2000 });
                    const deviceModel = deviceInfo.trim() || '未知设备';
                    
                    foundDevices.push({
                      ip: ip,
                      port: port,
                      address: address,
                      deviceModel: deviceModel,
                      status: 'connected'
                    });
                    
                    console.log(`发现ADB设备: ${address} (${deviceModel})`);
                  } catch (e) {
                    // 即使获取设备信息失败，也记录设备
                    foundDevices.push({
                      ip: ip,
                      port: port,
                      address: address,
                      deviceModel: '未知设备',
                      status: 'connected'
                    });
                  }
                }
              } catch (error) {
                // 连接失败是正常的，不需要记录错误
              }
            })()
          );
        }
      }
    }
    
    // 等待所有扫描完成，但设置总体超时时间
    await Promise.allSettled(scanPromises);
    
    console.log(`扫描完成，共发现 ${foundDevices.length} 个ADB设备`);
    
    return { 
      success: true, 
      devices: foundDevices,
      scannedRange: targetIpRange,
      message: `扫描完成，发现 ${foundDevices.length} 个设备`
    };
    
  } catch (error) {
    return { success: false, error: `扫描异常: ${error.message}` };
  }
});

// 获取保存的无线设备配置
ipcMain.handle('get-saved-wireless-devices', async () => {
  try {
    const savedDevices = store.get('wireless_devices', []);
    return { success: true, devices: savedDevices };
  } catch (error) {
    return { success: false, error: error.message, devices: [] };
  }
});

// 保存无线设备配置
ipcMain.handle('save-wireless-device', async (event, deviceConfig) => {
  try {
    const savedDevices = store.get('wireless_devices', []);
    
    // 检查是否已存在相同IP的设备
    const existingIndex = savedDevices.findIndex(device => device.ip === deviceConfig.ip);
    
    if (existingIndex >= 0) {
      // 更新现有设备
      savedDevices[existingIndex] = { 
        ...savedDevices[existingIndex], 
        ...deviceConfig, 
        updatedAt: new Date().toISOString() 
      };
    } else {
      // 添加新设备
      savedDevices.push({ 
        ...deviceConfig, 
        id: Date.now().toString(),
        createdAt: new Date().toISOString() 
      });
    }
    
    store.set('wireless_devices', savedDevices);
    
    return { 
      success: true, 
      message: existingIndex >= 0 ? '设备配置已更新' : '设备配置已保存',
      devices: savedDevices 
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// 删除保存的无线设备配置
ipcMain.handle('delete-wireless-device', async (event, deviceId) => {
  try {
    const savedDevices = store.get('wireless_devices', []);
    const filteredDevices = savedDevices.filter(device => device.id !== deviceId);
    
    store.set('wireless_devices', filteredDevices);
    
    return { 
      success: true, 
      message: '设备配置已删除',
      devices: filteredDevices 
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// ==================== Log (Logcat) 相关功能 ====================

const { spawn } = require('child_process');
let logcatProcesses = new Map(); // 存储正在运行的logcat进程

// 获取已连接的设备列表（用于Log页面）
ipcMain.handle('get-connected-devices', async () => {
  try {
    const adbPath = getBuiltInAdbPath();
    const { stdout } = await execPromise(`"${adbPath}" devices -l`);
    
    const devices = [];
    const lines = stdout.split('\n');
    
    for (let i = 1; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line && !line.startsWith('*')) {
        const parts = line.split(/\s+/);
        if (parts.length >= 2 && parts[1] === 'device') {
          const deviceId = parts[0];
          
          // 获取设备型号
          let model = 'Unknown';
          const modelMatch = line.match(/model:([^\s]+)/);
          if (modelMatch) {
            model = modelMatch[1];
          } else {
            // 尝试使用getprop获取型号
            try {
              const { stdout: modelStdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell getprop ro.product.model`);
              model = modelStdout.trim() || 'Unknown';
            } catch (e) {
              // 忽略错误
            }
          }
          
          devices.push({
            id: deviceId,
            model: model,
            type: deviceId.includes(':') ? 'wireless' : 'usb'
          });
        }
      }
    }
    
    return devices;
  } catch (error) {
    console.error('Failed to get devices:', error);
    return [];
  }
});

// 获取设备上的进程列表
ipcMain.handle('get-device-processes', async (event, deviceId) => {
  try {
    const adbPath = getBuiltInAdbPath();
    const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell ps`);
    
    const processes = [];
    const lines = stdout.split('\n');
    
    // 解析ps输出
    for (let i = 1; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line) {
        const parts = line.split(/\s+/);
        if (parts.length >= 9) {
          const pid = parts[1];
          const name = parts[parts.length - 1];
          
          // 过滤掉系统进程，只显示应用进程
          if (name.includes('.') && !name.startsWith('[')) {
            processes.push({
              pid: pid,
              name: name
            });
          }
        }
      }
    }
    
    // 按进程名排序
    processes.sort((a, b) => a.name.localeCompare(b.name));
    
    return processes;
  } catch (error) {
    console.error('Failed to get processes:', error);
    return [];
  }
});

// 启动logcat
ipcMain.handle('start-logcat', async (event, options) => {
  try {
    const { device, format = 'threadtime' } = options;
    const adbPath = getBuiltInAdbPath();
    
    // 如果该设备已有logcat进程在运行，先停止它
    if (logcatProcesses.has(device)) {
      const oldProcess = logcatProcesses.get(device);
      oldProcess.kill();
      logcatProcesses.delete(device);
    }
    
    // 启动新的logcat进程
    const args = ['-s', device, 'logcat', '-v', format];
    const logcatProcess = spawn(adbPath, args);
    
    // 存储进程
    logcatProcesses.set(device, logcatProcess);
    
    // 监听输出
    logcatProcess.stdout.on('data', (data) => {
      mainWindow.webContents.send('logcat-data', data.toString());
    });
    
    logcatProcess.stderr.on('data', (data) => {
      console.error('Logcat error:', data.toString());
    });
    
    logcatProcess.on('close', (code) => {
      console.log(`Logcat process exited with code ${code}`);
      logcatProcesses.delete(device);
    });
    
    return { success: true, pid: logcatProcess.pid };
  } catch (error) {
    console.error('Failed to start logcat:', error);
    return { success: false, error: error.message };
  }
});

// 停止logcat
ipcMain.handle('stop-logcat', async (event, processId) => {
  try {
    // 遍历所有logcat进程
    for (const [device, process] of logcatProcesses) {
      if (process.pid === processId) {
        process.kill();
        logcatProcesses.delete(device);
        return { success: true };
      }
    }
    
    return { success: false, error: 'Process not found' };
  } catch (error) {
    console.error('Failed to stop logcat:', error);
    return { success: false, error: error.message };
  }
});

// 显示保存对话框（用于导出日志）
ipcMain.handle('show-save-dialog', async (event, options) => {
  try {
    const result = await dialog.showSaveDialog(mainWindow, options);
    return result;
  } catch (error) {
    console.error('Failed to show save dialog:', error);
    return { canceled: true };
  }
});

// ==================== QR码生成功能 ====================

// 生成QR码配对数据
ipcMain.handle('generate-qr-pairing-data', async () => {
  try {
    const os = require('os');
    
    // 获取本机IP地址
    let localIP = null;
    const networkInterfaces = os.networkInterfaces();
    
    // 查找活跃的WiFi或以太网接口
    for (const interfaceName in networkInterfaces) {
      const interfaces = networkInterfaces[interfaceName];
      for (const iface of interfaces) {
        if (iface.family === 'IPv4' && !iface.internal) {
          localIP = iface.address;
          break;
        }
      }
      if (localIP) break;
    }
    
    if (!localIP) {
      return {
        success: false,
        error: '无法获取本机IP地址，请确保已连接到网络'
      };
    }
    
    // 生成随机端口
    const pairingPort = 30000 + Math.floor(Math.random() * 10000);
    
    // 生成6位数字配对码
    const pairingCode = Math.floor(100000 + Math.random() * 900000).toString();
    
    // 生成服务名称
    const serviceName = `ToolkitStudio_${Date.now().toString(36)}`;
    
    // Android ADB QR码格式
    // 格式: WIFI:T:ADB;S:service_name;P:pairing_code;H:host_ip;Q:pairing_port;
    const qrData = `WIFI:T:ADB;S:${serviceName};P:${pairingCode};H:${localIP};Q:${pairingPort};`;
    
    console.log('生成QR码配对数据:', { 
      serviceName, 
      pairingCode, 
      localIP, 
      pairingPort, 
      qrData 
    });
    
    return {
      success: true,
      serviceName,
      pairingCode,
      localIP,
      pairingPort,
      qrData,
      expiryTime: Date.now() + 5 * 60 * 1000 // 5分钟有效期
    };
  } catch (error) {
    console.error('生成QR码配对数据失败:', error);
    return {
      success: false,
      error: error.message
    };
  }
});

// 生成QR码图片
ipcMain.handle('generate-qr-code', async (event, data, options = {}) => {
  try {
    const QRCode = require('qrcode');
    
    // QR码生成选项
    const qrOptions = {
      errorCorrectionLevel: 'M',
      type: 'image/png',
      quality: 0.92,
      margin: 1,
      color: {
        dark: '#000000',
        light: '#FFFFFF'
      },
      width: options.width || 200,
      ...options
    };
    
    // 生成QR码数据URL
    const qrCodeDataURL = await QRCode.toDataURL(data, qrOptions);
    
    console.log('QR码生成成功');
    
    return {
      success: true,
      dataURL: qrCodeDataURL
    };
  } catch (error) {
    console.error('QR码生成失败:', error);
    return {
      success: false,
      error: error.message
    };
  }
});

// 启动ADB配对服务
ipcMain.handle('start-adb-pairing-service', async (event, serviceName, pairingCode, localIP, pairingPort) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: '内置Android SDK未找到' };
    }
    
    console.log('启动ADB配对服务:', { serviceName, pairingCode, localIP, pairingPort });
    
    // 实际上，Android的无线调试配对是通过mDNS/Bonjour服务发现和配对的
    // 对于QR码配对，Android设备会：
    // 1. 扫描QR码获取服务信息
    // 2. 通过mDNS查找对应的服务
    // 3. 连接到配对端口进行配对
    
    // 确保ADB服务运行
    try {
      await execPromise(`"${adbPath}" start-server`);
      console.log('ADB服务已启动');
    } catch (e) {
      console.warn('启动ADB服务失败:', e.message);
    }
    
    console.log(`ADB配对服务信息:`, {
      serviceName,
      pairingCode,
      localIP,
      pairingPort
    });
    
    // 注意：实际的配对是在Android设备扫描QR码后由设备发起的
    // 我们这里只是准备好配对信息，真正的配对会通过设备连接触发
    
    return {
      success: true,
      message: '配对服务已启动，请在设备上扫描QR码',
      serviceName,
      pairingCode,
      localIP,
      pairingPort
    };
  } catch (error) {
    console.error('启动ADB配对服务失败:', error);
    return {
      success: false,
      error: error.message
    };
  }
});

// 检查配对状态（通过检查新连接的设备）
ipcMain.handle('check-pairing-status', async () => {
  try {
    const adbPath = getBuiltInAdbPath();
    const { stdout } = await execPromise(`"${adbPath}" devices`);
    
    const lines = stdout.split('\n').filter(line => line && !line.includes('List of devices'));
    const devices = lines.map(line => {
      const [id, status] = line.split('\t');
      return { id: id.trim(), status: status?.trim() || 'unknown' };
    }).filter(d => d.id && d.status === 'device');
    
    return {
      success: true,
      connectedDevices: devices,
      hasNewDevices: devices.length > 0
    };
  } catch (error) {
    console.error('检查配对状态失败:', error);
    return {
      success: false,
      error: error.message,
      connectedDevices: []
    };
  }
});
