// TKE Parser 模块的 IPC 处理器
// 负责脚本解析和验证
// 注意：此模块只负责执行 TKE 命令并返回原始输出，JSON 解析在 renderer 层完成
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');
const { extractJsonFromOutput } = require('./tke-utils');

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

        // 从输出中提取有效的 JSON（处理混合输出的情况，如 JSON + WARN 日志）
        try {
          const jsonOutput = extractJsonFromOutput(stdout);
          resolve(jsonOutput); // 返回提取的 JSON 字符串
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
