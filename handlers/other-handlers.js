// 其他IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { exec } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);
const Store = require('electron-store');

// 初始化store
const store = new Store();

// 注册其他IPC处理器
function registerOtherHandlers(app) {
  // 获取应用版本信息
  ipcMain.handle('get-app-version', async () => {
    try {
      const packageJson = require('../package.json');
      return packageJson.version || 'unknown';
    } catch (error) {
      return 'unknown';
    }
  });


  // 获取用户数据路径
  ipcMain.handle('get-user-data-path', () => {
    return app.getPath('userData');
  });

  // 获取连接的设备（兼容旧版本的调用）
  ipcMain.handle('get-connected-devices', async () => {
    // 重新实现设备获取逻辑
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, devices: [], error: '内置Android SDK未找到' };
      }
      
      const { stdout, stderr } = await execPromise(`"${adbPath}" devices -l`);
      
      if (stderr && !stderr.includes('daemon')) {
        console.error('ADB错误:', stderr);
      }
      
      const lines = stdout.split('\n');
      const devices = [];
      
      for (const line of lines) {
        if (line.includes('device ') && !line.startsWith('List of devices')) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 2) {
            const deviceId = parts[0];
            let deviceInfo = { id: deviceId, status: 'device' };
            
            // 提取设备信息
            const modelMatch = line.match(/model:([^\s]+)/);
            const deviceMatch = line.match(/device:([^\s]+)/);
            const productMatch = line.match(/product:([^\s]+)/);
            const transportMatch = line.match(/transport_id:([^\s]+)/);
            
            if (modelMatch) deviceInfo.model = modelMatch[1];
            if (deviceMatch) deviceInfo.device = deviceMatch[1];
            if (productMatch) deviceInfo.product = productMatch[1];
            if (transportMatch) deviceInfo.transportId = transportMatch[1];
            
            // 判断设备类型
            if (deviceId.includes(':')) {
              deviceInfo.type = 'wireless';
            } else {
              deviceInfo.type = 'usb';
            }
            
            devices.push(deviceInfo);
          }
        }
      }
      
      return { success: true, devices };
    } catch (error) {
      console.error('获取设备列表失败:', error);
      return { success: false, devices: [], error: error.message };
    }
  });

  // adb-devices（兼容旧版本的调用）
  ipcMain.handle('adb-devices', async () => {
    // 直接复制get-connected-devices的逻辑，避免循环调用
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, devices: [], error: '内置Android SDK未找到' };
      }
      
      const { stdout, stderr } = await execPromise(`"${adbPath}" devices -l`);
      
      const lines = stdout.split('\n');
      const devices = [];
      
      for (const line of lines) {
        if (line.includes('device ') && !line.startsWith('List of devices')) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 2) {
            const deviceId = parts[0];
            let deviceInfo = { id: deviceId, status: 'device' };
            
            // 提取设备信息
            const modelMatch = line.match(/model:([^\s]+)/);
            if (modelMatch) deviceInfo.model = modelMatch[1];
            
            // 判断设备类型
            if (deviceId.includes(':')) {
              deviceInfo.type = 'wireless';
            } else {
              deviceInfo.type = 'usb';
            }
            
            devices.push(deviceInfo);
          }
        }
      }
      
      return { success: true, devices };
    } catch (error) {
      console.error('获取设备列表失败:', error);
      return { success: false, devices: [], error: error.message };
    }
  });

  // 获取设备信息
  ipcMain.handle('get-device-info', async (event, deviceId) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取设备基本信息
      const commands = [
        { key: 'model', cmd: `"${adbPath}" -s ${deviceId} shell getprop ro.product.model` },
        { key: 'brand', cmd: `"${adbPath}" -s ${deviceId} shell getprop ro.product.brand` },
        { key: 'androidVersion', cmd: `"${adbPath}" -s ${deviceId} shell getprop ro.build.version.release` },
        { key: 'sdkVersion', cmd: `"${adbPath}" -s ${deviceId} shell getprop ro.build.version.sdk` },
        { key: 'resolution', cmd: `"${adbPath}" -s ${deviceId} shell wm size` },
        { key: 'density', cmd: `"${adbPath}" -s ${deviceId} shell wm density` }
      ];

      const deviceInfo = {};
      
      for (const { key, cmd } of commands) {
        try {
          const { stdout } = await execPromise(cmd);
          deviceInfo[key] = stdout.trim();
          
          // 解析特殊格式
          if (key === 'resolution' && stdout.includes('Physical size:')) {
            deviceInfo[key] = stdout.match(/Physical size:\s*(\d+x\d+)/)?.[1] || stdout.trim();
          }
          if (key === 'density' && stdout.includes('Physical density:')) {
            deviceInfo[key] = stdout.match(/Physical density:\s*(\d+)/)?.[1] || stdout.trim();
          }
        } catch (error) {
          console.error(`获取设备${key}失败:`, error);
          deviceInfo[key] = 'N/A';
        }
      }

      return { success: true, deviceInfo };

    } catch (error) {
      console.error('获取设备信息失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 检查配对状态
  ipcMain.handle('check-pairing-status', async () => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      // 获取当前设备列表
      const { stdout } = await execPromise(`"${adbPath}" devices`);
      const lines = stdout.split('\n');
      const deviceCount = lines.filter(line => 
        line.includes('device') && !line.startsWith('List of devices')
      ).length;

      // 简单检查是否有新设备
      const previousCount = global.lastDeviceCount || 0;
      global.lastDeviceCount = deviceCount;

      return { 
        success: true, 
        hasNewDevices: deviceCount > previousCount,
        deviceCount: deviceCount
      };

    } catch (error) {
      console.error('检查配对状态失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取应用列表
  ipcMain.handle('get-app-list', async (event, deviceId) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取所有已安装的包
      const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell pm list packages`);
      
      const packages = stdout
        .split('\n')
        .filter(line => line.startsWith('package:'))
        .map(line => line.replace('package:', '').trim())
        .filter(pkg => pkg.length > 0);

      return { 
        success: true, 
        packages: packages,
        count: packages.length
      };

    } catch (error) {
      console.error('获取应用列表失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取第三方应用列表
  ipcMain.handle('get-third-party-apps', async (event, deviceId) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取第三方应用
      const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell pm list packages -3`);
      
      const packages = stdout
        .split('\n')
        .filter(line => line.startsWith('package:'))
        .map(line => line.replace('package:', '').trim())
        .filter(pkg => pkg.length > 0);

      return { 
        success: true, 
        packages: packages,
        count: packages.length
      };

    } catch (error) {
      console.error('获取第三方应用列表失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 重启设备
  ipcMain.handle('reboot-device', async (event, deviceId) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const rebootCommand = `"${adbPath}" -s ${deviceId} reboot`;
      await execPromise(rebootCommand);

      return { 
        success: true, 
        message: '设备正在重启'
      };

    } catch (error) {
      console.error('重启设备失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取设备日志
  ipcMain.handle('get-device-log', async (event, deviceId, lines = 100) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const logCommand = `"${adbPath}" -s ${deviceId} logcat -t ${lines}`;
      const { stdout } = await execPromise(logCommand);

      return { 
        success: true, 
        log: stdout
      };

    } catch (error) {
      console.error('获取设备日志失败:', error);
      return { success: false, error: error.message };
    }
  });



  // 检查文件/目录是否存在
  ipcMain.handle('file-exists', async (event, filePath) => {
    try {
      return { 
        success: true, 
        exists: fs.existsSync(filePath),
        path: filePath
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 获取目录内容
  ipcMain.handle('read-directory', async (event, dirPath) => {
    try {
      if (!dirPath || typeof dirPath !== 'string') {
        return { success: false, error: '无效的目录路径' };
      }

      if (!fs.existsSync(dirPath)) {
        return { success: false, error: '目录不存在' };
      }

      const items = fs.readdirSync(dirPath);
      const contents = items.map(item => {
        const itemPath = path.join(dirPath, item);
        const stats = fs.statSync(itemPath);
        return {
          name: item,
          path: itemPath,
          isDirectory: stats.isDirectory(),
          isFile: stats.isFile(),
          size: stats.size,
          mtime: stats.mtime.toISOString()
        };
      });

      return { 
        success: true, 
        contents: contents,
        path: dirPath
      };

    } catch (error) {
      console.error('读取目录失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 清除设备日志
  ipcMain.handle('clear-device-log', async (event, deviceId) => {
    try {
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
      let adbPath;
      
      if (app.isPackaged) {
        adbPath = path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
      } else {
        adbPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
      }

      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const clearCommand = `"${adbPath}" -s ${deviceId} logcat -c`;
      await execPromise(clearCommand);

      return { 
        success: true, 
        message: '设备日志已清除'
      };

    } catch (error) {
      console.error('清除设备日志失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerOtherHandlers
};