// Logcat相关的IPC处理器
const { ipcMain } = require('electron');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath, buildTkeAdbCommand } = require('./adb-handlers');

// 全局变量存储logcat进程
const logcatProcesses = new Map();
const logcatBuffers = new Map();


// 注册Logcat相关的IPC处理器
function registerLogcatHandlers(app, mainWindow) {
  // 启动logcat
  ipcMain.handle('start-logcat', async (event, options) => {
    try {
      const { device, format = 'long', buffer = 'all' } = options;
      const tkePath = getTkePath(app);
      
      // 如果该设备已有logcat进程在运行，先停止它
      if (logcatProcesses.has(device)) {
        const oldProcess = logcatProcesses.get(device);
        oldProcess.kill();
        logcatProcesses.delete(device);
      }
      
      // 构建 tke adb logcat 命令
      const command = tkePath;
      let args = ['--device', device, 'adb', 'logcat'];
      
      // 添加格式参数
      if (format === 'long') {
        args.push('-v', 'long');
      } else {
        args.push('-v', format);
      }
      
      // 添加buffer参数
      if (buffer && buffer !== 'main') {
        args.push('-b', buffer);
      }
      
      // 设置spawn选项
      const spawnOptions = {
        stdio: ['ignore', 'pipe', 'pipe']
      };
      
      // Windows平台特殊处理
      if (process.platform === 'win32') {
        spawnOptions.shell = false;
        console.log('Windows: Starting logcat with args:', args.join(' '));
      }
      
      console.log('Starting logcat process:', command, args.join(' '));
      const logcatProcess = spawn(command, args, spawnOptions);
      
      // 存储进程
      logcatProcesses.set(device, logcatProcess);
      
      // 监听进程错误
      logcatProcess.on('error', (error) => {
        console.error('Failed to start Logcat process:', error);
        logcatProcesses.delete(device);
      });
      
      // 初始化该设备的缓冲区
      if (!logcatBuffers.has(device)) {
        logcatBuffers.set(device, '');
      }
      
      // 监听输出
      logcatProcess.stdout.on('data', (data) => {
        let output;
        
        // 统一使用UTF-8编码
        try {
          output = data.toString('utf8');
          
          // Windows平台可能需要额外的编码处理
          if (process.platform === 'win32') {
            // 检查是否有明显的乱码字符
            const hasEncodingIssue = output.includes('\ufffd') || /[\x80-\xFF]{3,}/.test(output);
            
            if (hasEncodingIssue) {
              console.warn('Detected encoding issue, attempting to fix...');
              // 尝试使用iconv-lite转换
              try {
                const iconv = require('iconv-lite');
                // 尝试GBK编码
                const decoded = iconv.decode(data, 'cp936');
                if (!decoded.includes('\ufffd')) {
                  output = decoded;
                  console.log('Successfully decoded using GBK/CP936');
                }
              } catch (e) {
                console.warn('iconv-lite not available or decoding failed');
              }
            }
            
            // 清理控制字符
            output = output.replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
          }
        } catch (error) {
          console.error('Error decoding logcat output:', error);
          output = data.toString('ascii').replace(/[\x80-\xFF]/g, '?');
        }
        
        // 处理数据分片问题
        const currentBuffer = logcatBuffers.get(device) + output;
        const lines = currentBuffer.split('\n');
        
        // 保留最后一行（可能不完整）
        const incompleteLastLine = lines.pop();
        logcatBuffers.set(device, incompleteLastLine || '');
        
        // 发送完整的行
        if (lines.length > 0) {
          const completeOutput = lines.join('\n') + '\n';
          mainWindow.webContents.send('logcat-data', completeOutput);
        }
      });
      
      logcatProcess.stderr.on('data', (data) => {
        const errorOutput = data.toString('utf8');
        console.error('Logcat error:', errorOutput);
      });
      
      logcatProcess.on('close', (code) => {
        console.log(`Logcat process exited with code ${code}`);
        
        // 清理缓冲区中残留的不完整行
        const remainingBuffer = logcatBuffers.get(device);
        if (remainingBuffer && remainingBuffer.trim()) {
          console.log('Sending last line from buffer:', remainingBuffer);
          mainWindow.webContents.send('logcat-data', remainingBuffer + '\n');
        }
        
        logcatProcesses.delete(device);
        logcatBuffers.delete(device);
      });
      
      return { success: true, pid: logcatProcess.pid };
      
    } catch (error) {
      console.error('Failed to start logcat:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 停止logcat
  ipcMain.handle('stop-logcat', async (event, processId) => {
    try {
      // 遍历所有logcat进程
      for (const [device, process] of logcatProcesses) {
        if (process.pid === processId) {
          process.kill();
          logcatProcesses.delete(device);
          logcatBuffers.delete(device); // 同时清理缓冲区
          return { success: true };
        }
      }
      
      return { success: false, error: 'Process not found' };
    } catch (error) {
      console.error('Failed to stop logcat:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取logcat缓冲区内容
  ipcMain.handle('get-logcat-buffer', async (event, device) => {
    try {
      const buffer = logcatBuffers.get(device) || '';
      return {
        success: true,
        buffer: buffer,
        device: device
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 清除logcat缓冲区
  ipcMain.handle('clear-logcat-buffer', async (event, device) => {
    try {
      logcatBuffers.set(device, '');
      return {
        success: true,
        device: device
      };
    } catch (error) {
      return {
        success: false,
        error: error.message
      };
    }
  });
}

module.exports = {
  registerLogcatHandlers
};