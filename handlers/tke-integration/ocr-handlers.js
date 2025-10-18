// TKE OCR 模块的 IPC 处理器
// 负责在线和离线OCR识别
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

// 注册 TKE OCR 相关的 IPC 处理器
function registerOcrHandlers(app) {
  // 在线 OCR - tke ocr --image <image_path> --online --url <api_url>
  ipcMain.handle('tke-ocr-online', async (event, imagePath, apiUrl) => {
    try {
      if (!imagePath || typeof imagePath !== 'string') {
        return { success: false, error: '请提供图像路径' };
      }

      if (!fs.existsSync(imagePath)) {
        return { success: false, error: '图像文件不存在' };
      }

      if (!apiUrl || typeof apiUrl !== 'string') {
        return { success: false, error: '请提供OCR API URL' };
      }

      const args = ['ocr', '--image', imagePath, '--online', '--url', apiUrl];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE ocr在线识别失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 离线 OCR - tke ocr --image <image_path> --lang <language>
  ipcMain.handle('tke-ocr-offline', async (event, imagePath, language = 'eng') => {
    try {
      if (!imagePath || typeof imagePath !== 'string') {
        return { success: false, error: '请提供图像路径' };
      }

      if (!fs.existsSync(imagePath)) {
        return { success: false, error: '图像文件不存在' };
      }

      const args = ['ocr', '--image', imagePath, '--lang', language];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE ocr离线识别失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 通用 OCR 接口 - 自动选择在线或离线模式
  ipcMain.handle('tke-ocr', async (event, options) => {
    try {
      const { imagePath, mode = 'online', apiUrl, language = 'eng' } = options;

      if (!imagePath || typeof imagePath !== 'string') {
        return { success: false, error: '请提供图像路径' };
      }

      if (!fs.existsSync(imagePath)) {
        return { success: false, error: '图像文件不存在' };
      }

      let args = ['ocr', '--image', imagePath];

      // 根据模式添加不同的参数
      if (mode === 'online') {
        if (!apiUrl) {
          return { success: false, error: '在线模式需要提供OCR API URL' };
        }
        args.push('--online', '--url', apiUrl);
      } else if (mode === 'offline') {
        args.push('--lang', language);
      } else {
        return { success: false, error: '无效的OCR模式，请使用 "online" 或 "offline"' };
      }

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE ocr识别失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerOcrHandlers
};
