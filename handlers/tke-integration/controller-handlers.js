// TKE Controller 模块的 IPC 处理器
// 负责设备控制操作：点击、滑动、截图、应用启动等
// 注意：此模块只负责执行 TKE 命令并返回原始输出，JSON 解析在 renderer 层完成
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE 命令执行函数
async function execTkeCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  console.log('执行TKE命令:', tkePath, args.join(' '));

  return new Promise((resolve, reject) => {
    const child = spawn(tkePath, args);

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (code) => {
      if (code !== 0) {
        const error = new Error(`TKE命令失败 (exit code ${code}): ${stderr}`);
        error.code = code;
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
      } else {
        if (stderr && !stderr.includes('INFO')) {
          console.warn('TKE命令警告:', stderr);
        }

        // 验证输出是否为 JSON
        const output = stdout.trim();
        try {
          JSON.parse(output);
          resolve(output); // 返回原始 JSON 字符串
        } catch (e) {
          reject(new Error(`TKE 输出不是有效的 JSON: ${output}`));
        }
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

// 注册 TKE Controller 相关的 IPC 处理器
function registerControllerHandlers(app) {
  // 获取设备列表 - tke controller devices
  ipcMain.handle('tke-controller-devices', async () => {
    try {
      const args = ['controller', 'devices'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output // 返回原始 JSON 字符串
      };
    } catch (error) {
      console.error('TKE controller devices失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 截图和获取UI树 - tke controller capture
  ipcMain.handle('tke-controller-capture', async (event, deviceId, projectPath) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      // 确保workarea目录存在
      const workareaPath = path.join(projectPath, 'workarea');
      if (!fs.existsSync(workareaPath)) {
        fs.mkdirSync(workareaPath, { recursive: true });
      }

      const args = ['--device', deviceId, '--project', projectPath, 'controller', 'capture'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output // 返回原始 JSON 字符串
      };
    } catch (error) {
      console.error('TKE controller capture失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 点击操作 - tke controller tap <x> <y>
  ipcMain.handle('tke-controller-tap', async (event, deviceId, x, y) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (typeof x !== 'number' || typeof y !== 'number') {
        return { success: false, error: '坐标必须是数字' };
      }

      const args = ['--device', deviceId, 'controller', 'tap', x.toString(), y.toString()];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller tap失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 滑动操作 - tke controller swipe <x1> <y1> <x2> <y2> [--duration <ms>]
  ipcMain.handle('tke-controller-swipe', async (event, deviceId, x1, y1, x2, y2, duration = 300) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (typeof x1 !== 'number' || typeof y1 !== 'number' ||
          typeof x2 !== 'number' || typeof y2 !== 'number') {
        return { success: false, error: '坐标必须是数字' };
      }

      const args = [
        '--device', deviceId,
        'controller', 'swipe',
        x1.toString(), y1.toString(),
        x2.toString(), y2.toString(),
        '--duration', duration.toString()
      ];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller swipe失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 启动应用 - tke controller launch <package> <activity>
  ipcMain.handle('tke-controller-launch', async (event, deviceId, packageName, activityName) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!packageName || !activityName) {
        return { success: false, error: '请提供包名和Activity名' };
      }

      const args = ['--device', deviceId, 'controller', 'launch', packageName, activityName];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller launch失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止应用 - tke controller stop <package>
  ipcMain.handle('tke-controller-stop', async (event, deviceId, packageName) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!packageName) {
        return { success: false, error: '请提供包名' };
      }

      const args = ['--device', deviceId, 'controller', 'stop', packageName];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller stop失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 输入文本 - tke controller input "text"
  ipcMain.handle('tke-controller-input', async (event, deviceId, text) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!text || typeof text !== 'string') {
        return { success: false, error: '请提供要输入的文本' };
      }

      const args = ['--device', deviceId, 'controller', 'input', `"${text}"`];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller input失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 清空输入框 - tke controller clear-input
  ipcMain.handle('tke-controller-clear-input', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'controller', 'clear-input'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller clear-input失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 返回键 - tke controller back
  ipcMain.handle('tke-controller-back', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'controller', 'back'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller back失败:', error);
      return { success: false, error: error.message };
    }
  });

  // Home键 - tke controller home
  ipcMain.handle('tke-controller-home', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'controller', 'home'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE controller home失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerControllerHandlers
};
