// TKE Recognizer 模块的 IPC 处理器
// 负责元素识别：通过文本、XML、图像查找元素
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

// 注册 TKE Recognizer 相关的 IPC 处理器
function registerRecognizerHandlers(app) {
  // 通过文本查找元素 - tke recognizer find-text "text"
  ipcMain.handle('tke-recognizer-find-text', async (event, deviceId, projectPath, text) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      if (!text || typeof text !== 'string') {
        return { success: false, error: '请提供要查找的文本' };
      }

      const args = ['--device', deviceId, '--project', projectPath, 'recognizer', 'find-text', `"${text}"`];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE recognizer find-text失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 通过XML元素名查找 - tke recognizer find-xml <element_name>
  ipcMain.handle('tke-recognizer-find-xml', async (event, deviceId, projectPath, elementName) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      if (!elementName || typeof elementName !== 'string') {
        return { success: false, error: '请提供元素名称' };
      }

      const args = ['--device', deviceId, '--project', projectPath, 'recognizer', 'find-xml', elementName];
      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE recognizer find-xml失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 通过图像查找元素 - tke recognizer find-image <image_name> [--threshold <value>]
  ipcMain.handle('tke-recognizer-find-image', async (event, deviceId, projectPath, imageName, threshold = 0.5) => {
    try {
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      if (!projectPath || !path.isAbsolute(projectPath)) {
        return { success: false, error: '请提供有效的项目路径（绝对路径）' };
      }

      if (!imageName || typeof imageName !== 'string') {
        return { success: false, error: '请提供图像名称' };
      }

      const args = ['--device', deviceId, '--project', projectPath, 'recognizer', 'find-image', imageName];

      // 如果提供了阈值，添加到参数中
      if (threshold !== 0.5) {
        args.push('--threshold', threshold.toString());
      }

      const output = await execTkeCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE recognizer find-image失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerRecognizerHandlers
};
