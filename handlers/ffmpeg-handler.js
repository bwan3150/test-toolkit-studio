// FFmpeg Handler - 处理 FFmpeg 相关操作
// 提供视频处理、帧提取等功能

const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');

/**
 * 注册 FFmpeg 相关的 IPC handlers
 * @param {Electron.App} app - Electron app实例
 */
function registerFfmpegHandlers(app) {
  // 执行 tke ffmpeg 命令
  ipcMain.handle('tke-ffmpeg', async (event, { args }) => {
    return new Promise((resolve, reject) => {
      const tkeExecPath = getTkeExecutablePath(app);

      // 构建完整命令: tke ffmpeg [args...]
      const fullArgs = ['ffmpeg', ...args];

      const tkeProcess = spawn(tkeExecPath, fullArgs, {
        stdio: ['pipe', 'pipe', 'pipe']
      });

      let stdout = '';
      let stderr = '';
      const stdoutChunks = [];
      const stderrChunks = [];

      tkeProcess.stdout.on('data', (data) => {
        stdoutChunks.push(data);
        stdout += data.toString();
      });

      tkeProcess.stderr.on('data', (data) => {
        stderrChunks.push(data);
        stderr += data.toString();
      });

      tkeProcess.on('error', (error) => {
        console.error('TKE FFmpeg 执行失败:', error);
        reject(new Error(`执行FFmpeg失败: ${error.message}`));
      });

      tkeProcess.on('close', (code) => {
        if (code === 0) {
          resolve({
            success: true,
            stdout: Buffer.concat(stdoutChunks),
            stderr: stderr,
            exitCode: code
          });
        } else {
          reject(new Error(`FFmpeg 命令失败 (exit code ${code}): ${stderr}`));
        }
      });
    });
  });

  console.log('FFmpeg handlers 已注册');
}

/**
 * 获取 tke 可执行文件路径
 * @param {Electron.App} app
 * @returns {string}
 */
function getTkeExecutablePath(app) {
  const platform = process.platform;
  let platformName;

  if (platform === 'darwin') {
    platformName = 'darwin';
  } else if (platform === 'win32') {
    platformName = 'win32';
  } else {
    platformName = 'linux';
  }

  const execName = platform === 'win32' ? 'tke.exe' : 'tke';
  const tkeExecPath = path.join(
    app.getAppPath(),
    'resources',
    platformName,
    'toolkit-engine',
    execName
  );

  return tkeExecPath;
}

module.exports = {
  registerFfmpegHandlers
};
