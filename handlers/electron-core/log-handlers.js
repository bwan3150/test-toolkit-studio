// 日志相关的IPC处理器
const { ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const Store = require('electron-store');
const FormData = require('form-data');
const axios = require('axios');
const { getTkePath } = require('../tke-integration/adb-handlers');

// 初始化store
const store = new Store();

// 注册日志相关的IPC处理器
function registerLogHandlers(app) {
  // 导出日志处理（保留原有功能）
  ipcMain.handle('export-logs', async (event, logData) => {
    try {
      // 生成默认文件名
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const defaultFileName = `toolkit-studio-${timestamp}`;
      
      // 显示保存对话框
      const result = await dialog.showSaveDialog({
        title: 'Export Log File',
        defaultPath: path.join(app.getPath('desktop'), defaultFileName),
        filters: [
          { name: 'Log Files', extensions: ['log'] },
          { name: 'Text Files', extensions: ['txt'] },
          { name: 'All Files', extensions: ['*'] }
        ]
      });
      
      if (result.canceled || !result.filePath) {
        return { success: false, message: 'Cancelled' };
      }
      
      // 格式化日志内容
      const logContent = formatLogContent(logData, app);
      
      // 写入文件
      await fs.promises.writeFile(result.filePath, logContent, 'utf8');
      
      return { 
        success: true, 
        message: `日志已导出到: ${result.filePath}`,
        filePath: result.filePath
      };
    } catch (error) {
      console.error('导出日志失败:', error);
      return { 
        success: false, 
        message: `导出失败: ${error.message}` 
      };
    }
  });

  // 上传日志处理
  ipcMain.handle('upload-logs', async (event, logData) => {
    try {
      // 获取认证信息
      const accessToken = store.get('access_token');
      const baseUrl = store.get('base_url');
      
      if (!accessToken) {
        return { 
          success: false, 
          message: '请先登录' 
        };
      }
      
      if (!baseUrl) {
        return { 
          success: false, 
          message: '未配置API地址' 
        };
      }
      
      // 生成日志文件名 - 使用平台名称
      const fileName = `${process.platform}.log`;
      
      // 格式化日志内容
      const logContent = formatLogContent(logData, app);
      
      // 创建 FormData
      const form = new FormData();
      form.append('app', 'Toolkit_Studio');
      form.append('log', Buffer.from(logContent), {
        filename: fileName,
        contentType: 'text/plain'
      });
      
      // 上传日志
      const response = await axios.post(`${baseUrl}/api/logs/upload`, form, {
        headers: {
          ...form.getHeaders(),
          'Authorization': `Bearer ${accessToken}`
        }
      });
      
      if (response.data) {
        console.log('日志上传成功:', response.data);
        return { 
          success: true, 
          message: '日志上传成功',
          data: response.data
        };
      } else {
        return { 
          success: false, 
          message: '上传失败: 服务器响应异常' 
        };
      }
      
    } catch (error) {
      console.error('上传日志失败:', error);
      
      // 处理不同的错误情况
      if (error.response) {
        if (error.response.status === 401) {
          return { 
            success: false, 
            message: '认证失败，请重新登录' 
          };
        }
        return { 
          success: false, 
          message: `上传失败: ${error.response.data?.message || error.response.statusText}` 
        };
      } else if (error.request) {
        return { 
          success: false, 
          message: '上传失败: 网络连接错误' 
        };
      } else {
        return { 
          success: false, 
          message: `上传失败: ${error.message}` 
        };
      }
    }
  });
}

// 格式化日志内容的通用函数
function formatLogContent(logData, app) {
  // 获取时区信息
  const date = new Date();
  
  // 尝试获取 IANA 时区名称
  let timeZoneInfo;
  try {
    // 使用 Intl API 获取时区
    const timeZoneName = Intl.DateTimeFormat().resolvedOptions().timeZone;
    
    // 同时计算 UTC 偏移
    const timeZoneOffset = date.getTimezoneOffset();
    const offsetHours = Math.floor(Math.abs(timeZoneOffset) / 60);
    const offsetMinutes = Math.abs(timeZoneOffset) % 60;
    const offsetSign = timeZoneOffset <= 0 ? '+' : '-';
    const utcOffset = `UTC${offsetSign}${offsetHours.toString().padStart(2, '0')}:${offsetMinutes.toString().padStart(2, '0')}`;
    
    timeZoneInfo = `${timeZoneName} (${utcOffset})`;
  } catch (e) {
    // 如果 Intl API 不可用，降级到只显示 UTC 偏移
    const timeZoneOffset = date.getTimezoneOffset();
    const offsetHours = Math.floor(Math.abs(timeZoneOffset) / 60);
    const offsetMinutes = Math.abs(timeZoneOffset) % 60;
    const offsetSign = timeZoneOffset <= 0 ? '+' : '-';
    timeZoneInfo = `UTC${offsetSign}${offsetHours.toString().padStart(2, '0')}:${offsetMinutes.toString().padStart(2, '0')}`;
  }
  
  let logContent = '=== Toolkit Studio Log ===\n';
  logContent += `时间: ${date.toLocaleString()}\n`;
  logContent += `时区: ${timeZoneInfo}\n`;
  logContent += `应用版本: ${app.getVersion()}\n`;
  logContent += `平台: ${logData.platform}\n`;
  logContent += `Node版本: ${logData.nodeVersion}\n`;
  logContent += `Electron版本: ${logData.electronVersion}\n`;
  logContent += '================================\n\n';
  
  // 添加系统信息
  logContent += '=== 系统信息 ===\n';
  logContent += `process.resourcesPath (renderer): ${logData.resourcesPath}\n`;
  logContent += `process.resourcesPath (main): ${process.resourcesPath}\n`;
  logContent += `app.isPackaged: ${app.isPackaged}\n`;
  logContent += `app.getPath('exe'): ${app.getPath('exe')}\n`;
  logContent += `app.getAppPath(): ${app.getAppPath()}\n`;
  logContent += `__dirname (renderer): ${logData.dirname}\n`;
  logContent += `__dirname (main): ${__dirname}\n`;
  logContent += `TKE路径: ${getTkePath(app)}\n`;
  logContent += '================\n\n';
  
  // 添加日志内容
  logContent += '=== 日志内容 ===\n';
  if (logData.logs && Array.isArray(logData.logs)) {
    logData.logs.forEach(entry => {
      const levelTag = entry.level.toUpperCase().padEnd(5);
      logContent += `[${entry.timestamp}] [${levelTag}] ${entry.message}\n`;
    });
    
    if (logData.logs.length === 0) {
      logContent += '(暂无日志)\n';
    }
  } else {
    logContent += '(日志数据格式错误)\n';
  }
  
  return logContent;
}

module.exports = {
  registerLogHandlers
};