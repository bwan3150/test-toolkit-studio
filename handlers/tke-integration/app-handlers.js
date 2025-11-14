// TKE App 模块的 IPC 处理器
// 负责 Android 设备应用管理：列出已安装应用、卸载应用等
const { ipcMain } = require('electron');
const { execFile } = require('child_process');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');
const { extractJsonFromOutput } = require('./tke-utils');

/**
 * 通用的 TKE App 命令执行函数
 * @param {Object} app - Electron app 实例
 * @param {Array} args - 命令参数数组
 * @returns {Promise<string>} - 返回 JSON 字符串
 */
async function execTkeAppCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  console.log('执行TKE App命令:', tkePath, args.join(' '));

  return new Promise((resolve, reject) => {
    const child = execFile(tkePath, args);

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
        const error = new Error(`TKE App命令失败 (exit code ${code}): ${stderr}`);
        error.code = code;
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
      } else {
        if (stderr && !stderr.includes('INFO')) {
          console.warn('TKE App命令警告:', stderr);
        }

        // 从输出中提取有效的 JSON
        try {
          const jsonOutput = extractJsonFromOutput(stdout);
          resolve(jsonOutput);
        } catch (e) {
          reject(e);
        }
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

/**
 * 注册 TKE App 相关的 IPC 处理器
 * @param {Object} app - Electron app 实例
 */
function registerAppHandlers(app) {
  /**
   * 列出设备上所有第三方应用及版本信息
   * IPC 通道: tke-app-list
   * 参数: { deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> app list
   * 输出格式:
   * {
   *   "success": true,
   *   "count": 4,
   *   "apps": [
   *     {
   *       "package_name": "com.example.app",
   *       "version_name": "1.0.0",
   *       "version_code": 1,
   *       "apk_path": "/data/app/..."
   *     }
   *   ]
   * }
   */
  ipcMain.handle('tke-app-list', async (event, { deviceId = null } = {}) => {
    try {
      const args = ['app', 'list'];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeAppCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE app list 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  /**
   * 强制卸载应用
   * IPC 通道: tke-app-uninstall
   * 参数: { package: string, deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> app uninstall <package>
   * 输出格式:
   * {
   *   "success": true,
   *   "message": "应用 com.example.app 卸载成功",
   *   "package": "com.example.app"
   * }
   */
  ipcMain.handle('tke-app-uninstall', async (event, { package: packageName, deviceId = null }) => {
    try {
      if (!packageName) {
        return {
          success: false,
          error: '请提供应用包名'
        };
      }

      const args = ['app', 'uninstall', packageName];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeAppCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE app uninstall 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  /**
   * 获取当前聚焦的应用信息
   * IPC 通道: tke-app-focus
   * 参数: { deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> app focus
   * 输出格式:
   * {
   *   "success": true,
   *   "package_name": "com.example.app",
   *   "activity_name": "com.example.app.MainActivity",
   *   "window_info": "mCurrentFocus=Window{...}"
   * }
   */
  ipcMain.handle('tke-app-focus', async (event, { deviceId = null } = {}) => {
    try {
      const args = ['app', 'focus'];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeAppCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE app focus 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  /**
   * 启动应用
   * IPC 通道: tke-app-launch
   * 参数: { package: string, activity: string, deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> app launch <package> <activity>
   * 输出格式:
   * {
   *   "success": true,
   *   "message": "应用 com.example.app 启动成功",
   *   "package": "com.example.app",
   *   "activity": ".MainActivity"
   * }
   */
  ipcMain.handle('tke-app-launch', async (event, { package: packageName, activity, deviceId = null }) => {
    try {
      if (!packageName) {
        return {
          success: false,
          error: '请提供应用包名'
        };
      }

      if (!activity) {
        return {
          success: false,
          error: '请提供Activity名称'
        };
      }

      const args = ['app', 'launch', packageName, activity];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeAppCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE app launch 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('TKE App handlers 已注册');
}

module.exports = {
  registerAppHandlers
};
