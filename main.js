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
  mainWindow = new BrowserWindow({
    width: 1600,
    height: 900,
    minWidth: 1200,
    minHeight: 700,
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    trafficLightPosition: { x: 15, y: 13 },  // Position for macOS traffic lights
    frame: true,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
      webSecurity: false
    },
    backgroundColor: '#1e1e1e',
    icon: path.join(__dirname, 'assets', 'logo', 'toolkit_logo.png')
  });

  // Check if user has valid token
  const token = store.get('access_token');
  const tokenExpiry = store.get('token_expiry');
  
  if (token && tokenExpiry && new Date(tokenExpiry) > new Date()) {
    isAuthenticated = true;
    mainWindow.loadFile('renderer/index.html');
  } else {
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
          label: 'Documentation',
          click: async () => {
            const { shell } = require('electron');
            await shell.openExternal('https://docs.test-toolkit.app');
          }
        },
        {
          label: 'About',
          click: () => {
            dialog.showMessageBox(mainWindow, {
              type: 'info',
              title: 'About Toolkit Studio',
              message: 'Toolkit Studio',
              detail: 'Version 0.1.0\n\nA cross-platform IDE for UI automation testing.\n\n',
              buttons: ['OK']
            });
          }
        }
      ]
    }
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
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
    return { success: true, data };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('refresh-token', async () => {
  try {
    const axios = require('axios');
    const refreshToken = store.get('refresh_token');
    const baseUrl = store.get('base_url');
    
    const response = await axios.post(`${baseUrl}/api/auth/refresh`, 
      { refresh_token: refreshToken },
      {
        headers: {
          'Content-Type': 'application/json'
        }
      }
    );
    
    const data = response.data;
    
    // Update tokens
    store.set('access_token', data.access_token);
    store.set('refresh_token', data.refresh_token);
    store.set('token_expiry', new Date(Date.now() + data.expires_in * 1000).toISOString());
    
    return { success: true, data };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('logout', async () => {
  store.delete('access_token');
  store.delete('refresh_token');
  store.delete('token_expiry');
  isAuthenticated = false;
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

ipcMain.handle('create-project-structure', async (event, projectPath) => {
  try {
    // Create directory structure
    await fs.mkdir(path.join(projectPath, 'cases'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'devices'), { recursive: true });
    await fs.mkdir(path.join(projectPath, 'workarea'), { recursive: true });
    
    // Create initial files
    await fs.writeFile(path.join(projectPath, 'testcase_map.json'), '{}', 'utf-8');
    await fs.writeFile(path.join(projectPath, 'testcase_sheet.csv'), '', 'utf-8');
    
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

ipcMain.handle('adb-screenshot', async (event, deviceId) => {
  try {
    const adbPath = getBuiltInAdbPath();
    
    const fsSync = require('fs');
    if (!fsSync.existsSync(adbPath)) {
      return { success: false, error: 'Built-in Android SDK not found' };
    }
    
    const tempPath = path.join(app.getPath('temp'), 'screenshot.png');
    const deviceArg = deviceId ? `-s ${deviceId}` : '';
    
    await execPromise(`"${adbPath}" ${deviceArg} exec-out screencap -p > "${tempPath}"`);
    const imageData = await fs.readFile(tempPath);
    
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
