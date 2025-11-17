// TKE Device 模块的 IPC 处理器
// 负责通过 tke device 命令获取设备详细信息
const { ipcMain } = require('electron');
const { execFile } = require('child_process');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');
const { extractJsonFromOutput } = require('./tke-utils');

/**
 * 通用的 TKE Device 命令执行函数
 * @param {Object} app - Electron app 实例
 * @param {Array} args - 命令参数数组
 * @returns {Promise<string>} - 返回 JSON 字符串
 */
async function execTkeDeviceCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  console.log('执行TKE Device命令:', tkePath, args.join(' '));

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
        const error = new Error(`TKE Device命令失败 (exit code ${code}): ${stderr}`);
        error.code = code;
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
      } else {
        if (stderr && !stderr.includes('INFO')) {
          console.warn('TKE Device命令警告:', stderr);
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
 * 注册 TKE Device 相关的 IPC 处理器
 * @param {Object} app - Electron app 实例
 */
function registerTkeDeviceHandlers(app) {
  /**
   * 获取设备完整信息（包括硬件、电池、网络等所有信息）
   * IPC 通道: tke-device-info
   * 参数: { deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> device info
   * 输出格式:
   * {
   *   "id": "c9dc8614",
   *   "model": "CPH1921",
   *   "manufacturer": "OPPO",
   *   "android_version": "10",
   *   "screen_width": 1080,
   *   "screen_height": 2340,
   *   "hardware": {
   *     "cpu_model": "Qualcomm Technologies, Inc SM8150",
   *     "cpu_cores": 8,
   *     "cpu_abi": "arm64-v8a",
   *     "total_memory_mb": 7449,
   *     "available_memory_mb": 4977,
   *     "total_storage_gb": 223.04,
   *     "available_storage_gb": 182.59
   *   },
   *   "battery": {
   *     "level": 67,
   *     "temperature": 26.4,
   *     "health": "Good",
   *     "status": "Charging",
   *     "is_charging": true,
   *     "power_source": "USB"
   *   },
   *   "network": {
   *     "wifi_enabled": true,
   *     "wifi_ssid": "OPPO_Office",
   *     "mobile_network_type": "IWLAN",
   *     "operator_name": "",
   *     "operator_code": "",
   *     "country_iso": "au"
   *   }
   * }
   */
  ipcMain.handle('tke-device-info', async (event, { deviceId = null } = {}) => {
    try {
      const args = ['device', 'info'];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeDeviceCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE device info 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  /**
   * 获取设备的单个属性值（支持简短别名）
   * IPC 通道: tke-device-prop
   * 参数: { propName: string, deviceId?: string }
   * 返回: { success: boolean, output?: string, error?: string }
   *
   * 命令: tke --device <deviceId> device prop <propName>
   *
   * 支持的简短别名:
   * - 基础信息: version, sdk, model, manufacturer, brand, device, board, serial
   * - 硬件信息: cpu_abi, abi, hardware
   * - 构建信息: build_id, build_type, build_tags, fingerprint, build_date
   * - 网络信息: operator, carrier, operator_code, country, country_code, network_type
   *
   * 输出格式:
   * {
   *   "success": true,
   *   "property": "version",
   *   "value": "10"
   * }
   */
  ipcMain.handle('tke-device-prop', async (event, { propName, deviceId = null }) => {
    try {
      if (!propName) {
        return {
          success: false,
          error: '请提供属性名称'
        };
      }

      const args = ['device', 'prop', propName];

      // 如果指定了设备ID，添加 --device 参数
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const output = await execTkeDeviceCommand(app, args);

      return {
        success: true,
        output: output // 返回 JSON 字符串
      };
    } catch (error) {
      console.error('TKE device prop 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('TKE Device handlers 已注册');
}

module.exports = {
  registerTkeDeviceHandlers
};
