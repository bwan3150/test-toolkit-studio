// AAPT相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');

// 获取TKE可执行文件路径
function getTkePath(app) {
  const platform = process.platform;
  const tkeBinaryName = platform === 'win32' ? 'tke.exe' : 'tke';

  if (app.isPackaged) {
    // 生产模式：process.resourcesPath/[platform]/toolkit-engine/tke
    return path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinaryName);
  } else {
    // 开发模式：resources/[platform]/toolkit-engine/tke
    return path.join(app.getAppPath(), 'resources', platform, 'toolkit-engine', tkeBinaryName);
  }
}

// 辅助函数：执行 TKE AAPT 命令
async function execTkeAaptCommand(app, aaptArgs) {
  const tkePath = getTkePath(app);
  if (!fs.existsSync(tkePath)) {
    throw new Error('Toolkit Engine未找到');
  }

  // 构建参数数组：tke aapt [args...]
  const args = ['aapt'];

  if (Array.isArray(aaptArgs)) {
    args.push(...aaptArgs);
  } else {
    args.push(aaptArgs);
  }

  // 使用spawn执行命令
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
        const error = new Error(`TKE AAPT命令失败 (exit code ${code})`);
        error.code = code;
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
      } else {
        resolve({ stdout, stderr });
      }
    });

    child.on('error', (err) => {
      reject(err);
    });
  });
}

// 注册所有AAPT相关的IPC处理器
function registerAaptHandlers(app) {
  // 获取APK包名 - 使用TKE内置AAPT
  ipcMain.handle('get-apk-package-name', async (event, apkPath) => {
    try {
      console.log('开始获取APK包名（使用TKE AAPT），文件路径:', apkPath);

      if (!apkPath || !fs.existsSync(apkPath)) {
        console.error('APK文件不存在:', apkPath);
        return { success: false, error: 'APK文件不存在' };
      }

      console.log('APK文件存在，使用TKE AAPT解析包名');

      try {
        // 使用 tke aapt dump badging <apk_path>
        console.log('执行 tke aapt dump badging 命令');
        const { stdout } = await execTkeAaptCommand(app, ['dump', 'badging', apkPath]);
        console.log('TKE AAPT输出前100字符:', stdout.substring(0, 100));

        const packageMatch = stdout.match(/package:\s+name='([^']+)'/);
        if (packageMatch && packageMatch[1]) {
          const packageName = packageMatch[1];
          console.log('通过TKE AAPT获取到包名:', packageName);
          return { success: true, packageName };
        } else {
          console.log('TKE AAPT输出中未找到包名匹配');
          return { success: false, error: 'AAPT输出中未找到包名信息' };
        }
      } catch (error) {
        console.error('TKE AAPT方法失败:', error.message);
        return { success: false, error: `AAPT执行失败: ${error.message}` };
      }

    } catch (error) {
      console.error('获取APK包名失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 执行通用AAPT命令
  ipcMain.handle('execute-aapt-command', async (event, args) => {
    try {
      console.log('执行TKE AAPT命令:', args);

      const { stdout, stderr } = await execTkeAaptCommand(app, args);

      return {
        success: true,
        output: stdout,
        error: stderr
      };
    } catch (error) {
      console.error('执行TKE AAPT命令失败:', error);
      return {
        success: false,
        error: error.message,
        stdout: error.stdout || '',
        stderr: error.stderr || ''
      };
    }
  });

  // 获取APK详细信息
  ipcMain.handle('get-apk-info', async (event, apkPath) => {
    try {
      if (!apkPath || !fs.existsSync(apkPath)) {
        return { success: false, error: 'APK文件不存在' };
      }

      console.log('获取APK详细信息:', apkPath);

      // 使用 tke aapt dump badging 获取完整信息
      const { stdout } = await execTkeAaptCommand(app, ['dump', 'badging', apkPath]);

      // 解析输出
      const info = {};

      // 包名
      const packageMatch = stdout.match(/package:\s+name='([^']+)'/);
      if (packageMatch) info.packageName = packageMatch[1];

      // 版本名
      const versionNameMatch = stdout.match(/versionName='([^']+)'/);
      if (versionNameMatch) info.versionName = versionNameMatch[1];

      // 版本号
      const versionCodeMatch = stdout.match(/versionCode='([^']+)'/);
      if (versionCodeMatch) info.versionCode = versionCodeMatch[1];

      // 应用标签
      const labelMatch = stdout.match(/application-label:'([^']+)'/);
      if (labelMatch) info.label = labelMatch[1];

      // SDK版本
      const minSdkMatch = stdout.match(/sdkVersion:'([^']+)'/);
      if (minSdkMatch) info.minSdkVersion = minSdkMatch[1];

      const targetSdkMatch = stdout.match(/targetSdkVersion:'([^']+)'/);
      if (targetSdkMatch) info.targetSdkVersion = targetSdkMatch[1];

      console.log('APK信息:', info);

      return {
        success: true,
        info: info,
        rawOutput: stdout
      };

    } catch (error) {
      console.error('获取APK信息失败:', error);
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerAaptHandlers,
  execTkeAaptCommand
};
