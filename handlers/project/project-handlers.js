// 项目管理相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { promisify } = require('util');
const { createProjectDatabase } = require('../database/db-core');

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
            await mkdir(path.join(projectPath, 'workarea'), { recursive: true });
            await mkdir(path.join(projectPath, 'img'), { recursive: true });

            // 创建并初始化 SQLite 数据库
            createProjectDatabase(projectPath);

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
