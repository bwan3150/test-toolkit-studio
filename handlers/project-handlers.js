// 项目管理相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

// 注册项目相关的IPC处理器
function registerProjectHandlers() {
  // 创建项目结构
  ipcMain.handle('create-project-structure', async (event, projectPath) => {
    try {
      if (!projectPath || !fs.existsSync(projectPath)) {
        return { success: false, error: '项目路径不存在' };
      }

      const dirs = ['testcases', 'scripts', 'reports', 'temp', 'data'];
      for (const dir of dirs) {
        const dirPath = path.join(projectPath, dir);
        if (!fs.existsSync(dirPath)) {
          fs.mkdirSync(dirPath, { recursive: true });
        }
      }

      return { success: true, path: projectPath };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 保存文件
  ipcMain.handle('save-file', async (event, filepath, content) => {
    try {
      const dir = path.dirname(filepath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
      fs.writeFileSync(filepath, content, 'utf8');
      return { success: true, path: filepath };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerProjectHandlers
};