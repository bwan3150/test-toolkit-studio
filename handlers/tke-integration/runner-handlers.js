// TKE Runner 模块的 IPC 处理器
// 负责脚本和项目的执行
// 注意：所有 runner 命令都返回 JSON 格式输出
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE Runner 命令执行函数（返回 JSON）
async function execTkeRunnerCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  console.log('执行TKE Runner命令:', tkePath, args.join(' '));

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
      // Runner 命令即使失败也会返回 JSON，所以不要只根据 exit code 判断
      if (stderr && !stderr.includes('INFO')) {
        console.warn('TKE Runner命令警告:', stderr);
      }

      try {
        // 解析 JSON 输出
        const result = JSON.parse(stdout.trim());
        resolve(result);
      } catch (parseError) {
        // JSON 解析失败
        reject(new Error(`TKE Runner命令输出无效的JSON: ${parseError.message}\n输出: ${stdout}`));
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

// 注册 TKE Runner 相关的 IPC 处理器
function registerRunnerHandlers(app) {
  // 执行单行脚本指令 - tke run step <line>
  ipcMain.handle('tke-run-step', async (event, deviceId, projectPath, line) => {
    try {
      if (!line || typeof line !== 'string') {
        return { success: false, error: '请提供脚本指令' };
      }

      // 构建命令参数
      const args = ['run', 'step', line];

      // 如果提供了设备ID和项目路径，添加到参数中
      if (deviceId) {
        args.unshift('--device', deviceId);
      }
      if (projectPath) {
        args.unshift('--project', projectPath);
      }

      const result = await execTkeRunnerCommand(app, args);
      return result; // 直接返回 JSON 结果
    } catch (error) {
      console.error('TKE run step失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 执行单个脚本文件 - tke run script <script_path>
  ipcMain.handle('tke-run-script', async (event, deviceId, projectPath, scriptPath) => {
    try {
      if (!scriptPath || typeof scriptPath !== 'string') {
        return { success: false, error: '请提供脚本路径' };
      }

      if (!fs.existsSync(scriptPath)) {
        return { success: false, error: '脚本文件不存在' };
      }

      // 构建命令参数
      const args = ['run', 'script', scriptPath];

      // 如果提供了设备ID和项目路径，添加到参数中
      if (deviceId) {
        args.unshift('--device', deviceId);
      }
      if (projectPath) {
        args.unshift('--project', projectPath);
      }

      const result = await execTkeRunnerCommand(app, args);
      return result; // 直接返回 JSON 结果
    } catch (error) {
      console.error('TKE run script失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 执行项目中所有脚本 - tke run project
  ipcMain.handle('tke-run-project', async (event, deviceId, projectPath) => {
    try {
      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      // 构建命令参数
      const args = ['--project', projectPath, 'run', 'project'];

      // 如果提供了设备ID，添加到参数中
      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      const result = await execTkeRunnerCommand(app, args);
      return result; // 直接返回 JSON 结果
    } catch (error) {
      console.error('TKE run project失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });
}

module.exports = {
  registerRunnerHandlers
};
