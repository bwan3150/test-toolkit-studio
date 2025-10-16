// TKE Runner 模块的 IPC 处理器
// 负责脚本和项目的执行
// 注意：此模块只负责执行 TKE 命令并返回原始输出
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE 命令执行函数（runner 返回纯文本，不是 JSON）
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

        // runner 返回纯文本输出，不需要验证 JSON
        resolve(stdout.trim());
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

// 注册 TKE Runner 相关的 IPC 处理器
function registerRunnerHandlers(app) {
  // 执行单个脚本 - tke run script <script_path>
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

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output // 返回纯文本输出
      };
    } catch (error) {
      console.error('TKE run script失败:', error);
      return {
        success: false,
        error: error.message,
        output: error.stdout || ''
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

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output // 返回纯文本输出
      };
    } catch (error) {
      console.error('TKE run project失败:', error);
      return {
        success: false,
        error: error.message,
        output: error.stdout || ''
      };
    }
  });
}

module.exports = {
  registerRunnerHandlers
};
