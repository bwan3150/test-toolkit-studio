// ADB相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { exec } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// 获取内置ADB路径的辅助函数
function getBuiltInAdbPath(app) {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';
  
  if (app.isPackaged) {
    return path.join(process.resourcesPath, platform, 'android-sdk', 'platform-tools', adbName);
  } else {
    return path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'platform-tools', adbName);
  }
}

// 获取内置Scrcpy路径的辅助函数
function getBuiltInScrcpyPath(app) {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const scrcpyName = process.platform === 'win32' ? 'scrcpy.exe' : 'scrcpy';
  
  if (app.isPackaged) {
    return path.join(process.resourcesPath, platform, 'scrcpy', scrcpyName);
  } else {
    return path.join(__dirname, '..', 'resources', platform, 'scrcpy', scrcpyName);
  }
}

// 获取内置STB路径的辅助函数
function getBuiltInStbPath(app) {
  const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
  const stbName = process.platform === 'win32' ? 'stb.exe' : 'stb';
  
  if (app.isPackaged) {
    return path.join(process.resourcesPath, platform, 'stb', stbName);
  } else {
    return path.join(__dirname, '..', 'resources', platform, 'stb', stbName);
  }
}

// 注册所有ADB相关的IPC处理器
function registerAdbHandlers(app) {
  // 获取连接的设备列表
  ipcMain.handle('get-devices', async () => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, devices: [], error: '内置Android SDK未找到' };
      }
      
      const { stdout, stderr } = await execPromise(`"${adbPath}" devices -l`);
      
      if (stderr && !stderr.includes('daemon')) {
        console.error('ADB错误:', stderr);
      }
      
      const lines = stdout.split('\n');
      const devices = [];
      
      for (const line of lines) {
        if (line.includes('device ') && !line.startsWith('List of devices')) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 2) {
            const deviceId = parts[0];
            let deviceInfo = { id: deviceId, status: 'device' };
            
            // 提取设备信息
            const modelMatch = line.match(/model:([^\s]+)/);
            const deviceMatch = line.match(/device:([^\s]+)/);
            const productMatch = line.match(/product:([^\s]+)/);
            const transportMatch = line.match(/transport_id:([^\s]+)/);
            
            if (modelMatch) deviceInfo.model = modelMatch[1];
            if (deviceMatch) deviceInfo.device = deviceMatch[1];
            if (productMatch) deviceInfo.product = productMatch[1];
            if (transportMatch) deviceInfo.transportId = transportMatch[1];
            
            // 判断设备类型
            if (deviceId.includes(':')) {
              deviceInfo.type = 'wireless';
            } else {
              deviceInfo.type = 'usb';
            }
            
            devices.push(deviceInfo);
          }
        }
      }
      
      return { success: true, devices };
    } catch (error) {
      console.error('获取设备列表失败:', error);
      return { success: false, devices: [], error: error.message };
    }
  });

  // 执行ADB Shell命令
  ipcMain.handle('adb-shell-command', async (event, command, deviceId = null) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      const deviceArg = deviceId ? `-s ${deviceId}` : '';
      const fullCommand = `"${adbPath}" ${deviceArg} shell ${command}`;
      
      console.log('执行ADB Shell命令:', fullCommand);
      
      const { stdout, stderr } = await execPromise(fullCommand);
      
      if (stderr && !stderr.includes('Warning')) {
        console.warn('ADB命令警告:', stderr);
      }
      
      return { 
        success: true, 
        output: stdout,
        error: stderr
      };
    } catch (error) {
      console.error('ADB Shell命令执行失败:', error);
      return { 
        success: false, 
        error: error.message,
        output: error.stdout || '',
        stderr: error.stderr || ''
      };
    }
  });

  // APK安装功能
  ipcMain.handle('adb-install-apk', async (event, deviceId, apkPath, forceReinstall = false) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !apkPath) {
        return { success: false, error: '无效的设备ID或APK路径' };
      }
      
      // 检查APK文件是否存在
      if (!fs.existsSync(apkPath)) {
        return { success: false, error: 'APK文件不存在' };
      }
      
      console.log('正在安装APK到设备:', deviceId, apkPath);
      
      // 构建安装命令
      let installFlags = '-r'; // 替换安装
      if (forceReinstall) {
        installFlags += ' -d'; // 允许降级
      }
      installFlags += ' -g'; // 授予权限
      
      const installCommand = `"${adbPath}" -s ${deviceId} install ${installFlags} "${apkPath}"`;
      console.log('执行安装命令:', installCommand);
      
      const { stdout, stderr } = await execPromise(installCommand);
      
      // 合并输出以便检查
      const fullOutput = stdout + '\n' + stderr;
      
      // 检查安装结果
      if (fullOutput.includes('Success')) {
        console.log('APK安装成功');
        return { 
          success: true, 
          message: 'APK安装成功',
          output: stdout
        };
      } else if (fullOutput.includes('Failure') || fullOutput.includes('Failed') || fullOutput.includes('INSTALL_FAILED')) {
        // 解析错误原因
        let errorMsg = '安装失败';
        let packageName = null;
        
        if (fullOutput.includes('INSTALL_FAILED_ALREADY_EXISTS')) {
          errorMsg = '应用已存在，请先卸载后重试';
        } else if (fullOutput.includes('INSTALL_FAILED_UPDATE_INCOMPATIBLE')) {
          // 从错误信息中提取包名
          const packageMatch = fullOutput.match(/package\s+([a-zA-Z0-9._]+)/i);
          if (packageMatch) {
            packageName = packageMatch[1];
          }
          errorMsg = `签名不匹配: ${packageName || '未知包名'} 调试版和发布版签名不同，需要先卸载`;
        } else if (fullOutput.includes('INSTALL_FAILED_VERSION_DOWNGRADE')) {
          errorMsg = '不允许降级，请使用强制安装选项';
        } else if (fullOutput.includes('INSTALL_FAILED_INSUFFICIENT_STORAGE')) {
          errorMsg = '设备存储空间不足';
        } else if (fullOutput.includes('INSTALL_FAILED_INVALID_APK')) {
          errorMsg = 'APK文件无效或损坏';
        }
        
        console.error('APK安装失败:', fullOutput);
        return { 
          success: false, 
          error: errorMsg,
          details: fullOutput,
          packageName: packageName
        };
      } else {
        // 未知结果，可能成功
        return { 
          success: true, 
          message: '安装命令已执行',
          output: stdout
        };
      }
      
    } catch (error) {
      console.error('安装APK失败:', error);
      
      // 检查错误信息中是否包含签名不匹配
      const errorOutput = (error.stdout || '') + (error.stderr || '') + error.message;
      console.log('完整错误输出:', errorOutput);
      
      // 从错误信息中提取包名
      let extractedPackageName = null;
      const packagePatterns = [
        /INSTALL_FAILED_UPDATE_INCOMPATIBLE:\s*Existing\s+package\s+([a-zA-Z0-9._]+)/,
        /Existing\s+package\s+([a-zA-Z0-9._]+)\s+signatures/,
        /package\s+([a-zA-Z0-9._]+)\s+signatures\s+do\s+not\s+match/,
        /Package\s+([a-zA-Z0-9._]+)\s+signatures/
      ];
      
      for (const pattern of packagePatterns) {
        const match = errorOutput.match(pattern);
        if (match && match[1]) {
          extractedPackageName = match[1];
          console.log('从错误中提取到包名:', extractedPackageName);
          break;
        }
      }
      
      if (errorOutput.includes('INSTALL_FAILED_UPDATE_INCOMPATIBLE') || errorOutput.includes('signatures do not match')) {
        return { 
          success: false, 
          error: '签名不匹配错误',
          needUninstall: true,
          packageName: extractedPackageName,
          details: errorOutput
        };
      }
      
      return { 
        success: false, 
        error: error.message,
        packageName: extractedPackageName
      };
    }
  });

  // 获取APK包名
  ipcMain.handle('get-apk-package-name', async (event, apkPath) => {
    try {
      console.log('开始获取APK包名，文件路径:', apkPath);
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        console.error('ADB路径不存在:', adbPath);
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!apkPath || !fs.existsSync(apkPath)) {
        console.error('APK文件不存在:', apkPath);
        return { success: false, error: 'APK文件不存在' };
      }
      
      console.log('APK文件存在，开始解析包名');
      
      // 方法1：尝试通过aapt获取（如果有的话）
      const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
      const aaptName = process.platform === 'win32' ? 'aapt.exe' : 'aapt';
      let aaptPath;
      
      if (app.isPackaged) {
        aaptPath = path.join(process.resourcesPath, platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
      } else {
        aaptPath = path.join(__dirname, '..', 'resources', platform, 'android-sdk', 'build-tools', '33.0.2', aaptName);
      }
      
      console.log('尝试aapt路径:', aaptPath);
      console.log('aapt文件是否存在:', fs.existsSync(aaptPath));
      
      // 尝试使用aapt
      if (fs.existsSync(aaptPath)) {
        try {
          console.log('使用aapt获取APK包名');
          const aaptCommand = `"${aaptPath}" dump badging "${apkPath}"`;
          console.log('执行aapt命令:', aaptCommand);
          
          const { stdout } = await execPromise(aaptCommand);
          console.log('aapt输出前100字符:', stdout.substring(0, 100));
          
          const packageMatch = stdout.match(/package:\s+name='([^']+)'/);
          if (packageMatch && packageMatch[1]) {
            const packageName = packageMatch[1];
            console.log('通过aapt获取到包名:', packageName);
            return { success: true, packageName };
          } else {
            console.log('aapt输出中未找到包名匹配');
            return { success: false, error: 'aapt输出中未找到包名信息' };
          }
        } catch (error) {
          console.error('aapt方法失败:', error.message);
          return { success: false, error: `aapt执行失败: ${error.message}` };
        }
      } else {
        console.log('aapt工具未找到，将尝试直接安装而不需要包名');
        
        // 如果aapt不可用，返回特殊标记，让安装流程继续
        // 不需要预先获取包名，直接尝试安装
        return { 
          success: false, 
          error: 'aapt不可用', 
          needDirectInstall: true,
          message: '将尝试直接安装而不需要包名'
        };
      }
      
    } catch (error) {
      console.error('获取APK包名失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 卸载应用
  ipcMain.handle('adb-uninstall-app', async (event, deviceId, packageName) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !packageName) {
        return { success: false, error: '无效的设备ID或包名' };
      }
      
      console.log('卸载应用:', deviceId, packageName);
      
      const uninstallCommand = `"${adbPath}" -s ${deviceId} uninstall ${packageName}`;
      const { stdout, stderr } = await execPromise(uninstallCommand);
      
      if (stdout.includes('Success')) {
        return { success: true, message: '应用卸载成功' };
      } else if (stdout.includes('Failure')) {
        if (stdout.includes('DELETE_FAILED_INTERNAL_ERROR')) {
          return { success: false, error: '内部错误，无法卸载' };
        } else if (stdout.includes('Unknown package')) {
          return { success: false, error: '应用未安装' };
        }
        return { success: false, error: '卸载失败', details: stdout };
      }
      
      return { success: true, message: '卸载命令已执行', output: stdout };
      
    } catch (error) {
      console.error('卸载应用失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 无线配对功能
  ipcMain.handle('adb-pair-wireless', async (event, ipAddress, port, pairingCode) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!ipAddress || !port || !pairingCode) {
        return { success: false, error: '请提供IP地址、端口和配对码' };
      }
      
      console.log('开始ADB无线配对:', ipAddress, port);
      
      const pairCommand = `"${adbPath}" pair ${ipAddress}:${port} ${pairingCode}`;
      const { stdout, stderr } = await execPromise(pairCommand);
      
      console.log('配对输出:', stdout);
      if (stderr) console.log('配对错误:', stderr);
      
      if (stdout.includes('Successfully paired') || stdout.includes('成功配对')) {
        return { 
          success: true, 
          message: `成功配对到设备 ${ipAddress}:${port}`,
          output: stdout
        };
      } else if (stdout.includes('Failed') || (stderr && stderr.includes('failed'))) {
        return { 
          success: false, 
          error: `配对失败: ${stderr || stdout}` 
        };
      } else {
        // 某些情况下配对可能成功但没有明确的成功消息
        return { 
          success: true, 
          message: `配对命令已执行，请尝试连接设备`,
          output: stdout,
          warning: '配对状态不确定，请尝试连接'
        };
      }
      
    } catch (error) {
      console.error('ADB配对失败:', error);
      return { success: false, error: `配对异常: ${error.message}` };
    }
  });

  // 无线连接功能
  ipcMain.handle('adb-connect-wireless', async (event, ipAddress, port = 5555) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!ipAddress) {
        return { success: false, error: '请提供IP地址' };
      }
      
      const connectionAddress = `${ipAddress}:${port}`;
      console.log('连接到无线ADB设备:', connectionAddress);
      
      const { stdout, stderr } = await execPromise(`"${adbPath}" connect ${connectionAddress}`);
      
      if (stderr && stderr.includes('failed')) {
        return { success: false, error: `连接失败: ${stderr}` };
      }
      
      if (stdout.includes('connected to') || stdout.includes('already connected')) {
        return { 
          success: true, 
          message: `成功连接到设备 ${connectionAddress}`,
          deviceId: connectionAddress 
        };
      } else if (stdout.includes('unable to connect')) {
        return { 
          success: false, 
          error: `无法连接到 ${connectionAddress}，请确保设备已启用ADB调试并在同一网络` 
        };
      } else {
        return { 
          success: false, 
          error: `连接状态未知: ${stdout}` 
        };
      }
      
    } catch (error) {
      return { success: false, error: `连接异常: ${error.message}` };
    }
  });

  // 断开无线连接
  ipcMain.handle('adb-disconnect-wireless', async (event, ipAddress, port = 5555) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!ipAddress) {
        return { success: false, error: '请提供IP地址' };
      }
      
      const connectionAddress = `${ipAddress}:${port}`;
      console.log('断开无线ADB设备连接:', connectionAddress);
      
      const { stdout, stderr } = await execPromise(`"${adbPath}" disconnect ${connectionAddress}`);
      
      if (stderr && !stderr.includes('Warning')) {
        return { success: false, error: `断开连接失败: ${stderr}` };
      }
      
      return { 
        success: true, 
        message: `已断开设备 ${connectionAddress}`,
        output: stdout.trim()
      };
      
    } catch (error) {
      return { success: false, error: `断开连接异常: ${error.message}` };
    }
  });

  // 批量执行ADB命令
  ipcMain.handle('adb-batch-commands', async (event, commands, deviceId = null) => {
    const results = [];
    
    for (const command of commands) {
      const result = await ipcMain._events['adb-shell-command'][0](event, command, deviceId);
      results.push({
        command,
        ...result
      });
      
      // 如果某个命令失败，停止执行
      if (!result.success && command.required !== false) {
        break;
      }
    }
    
    return results;
  });

  // 获取当前运行的App信息
  ipcMain.handle('get-current-app', async (event, deviceId) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }
      
      let packageName = '';
      let activityName = '';
      
      // 方法1: 尝试获取顶层Activity
      try {
        const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell dumpsys activity top`);
        if (stdout) {
          const lines = stdout.split('\n');
          for (const line of lines) {
            if (line.includes('ACTIVITY')) {
              const matches = line.match(/([a-zA-Z0-9._]+)\/([a-zA-Z0-9._$]+)/g);
              if (matches && matches.length > 0) {
                const fullActivity = matches[0];
                if (!fullActivity.includes('android/') && !fullActivity.includes('system/') && !fullActivity.includes('com.android.')) {
                  const parts = fullActivity.split('/');
                  packageName = parts[0];
                  activityName = parts[1];
                  if (activityName.startsWith('.')) {
                    activityName = packageName + activityName;
                  }
                  break;
                }
              }
            }
          }
        }
      } catch (error) {
        // 继续尝试其他方法
      }
      
      if (packageName) {
        return { 
          success: true, 
          packageName: packageName,
          activityName: activityName
        };
      } else {
        return { 
          success: false, 
          error: '无法获取当前运行的应用信息' 
        };
      }
      
    } catch (error) {
      console.error('获取当前应用失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 启动应用
  ipcMain.handle('start-app', async (event, deviceId, packageName, activityName = null) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }
      
      let startCommand;
      if (activityName) {
        startCommand = `"${adbPath}" -s ${deviceId} shell am start -n ${packageName}/${activityName}`;
      } else {
        // 使用monkey命令启动应用
        startCommand = `"${adbPath}" -s ${deviceId} shell monkey -p ${packageName} -c android.intent.category.LAUNCHER 1`;
      }
      
      console.log('启动应用命令:', startCommand);
      const { stdout, stderr } = await execPromise(startCommand);
      
      if (stderr && !stderr.includes('Warning')) {
        return { success: false, error: stderr };
      }
      
      return { success: true, message: '应用启动成功', output: stdout };
      
    } catch (error) {
      console.error('启动应用失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 停止应用
  ipcMain.handle('stop-app', async (event, deviceId, packageName) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }
      
      const stopCommand = `"${adbPath}" -s ${deviceId} shell am force-stop ${packageName}`;
      console.log('停止应用命令:', stopCommand);
      
      const { stdout, stderr } = await execPromise(stopCommand);
      
      if (stderr && !stderr.includes('Warning')) {
        return { success: false, error: stderr };
      }
      
      return { success: true, message: '应用已停止', output: stdout };
      
    } catch (error) {
      console.error('停止应用失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 清除应用数据
  ipcMain.handle('clear-app-data', async (event, deviceId, packageName) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }
      
      const clearCommand = `"${adbPath}" -s ${deviceId} shell pm clear ${packageName}`;
      console.log('清除应用数据命令:', clearCommand);
      
      const { stdout, stderr } = await execPromise(clearCommand);
      
      if (stdout.includes('Success')) {
        return { success: true, message: '应用数据已清除' };
      } else if (stdout.includes('Failed')) {
        return { success: false, error: '清除数据失败', details: stdout };
      }
      
      return { success: true, message: '清除命令已执行', output: stdout };
      
    } catch (error) {
      console.error('清除应用数据失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 推送文件到设备
  ipcMain.handle('push-file', async (event, deviceId, localPath, remotePath) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !localPath || !remotePath) {
        return { success: false, error: '参数无效' };
      }
      
      if (!fs.existsSync(localPath)) {
        return { success: false, error: '本地文件不存在' };
      }
      
      const pushCommand = `"${adbPath}" -s ${deviceId} push "${localPath}" "${remotePath}"`;
      console.log('推送文件命令:', pushCommand);
      
      const { stdout, stderr } = await execPromise(pushCommand);
      
      if (stdout.includes('pushed') || stdout.includes('100%')) {
        return { success: true, message: '文件推送成功', output: stdout };
      }
      
      if (stderr && !stderr.includes('Warning')) {
        return { success: false, error: stderr };
      }
      
      return { success: true, message: '推送命令已执行', output: stdout };
      
    } catch (error) {
      console.error('推送文件失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 从设备拉取文件
  ipcMain.handle('pull-file', async (event, deviceId, remotePath, localPath) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      if (!deviceId || !remotePath || !localPath) {
        return { success: false, error: '参数无效' };
      }
      
      const pullCommand = `"${adbPath}" -s ${deviceId} pull "${remotePath}" "${localPath}"`;
      console.log('拉取文件命令:', pullCommand);
      
      const { stdout, stderr } = await execPromise(pullCommand);
      
      if (stdout.includes('pulled') || stdout.includes('100%')) {
        return { success: true, message: '文件拉取成功', output: stdout };
      }
      
      if (stderr && !stderr.includes('Warning')) {
        return { success: false, error: stderr };
      }
      
      return { success: true, message: '拉取命令已执行', output: stdout };
      
    } catch (error) {
      console.error('拉取文件失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 增强版截图，同时保存到工作区并获取UI树
  ipcMain.handle('adb-screenshot', async (event, deviceId, projectPath = null) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const tempPath = path.join(app.getPath('temp'), 'screenshot.png');
      const deviceArg = `-s ${deviceId}`;
      
      // 获取截图
      await execPromise(`"${adbPath}" ${deviceArg} exec-out screencap -p > "${tempPath}"`);
      const imageData = fs.readFileSync(tempPath);
      
      // 如果提供了项目路径，保存到工作区
      if (projectPath) {
        try {
          const workareaPath = path.join(projectPath, 'workarea');
          
          // 确保workarea目录存在
          if (!fs.existsSync(workareaPath)) {
            fs.mkdirSync(workareaPath, { recursive: true });
          }
          
          // 保存截图到工作区
          const screenshotPath = path.join(workareaPath, 'current_screenshot.png');
          fs.writeFileSync(screenshotPath, imageData);
          
          // 同时获取UI树
          try {
            await execPromise(`"${adbPath}" ${deviceArg} shell uiautomator dump /sdcard/window_dump.xml`);
            const { stdout: xmlContent } = await execPromise(`"${adbPath}" ${deviceArg} shell cat /sdcard/window_dump.xml`);
            
            // 保存XML到工作区
            const xmlPath = path.join(workareaPath, 'current_ui_tree.xml');
            fs.writeFileSync(xmlPath, xmlContent, 'utf8');
            
            console.log('已保存截图和UI树到workspace:', workareaPath);
          } catch (xmlError) {
            console.warn('获取UI树失败，但截图保存成功:', xmlError.message);
          }
          
          // 返回截图路径和base64数据
          return { 
            success: true, 
            data: imageData.toString('base64'),
            screenshotPath: screenshotPath
          };
        } catch (saveError) {
          console.warn('保存到workspace失败:', saveError.message);
          // 不影响截图返回
        }
      }
      
      return { success: true, data: imageData.toString('base64') };
    } catch (error) {
      console.error('截图失败:', error);
      return { success: false, error: error.message };
    }
  });

  // UI dump功能
  ipcMain.handle('adb-ui-dump', async (event, deviceId) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const remotePath = '/sdcard/ui_dump.xml';
      const localPath = path.join(app.getPath('temp'), `ui_dump_${deviceId}_${Date.now()}.xml`);

      // 在设备上生成UI dump
      const dumpCommand = `"${adbPath}" -s ${deviceId} shell uiautomator dump ${remotePath}`;
      await execPromise(dumpCommand);

      // 拉取UI dump到本地
      const pullCommand = `"${adbPath}" -s ${deviceId} pull ${remotePath} "${localPath}"`;
      await execPromise(pullCommand);

      // 读取文件内容
      const content = fs.readFileSync(localPath, 'utf8');

      // 清理文件
      fs.unlinkSync(localPath);
      const deleteCommand = `"${adbPath}" -s ${deviceId} shell rm ${remotePath}`;
      await execPromise(deleteCommand);

      return { 
        success: true, 
        content: content
      };

    } catch (error) {
      console.error('UI dump失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 增强版UI dump，包含屏幕尺寸信息
  ipcMain.handle('adb-ui-dump-enhanced', async (event, deviceId) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const deviceArg = `-s ${deviceId}`;
      
      // 1. 获取UI hierarchy
      await execPromise(`"${adbPath}" ${deviceArg} shell uiautomator dump /sdcard/window_dump.xml`);
      const { stdout: xmlContent } = await execPromise(`"${adbPath}" ${deviceArg} shell cat /sdcard/window_dump.xml`);
      
      // 2. 获取屏幕尺寸
      let screenSize = null;
      try {
        const { stdout: sizeOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell wm size`);
        const sizeMatch = sizeOutput.match(/Physical size: (\d+)x(\d+)/);
        if (sizeMatch) {
          screenSize = { 
            width: parseInt(sizeMatch[1]), 
            height: parseInt(sizeMatch[2]) 
          };
        } else {
          // 尝试从dumpsys获取
          const { stdout: dumpsysOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell dumpsys window displays | grep -E "Display|cur="`);
          const displayMatch = dumpsysOutput.match(/cur=(\d+)x(\d+)/);
          if (displayMatch) {
            screenSize = {
              width: parseInt(displayMatch[1]),
              height: parseInt(displayMatch[2])
            };
          }
        }
      } catch (sizeError) {
        console.warn('无法获取屏幕尺寸:', sizeError.message);
        // 使用默认尺寸
        screenSize = { width: 1080, height: 1920 };
      }
      
      // 3. 获取设备信息（可选）
      let deviceInfo = {};
      try {
        const { stdout: brandOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.product.brand`);
        const { stdout: modelOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.product.model`);
        const { stdout: versionOutput } = await execPromise(`"${adbPath}" ${deviceArg} shell getprop ro.build.version.release`);
        
        deviceInfo = {
          brand: brandOutput.trim(),
          model: modelOutput.trim(),
          androidVersion: versionOutput.trim()
        };
      } catch (infoError) {
        console.warn('无法获取设备信息:', infoError.message);
      }
      
      return { 
        success: true, 
        xml: xmlContent,
        screenSize: screenSize,
        deviceInfo: deviceInfo,
        timestamp: Date.now()
      };
    } catch (error) {
      console.error('Enhanced UI dump失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 初始化项目工作区
  ipcMain.handle('init-project-workarea', async (event, projectPath) => {
    try {
      const workareaPath = path.join(projectPath, 'workarea');
      
      // 确保workarea目录存在
      if (!fs.existsSync(workareaPath)) {
        fs.mkdirSync(workareaPath, { recursive: true });
        console.log('创建workspace目录:', workareaPath);
      }
      
      return { success: true, path: workareaPath };
    } catch (error) {
      console.error('初始化项目workspace失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 获取设备进程列表
  ipcMain.handle('get-device-processes', async (event, deviceId) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      const { stdout } = await execPromise(`"${adbPath}" -s ${deviceId} shell ps`);
      
      const lines = stdout.split('\n');
      const processes = [];
      
      for (let i = 1; i < lines.length; i++) { // 跳过标题行
        const line = lines[i].trim();
        if (line) {
          const parts = line.split(/\s+/);
          if (parts.length >= 9) {
            processes.push({
              user: parts[0],
              pid: parts[1],
              ppid: parts[2],
              vsz: parts[3],
              rss: parts[4],
              wchan: parts[5],
              addr: parts[6],
              s: parts[7],
              name: parts[8]
            });
          }
        }
      }

      return { 
        success: true, 
        processes: processes
      };

    } catch (error) {
      console.error('获取设备进程失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 无线设备管理
  ipcMain.handle('get-saved-wireless-devices', async () => {
    try {
      const devices = await ipcMain._events['store-get'][0](null, 'wireless_devices') || [];
      return { success: true, devices: devices };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('save-wireless-device', async (event, deviceConfig) => {
    try {
      let devices = await ipcMain._events['store-get'][0](null, 'wireless_devices') || [];
      
      // 检查是否已存在
      const existingIndex = devices.findIndex(d => d.ipAddress === deviceConfig.ipAddress);
      if (existingIndex !== -1) {
        devices[existingIndex] = deviceConfig;
      } else {
        devices.push(deviceConfig);
      }
      
      await ipcMain._events['store-set'][0](null, 'wireless_devices', devices);
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('delete-wireless-device', async (event, ipAddress) => {
    try {
      let devices = await ipcMain._events['store-get'][0](null, 'wireless_devices') || [];
      devices = devices.filter(d => d.ipAddress !== ipAddress);
      await ipcMain._events['store-set'][0](null, 'wireless_devices', devices);
      return { success: true };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 二维码和配对相关
  ipcMain.handle('generate-qr-code', async (event, data) => {
    try {
      // 这里应该使用二维码生成库，暂时返回模拟数据
      return {
        success: true,
        qrData: data,
        message: 'QR code generated successfully'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('generate-qr-pairing-data', async (event, ipAddress, port = 5555) => {
    try {
      const pairingData = {
        type: 'adb_pairing',
        ip: ipAddress,
        port: port,
        timestamp: Date.now()
      };
      
      return {
        success: true,
        data: JSON.stringify(pairingData)
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('start-adb-pairing-service', async (event, port = 5555) => {
    try {
      // 这里应该启动ADB配对服务
      // 暂时返回成功状态
      return {
        success: true,
        port: port,
        message: 'ADB pairing service started'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('scan-wireless-devices', async (event, ipRange = null) => {
    try {
      const adbPath = getBuiltInAdbPath(app);
      
      if (!fs.existsSync(adbPath)) {
        return { success: false, error: '内置Android SDK未找到' };
      }
      
      // 简化的扫描逻辑，实际应该扫描网络
      return {
        success: true,
        devices: [],
        message: '扫描完成'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });
}

module.exports = {
  registerAdbHandlers,
  getBuiltInAdbPath,
  getBuiltInScrcpyPath,
  getBuiltInStbPath
};