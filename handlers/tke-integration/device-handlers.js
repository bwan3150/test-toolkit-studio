// 设备管理相关的IPC处理器
// 负责设备信息获取、应用列表、设备控制等
const { ipcMain } = require('electron');
const { execTkeAdbCommand } = require('./adb-handlers');

// 注册设备管理相关的IPC处理器
function registerDeviceHandlers(app) {
  // 获取连接的设备（兼容旧版本的调用）
  ipcMain.handle('get-connected-devices', async () => {
    try {
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['devices', '-l']);

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
    try {
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['devices', '-l']);

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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取设备基本信息
      const commands = [
        { key: 'model', args: ['shell', 'getprop', 'ro.product.model'] },
        { key: 'brand', args: ['shell', 'getprop', 'ro.product.brand'] },
        { key: 'androidVersion', args: ['shell', 'getprop', 'ro.build.version.release'] },
        { key: 'sdkVersion', args: ['shell', 'getprop', 'ro.build.version.sdk'] },
        { key: 'resolution', args: ['shell', 'wm', 'size'] },
        { key: 'density', args: ['shell', 'wm', 'density'] }
      ];

      const deviceInfo = {};

      for (const { key, args } of commands) {
        try {
          const { stdout } = await execTkeAdbCommand(app, deviceId, args);
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
      // 获取当前设备列表
      const { stdout } = await execTkeAdbCommand(app, null, ['devices']);
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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取所有已安装的包
      const { stdout } = await execTkeAdbCommand(app, deviceId, ['shell', 'pm', 'list', 'packages']);

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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 获取第三方应用
      const { stdout } = await execTkeAdbCommand(app, deviceId, ['shell', 'pm', 'list', 'packages', '-3']);

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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      await execTkeAdbCommand(app, deviceId, ['reboot']);

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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const { stdout } = await execTkeAdbCommand(app, deviceId, ['logcat', '-t', lines.toString()]);

      return {
        success: true,
        log: stdout
      };

    } catch (error) {
      console.error('获取设备日志失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 清除设备日志
  ipcMain.handle('clear-device-log', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      await execTkeAdbCommand(app, deviceId, ['logcat', '-c']);

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
  registerDeviceHandlers
};
