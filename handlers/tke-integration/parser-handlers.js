// TKE Parser 模块的 IPC 处理器
// 负责脚本解析和验证
// 注意：此模块只负责执行 TKE 命令并返回原始输出，JSON 解析在 renderer 层完成
const { ipcMain } = require('electron');
const { promisify } = require('util');
const { exec } = require('child_process');
const path = require('path');
const fs = require('fs');
const execPromise = promisify(exec);
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE 命令执行函数
async function execTkeCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  const command = `"${tkePath}" ${args.join(' ')}`;
  console.log('执行TKE命令:', command);

  const { stdout, stderr } = await execPromise(command);

  if (stderr && !stderr.includes('INFO')) {
    console.warn('TKE命令警告:', stderr);
  }

  // 验证输出是否为 JSON
  const output = stdout.trim();
  try {
    JSON.parse(output);
    return output; // 返回原始 JSON 字符串
  } catch (e) {
    throw new Error(`TKE 输出不是有效的 JSON: ${output}`);
  }
}

// 注册 TKE Parser 相关的 IPC 处理器
function registerParserHandlers(app) {
  // 解析脚本 - tke parser parse <script_path>
  ipcMain.handle('tke-parser-parse', async (event, deviceId, projectPath, scriptPath) => {
    try {
      if (!scriptPath || typeof scriptPath !== 'string') {
        return { success: false, error: '请提供脚本路径' };
      }

      if (!fs.existsSync(scriptPath)) {
        return { success: false, error: '脚本文件不存在' };
      }

      // 构建命令参数
      const args = ['parser', 'parse', scriptPath];

      // 如果提供了设备ID和项目路径，添加到参数中
      if (deviceId) {
        args.unshift('--device', deviceId);
      }
      if (projectPath) {
        args.unshift('--project', projectPath);
      }

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE parser parse失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 验证脚本 - tke parser validate <script_path>
  ipcMain.handle('tke-parser-validate', async (event, deviceId, projectPath, scriptPath) => {
    try {
      if (!scriptPath || typeof scriptPath !== 'string') {
        return { success: false, error: '请提供脚本路径' };
      }

      if (!fs.existsSync(scriptPath)) {
        return { success: false, error: '脚本文件不存在' };
      }

      // 构建命令参数
      const args = ['parser', 'validate', scriptPath];

      // 如果提供了设备ID和项目路径，添加到参数中
      if (deviceId) {
        args.unshift('--device', deviceId);
      }
      if (projectPath) {
        args.unshift('--project', projectPath);
      }

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE parser validate失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerParserHandlers
};
