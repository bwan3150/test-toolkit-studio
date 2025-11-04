// 视频流截图处理器
// 负责保存从视频流中捕获的截图到项目 workarea

const { ipcMain } = require('electron');
const fs = require('fs');
const path = require('path');

/**
 * 注册视频流截图相关的 IPC 处理器
 * @param {Object} app - Electron app 实例
 */
function registerStreamCaptureHandlers(app) {
  // 保存从视频流捕获的截图
  ipcMain.handle('save-screenshot-from-stream', async (event, { imageBuffer }) => {
    try {
      // 获取当前项目路径
      const projectPath = global.currentProjectPath;
      if (!projectPath) {
        throw new Error('未设置项目路径');
      }

      // 确保 workarea 目录存在
      const workareaPath = path.join(projectPath, 'workarea');
      if (!fs.existsSync(workareaPath)) {
        fs.mkdirSync(workareaPath, { recursive: true });
      }

      // 保存截图
      const screenshotPath = path.join(workareaPath, 'current_screenshot.png');
      fs.writeFileSync(screenshotPath, imageBuffer);

      console.log('✅ 视频流截图已保存:', screenshotPath);

      return {
        success: true,
        path: screenshotPath
      };
    } catch (error) {
      console.error('保存视频流截图失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('✅ stream-capture handlers 已注册');
}

module.exports = {
  registerStreamCaptureHandlers
};
