// TKE File 模块的 IPC 处理器
// 负责 Android 设备文件系统管理：浏览、搜索、增删改查等
const { ipcMain } = require('electron');
const { execFile } = require('child_process');
const path = require('path');
const fs = require('fs');
const { getTkePath } = require('./adb-handlers');

// 通用的 TKE File 命令执行函数
async function execTkeFileCommand(app, args) {
  const tkePath = getTkePath(app);

  if (!fs.existsSync(tkePath)) {
    throw new Error('TKE可执行文件未找到');
  }

  console.log('执行TKE File命令:', tkePath, args.join(' '));

  return new Promise((resolve, reject) => {
    const child = execFile(tkePath, args);

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
        const error = new Error(`TKE File命令失败 (exit code ${code}): ${stderr}`);
        error.code = code;
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
      } else {
        if (stderr && !stderr.includes('INFO')) {
          console.warn('TKE File命令警告:', stderr);
        }

        resolve(stdout); // 返回原始输出
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

// 注册 TKE File 相关的 IPC 处理器
function registerFileHandlers(app) {
  // 列出目录内容 (树形结构) - tke file ls <path> -L <level>
  ipcMain.handle('tke-file-ls', async (event, { path: filePath, level = 1, deviceId = null }) => {
    try {
      const args = ['file', 'ls'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(filePath || '/sdcard/');
      args.push('-L', String(level));

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file ls 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 搜索文件 - tke file find <path> <pattern>
  ipcMain.handle('tke-file-find', async (event, { path: searchPath, pattern, deviceId = null }) => {
    try {
      const args = ['file', 'find'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(searchPath, pattern);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file find 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 读取文件内容 - tke file cat <path>
  ipcMain.handle('tke-file-cat', async (event, { path: filePath, deviceId = null }) => {
    try {
      const args = ['file', 'cat'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(filePath);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        content: output
      };
    } catch (error) {
      console.error('TKE file cat 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 获取目录大小 - tke file size <path>
  ipcMain.handle('tke-file-size', async (event, { path: filePath, deviceId = null }) => {
    try {
      const args = ['file', 'size'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(filePath);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file size 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 创建目录 - tke file mkdir <path>
  ipcMain.handle('tke-file-mkdir', async (event, { path: dirPath, deviceId = null }) => {
    try {
      const args = ['file', 'mkdir'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(dirPath);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file mkdir 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 删除文件/目录 - tke file rm <path>
  ipcMain.handle('tke-file-rm', async (event, { path: filePath, deviceId = null }) => {
    try {
      const args = ['file', 'rm'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(filePath);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file rm 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 复制文件/目录 - tke file cp <source> <dest>
  ipcMain.handle('tke-file-cp', async (event, { source, dest, deviceId = null }) => {
    try {
      const args = ['file', 'cp'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(source, dest);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file cp 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 移动/重命名文件 - tke file mv <source> <dest>
  ipcMain.handle('tke-file-mv', async (event, { source, dest, deviceId = null }) => {
    try {
      const args = ['file', 'mv'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(source, dest);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file mv 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 写入文件 - tke file write <path> <content>
  ipcMain.handle('tke-file-write', async (event, { path: filePath, content, deviceId = null }) => {
    try {
      const args = ['file', 'write'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(filePath, content);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file write 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 下载文件到本地 - tke file pull <remote> <local>
  ipcMain.handle('tke-file-pull', async (event, { remote, local, deviceId = null }) => {
    try {
      const args = ['file', 'pull'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(remote, local);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file pull 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 上传文件到设备 - tke file push <local> <remote>
  ipcMain.handle('tke-file-push', async (event, { local, remote, deviceId = null }) => {
    try {
      const args = ['file', 'push'];

      if (deviceId) {
        args.unshift('--device', deviceId);
      }

      args.push(local, remote);

      const output = await execTkeFileCommand(app, args);

      return {
        success: true,
        output: output
      };
    } catch (error) {
      console.error('TKE file push 失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  console.log('TKE File handlers 已注册');
}

module.exports = {
  registerFileHandlers
};
