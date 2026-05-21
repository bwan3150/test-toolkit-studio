// FFmpeg Handler - 处理 FFmpeg 相关操作
// 提供视频处理、帧提取等功能
// 注意: 作为依赖工具，直接使用 Electron 应用内置的 ffmpeg

const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * 注册 FFmpeg 相关的 IPC handlers
 * @param {Electron.App} app - Electron app实例
 */
function registerFfmpegHandlers(app) {
  // 执行 ffmpeg 命令 (使用 Electron 内置的 ffmpeg)
  ipcMain.handle('dep-ffmpeg', async (event, { args }) => {
    return new Promise((resolve, reject) => {
      const ffmpegPath = getFfmpegExecutablePath(app);

      // 检查 ffmpeg 是否存在
      if (!fs.existsSync(ffmpegPath)) {
        reject(new Error(`FFmpeg 未找到: ${ffmpegPath}`));
        return;
      }

      // 直接使用 ffmpeg 命令和参数
      const ffmpegProcess = spawn(ffmpegPath, args, {
        stdio: ['pipe', 'pipe', 'pipe']
      });

      let stdout = '';
      let stderr = '';
      const stdoutChunks = [];
      const stderrChunks = [];

      ffmpegProcess.stdout.on('data', (data) => {
        stdoutChunks.push(data);
        stdout += data.toString();
      });

      ffmpegProcess.stderr.on('data', (data) => {
        stderrChunks.push(data);
        stderr += data.toString();
      });

      ffmpegProcess.on('error', (error) => {
        console.error('FFmpeg 执行失败:', error);
        reject(new Error(`执行FFmpeg失败: ${error.message}`));
      });

      ffmpegProcess.on('close', (code) => {
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

  // 获取 ffmpeg 版本信息
  ipcMain.handle('get-ffmpeg-version', async () => {
    try {
      const ffmpegPath = getFfmpegExecutablePath(app);

      // 检查 ffmpeg 是否存在
      if (!fs.existsSync(ffmpegPath)) {
        return {
          success: false,
          error: 'FFmpeg 未找到'
        };
      }

      // 使用 -version 参数获取版本信息
      return new Promise((resolve) => {
        const ffmpegProcess = spawn(ffmpegPath, ['-version'], {
          stdio: ['pipe', 'pipe', 'pipe']
        });

        let stdout = '';
        let stderr = '';

        ffmpegProcess.stdout.on('data', (data) => {
          stdout += data.toString();
        });

        ffmpegProcess.stderr.on('data', (data) => {
          stderr += data.toString();
        });

        ffmpegProcess.on('error', (error) => {
          resolve({
            success: false,
            error: `执行失败: ${error.message}`
          });
        });

        ffmpegProcess.on('close', (code) => {
          if (code === 0 && stdout) {
            // 从输出中提取版本号
            // 输出格式: "ffmpeg version 8.0-tessus  https://evermeet.cx/ffmpeg/ ..."
            const versionMatch = stdout.match(/ffmpeg version ([^\s]+)/i);
            const version = versionMatch ? versionMatch[1] : 'Unknown';

            resolve({
              success: true,
              version: version
            });
          } else {
            resolve({
              success: false,
              error: 'Unable to get version'
            });
          }
        });
      });
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('FFmpeg handlers 已注册');
}

/**
 * 获取 ffmpeg 可执行文件路径
 * @param {Electron.App} app
 * @returns {string}
 */
function getFfmpegExecutablePath(app) {
  const platform = process.platform;
  let platformName;
  let execName;

  if (platform === 'darwin') {
    platformName = 'darwin';
    execName = 'ffmpeg';
  } else if (platform === 'win32') {
    platformName = 'win32';
    execName = 'ffmpeg.exe';
  } else {
    platformName = 'linux';
    execName = 'ffmpeg';
  }

  // 判断是开发环境还是打包环境
  // 开发环境: app.getAppPath() 返回项目根目录
  // 打包环境: app.getAppPath() 返回 app.asar 所在目录 (Contents/Resources/app.asar)
  const appPath = app.getAppPath();
  let ffmpegPath;

  if (app.isPackaged) {
    // 打包环境: /Applications/Toolkit Studio.app/Contents/Resources/darwin/toolkit-engine/ffmpeg
    // app.getAppPath() = /Applications/Toolkit Studio.app/Contents/Resources/app.asar
    // 需要回退到 Contents/Resources/ 然后进入 darwin/toolkit-engine/
    ffmpegPath = path.join(
      path.dirname(appPath), // Contents/Resources/
      platformName,
      'toolkit-engine',
      execName
    );
  } else {
    // 开发环境: 项目根目录/bin/darwin/toolkit-engine/ffmpeg
    ffmpegPath = path.join(
      appPath,
      'bin',
      platformName,
      'toolkit-engine',
      execName
    );
  }

  return ffmpegPath;
}

module.exports = {
  registerFfmpegHandlers
};
