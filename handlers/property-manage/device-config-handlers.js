// 设备配置管理处理器
// 用于管理用户保存的设备配置（如设备别名、常用设备等）
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

/**
 * 获取设备配置文件路径
 * @param {Electron.App} app - Electron app实例
 * @returns {string} 设备配置文件的完整路径
 */
function getDeviceConfigPath(app) {
  return path.join(app.getPath('userData'), 'saved-devices.json');
}

/**
 * 注册所有设备配置相关的IPC处理器
 * @param {Electron.App} app - Electron app实例
 */
function registerDeviceConfigHandlers(app) {

  // 获取保存的设备列表
  ipcMain.handle('get-saved-devices', async () => {
    try {
      const configPath = getDeviceConfigPath(app);

      // 如果配置文件不存在，返回空列表
      if (!fs.existsSync(configPath)) {
        return { success: true, devices: [] };
      }

      // 读取并解析配置文件
      const data = fs.readFileSync(configPath, 'utf8');
      const devices = JSON.parse(data);

      return {
        success: true,
        devices: Array.isArray(devices) ? devices : []
      };

    } catch (error) {
      console.error('获取保存的设备配置失败:', error);
      return {
        success: false,
        devices: [],
        error: error.message
      };
    }
  });

  // 保存设备列表
  ipcMain.handle('save-devices', async (event, devices) => {
    try {
      // 验证输入
      if (!Array.isArray(devices)) {
        return {
          success: false,
          error: '设备列表必须是数组格式'
        };
      }

      const configPath = getDeviceConfigPath(app);

      // 确保目录存在
      const configDir = path.dirname(configPath);
      if (!fs.existsSync(configDir)) {
        fs.mkdirSync(configDir, { recursive: true });
      }

      // 写入配置文件（格式化JSON以便阅读）
      fs.writeFileSync(configPath, JSON.stringify(devices, null, 2), 'utf8');

      console.log(`设备配置已保存: ${devices.length} 个设备`);

      return { success: true };

    } catch (error) {
      console.error('保存设备配置失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 添加单个设备配置
  ipcMain.handle('add-saved-device', async (event, device) => {
    try {
      // 获取现有设备列表
      const result = await ipcMain._events['get-saved-devices'][0]();
      if (!result.success) {
        return result;
      }

      const devices = result.devices;

      // 检查设备是否已存在（通过设备ID）
      const existingIndex = devices.findIndex(d => d.id === device.id);

      if (existingIndex !== -1) {
        // 更新现有设备
        devices[existingIndex] = { ...devices[existingIndex], ...device };
      } else {
        // 添加新设备
        devices.push(device);
      }

      // 保存更新后的列表
      return await ipcMain._events['save-devices'][0](event, devices);

    } catch (error) {
      console.error('添加设备配置失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 删除单个设备配置
  ipcMain.handle('remove-saved-device', async (event, deviceId) => {
    try {
      // 获取现有设备列表
      const result = await ipcMain._events['get-saved-devices'][0]();
      if (!result.success) {
        return result;
      }

      // 过滤掉指定设备
      const devices = result.devices.filter(d => d.id !== deviceId);

      // 保存更新后的列表
      return await ipcMain._events['save-devices'][0](event, devices);

    } catch (error) {
      console.error('删除设备配置失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('设备配置管理处理器已注册');
}

module.exports = {
  registerDeviceConfigHandlers,
  getDeviceConfigPath
};
