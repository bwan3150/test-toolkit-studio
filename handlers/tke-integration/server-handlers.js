// TKE Server 模块的 IPC 处理器
// 负责管理 tke-autoserver：启动、停止、状态检查
// 注意：此模块只负责执行 TKE 命令并返回原始输出，JSON 解析在 renderer 层完成
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
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

// 注册 TKE Server 相关的 IPC 处理器
function registerServerHandlers(app) {
  // 启动 autoserver - tke server start [-p port]
  ipcMain.handle('tke-server-start', async (event, deviceId, port) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'server', 'start'];

      // 如果指定了端口，添加端口参数
      if (port && typeof port === 'number') {
        args.push('-p', port.toString());
      }

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output // 返回原始 JSON 字符串
      };
    } catch (error) {
      console.error('TKE server start失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止 autoserver - tke server stop
  ipcMain.handle('tke-server-stop', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'server', 'stop'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE server stop失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 检查 autoserver 状态 - tke server status
  ipcMain.handle('tke-server-status', async (event, deviceId) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const args = ['--device', deviceId, 'server', 'status'];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE server status失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerServerHandlers
};
