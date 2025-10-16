// 项目管理相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { promisify } = require('util');

const writeFile = promisify(fs.writeFile);
const mkdir = promisify(fs.mkdir);

// 注册项目相关的IPC处理器
function registerProjectHandlers() {
  // 创建项目结构
  ipcMain.handle('create-project-structure', async (event, projectPath) => {
    try {
      if (!projectPath) {
        return { success: false, error: '项目路径不能为空' };
      }

      // 创建目录结构
      await mkdir(path.join(projectPath, 'cases'), { recursive: true });
      await mkdir(path.join(projectPath, 'devices'), { recursive: true });
      await mkdir(path.join(projectPath, 'workarea'), { recursive: true });
      await mkdir(path.join(projectPath, 'locator'), { recursive: true });
      await mkdir(path.join(projectPath, 'locator', 'img'), { recursive: true });
      
      // 创建初始文件
      await writeFile(path.join(projectPath, 'testcase_map.json'), '{}', 'utf-8');
      await writeFile(path.join(projectPath, 'testcase_sheet.csv'), '', 'utf-8');
      await writeFile(path.join(projectPath, 'locator', 'element.json'), '{}', 'utf-8');
      
      // 创建README文件
      await writeFile(
        path.join(projectPath, 'workarea', 'README.md'),
        '# Workarea\n\n此文件夹用于存放临时文件，如当前截图和UI树XML文件。',
        'utf-8'
      );
      
      await writeFile(
        path.join(projectPath, 'locator', 'README.md'),
        '# Locator\n\n此文件夹用于存放所有测试用例共享的元素定位信息。\n- element.json: XML元素定位信息\n- img/: 图像识别元素定位截图',
        'utf-8'
      );
      
      console.log('项目结构已创建于:', projectPath);
      return { success: true };
    } catch (error) {
      console.error('创建项目结构失败:', error);
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