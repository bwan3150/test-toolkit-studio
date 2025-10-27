// ADB相关的IPC处理器
const { ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const os = require('os');
const { exec, spawn } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// 注册所有ADB相关的IPC处理器
function registerAdbHandlers(app) {
  // 获取连接的设备列表
  ipcMain.handle('get-devices', async () => {
    try {
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['devices', '-l']);
      
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
      console.log('执行TKE ADB Shell命令:', command, '设备:', deviceId);
      
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['shell', command]);
      
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
      
      if (!deviceId || !apkPath) {
        return { success: false, error: '无效的设备ID或APK路径' };
      }
      
      // 检查APK文件是否存在
      if (!fs.existsSync(apkPath)) {
        return { success: false, error: 'APK文件不存在' };
      }
      
      console.log('正在安装APK到设备:', deviceId, apkPath);
      
      // 构建安装命令
      let installFlags = ['-r']; // 替换安装
      if (forceReinstall) {
        installFlags.push('-d'); // 允许降级
      }
      installFlags.push('-g'); // 授予权限
      installFlags.push(apkPath);

      console.log('执行TKE安装命令:', installFlags);

      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['install', ...installFlags]);
      
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

  // 卸载应用
  ipcMain.handle('adb-uninstall-app', async (event, deviceId, packageName) => {
    try {
      if (!deviceId || !packageName) {
        return { success: false, error: '无效的设备ID或包名' };
      }

      console.log('卸载应用:', deviceId, packageName);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['uninstall', packageName]);

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
      if (!ipAddress || !port || !pairingCode) {
        return { success: false, error: '请提供IP地址、端口和配对码' };
      }

      console.log('开始ADB无线配对:', ipAddress, port);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['pair', `${ipAddress}:${port}`, pairingCode]);

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
      if (!ipAddress) {
        return { success: false, error: '请提供IP地址' };
      }

      const connectionAddress = `${ipAddress}:${port}`;
      console.log('连接到无线ADB设备:', connectionAddress);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['connect', connectionAddress]);

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
      if (!ipAddress) {
        return { success: false, error: '请提供IP地址' };
      }

      const connectionAddress = `${ipAddress}:${port}`;
      console.log('断开无线ADB设备连接:', connectionAddress);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, null, ['disconnect', connectionAddress]);

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

  // 执行通用ADB命令(现在adb直接运行不再使用了, 为了兼容渲染进程, 这个handler同样转发命令给tke运行内封adb)
  ipcMain.handle('execute-adb-command', async (event, deviceId, args) => {
    try {
      console.log('执行ADB命令:', args, '设备:', deviceId);
      
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, args);
      
      return { 
        success: true, 
        output: stdout,
        error: stderr
      };
    } catch (error) {
      console.error('执行ADB命令失败:', error);
      return { success: false, error: error.message };
    }
  });

  // 执行TKE ADB命令（通过TKE）
  ipcMain.handle('execute-tke-adb-command', async (event, deviceId, args) => {
    try {
      console.log('执行TKE ADB命令:', args, '设备:', deviceId);
      
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, args);
      
      return { 
        success: true, 
        output: stdout,
        error: stderr
      };
    } catch (error) {
      console.error('执行TKE ADB命令失败:', error);
      return { success: false, error: error.message };
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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }
      
      let packageName = '';
      let activityName = '';
      
      // 方法1: 尝试获取顶层Activity
      try {
        const { stdout } = await execTkeAdbCommand(app, deviceId, ['shell', 'dumpsys', 'activity', 'top']);
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
      
      // 方法2: 如果方法1没有成功，尝试通过window dumpsys获取
      if (!packageName) {
        try {
          const { stdout } = await execTkeAdbCommand(app, deviceId, ['shell', 'dumpsys', 'window']);
          if (stdout) {
            const focusMatch = stdout.match(/mCurrentFocus=Window\{[^}]*\s+([^\/\s]+)\/([^}\s]+)\}/);
            if (focusMatch && focusMatch.length >= 3) {
              packageName = focusMatch[1];
              activityName = focusMatch[2];
            }
          }
        } catch (error) {
          // 如果都失败了，返回错误
        }
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
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }
      
      let adbArgs;
      if (activityName) {
        adbArgs = ['shell', 'am', 'start', '-n', `${packageName}/${activityName}`];
      } else {
        // 使用monkey命令启动应用
        adbArgs = ['shell', 'monkey', '-p', packageName, '-c', 'android.intent.category.LAUNCHER', '1'];
      }
      
      console.log('启动应用参数:', adbArgs);
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, adbArgs);
      
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
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }

      console.log('停止应用:', deviceId, packageName);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['shell', 'am', 'force-stop', packageName]);

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
      if (!deviceId || !packageName) {
        return { success: false, error: '请提供设备ID和包名' };
      }

      console.log('清除应用数据:', deviceId, packageName);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['shell', 'pm', 'clear', packageName]);

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
      if (!deviceId || !localPath || !remotePath) {
        return { success: false, error: '参数无效' };
      }

      if (!fs.existsSync(localPath)) {
        return { success: false, error: '本地文件不存在' };
      }

      console.log('推送文件到设备:', deviceId, localPath, '->', remotePath);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['push', localPath, remotePath]);

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
      if (!deviceId || !remotePath || !localPath) {
        return { success: false, error: '参数无效' };
      }

      console.log('从设备拉取文件:', deviceId, remotePath, '->', localPath);

      // 使用TKE内置ADB
      const { stdout, stderr } = await execTkeAdbCommand(app, deviceId, ['pull', remotePath, localPath]);

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

  // 注意: adb-screenshot, adb-ui-dump, adb-ui-dump-enhanced 等旧的截图和UI获取方法已移除
  // 现在统一使用 tke-controller-capture (在 controller-handlers.js 中)

  // 初始化项目工作区
  ipcMain.handle('init-project-workarea', async (event, projectPath) => {
    try {
      // 验证projectPath是否有效（不为空且为绝对路径）
      if (!projectPath || typeof projectPath !== 'string' || projectPath.trim() === '') {
        return { success: false, error: '请提供有效的项目路径' };
      }

      if (!path.isAbsolute(projectPath)) {
        return { success: false, error: '项目路径必须是绝对路径' };
      }

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
      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      console.log('获取设备进程列表:', deviceId);

      // 使用TKE内置ADB
      const { stdout } = await execTkeAdbCommand(app, deviceId, ['shell', 'ps']);

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
  ipcMain.handle('generate-qr-code', async (event, data, options = {}) => {
    try {
      const QRCode = require('qrcode');
      
      // 设置QR码选项
      const qrOptions = {
        width: options.width || 200,
        height: options.height || 200,
        margin: options.margin || 1,
        color: {
          dark: options.darkColor || '#000000',
          light: options.lightColor || '#FFFFFF'
        },
        errorCorrectionLevel: 'M'
      };
      
      // 生成QR码的Data URL（base64格式的图片）
      const dataURL = await QRCode.toDataURL(data, qrOptions);
      
      return {
        success: true,
        dataURL: dataURL,
        message: 'QR code generated successfully'
      };
    } catch (error) {
      console.error('生成QR码失败:', error);
      return { success: false, error: error.message };
    }
  });

  ipcMain.handle('generate-qr-pairing-data', async (event) => {
    try {
      // 生成随机的配对码（6位数字）
      const pairingCode = Math.floor(100000 + Math.random() * 900000).toString();
      
      // 生成随机的配对端口（在合理范围内）
      const pairingPort = Math.floor(30000 + Math.random() * 10000);
      
      // 生成服务名称（用于mDNS广播）
      const serviceName = `adb-pairing-${Date.now()}`;
      
      // 获取本机IP地址
      const os = require('os');
      const networkInterfaces = os.networkInterfaces();
      let localIP = '127.0.0.1';
      
      // 查找本地IP地址（优先选择IPv4地址）
      for (const interfaceName in networkInterfaces) {
        const interfaces = networkInterfaces[interfaceName];
        for (const iface of interfaces) {
          if (iface.family === 'IPv4' && !iface.internal) {
            localIP = iface.address;
            break;
          }
        }
        if (localIP !== '127.0.0.1') break;
      }
      
      // 生成QR码数据
      const qrData = JSON.stringify({
        type: 'adb_pairing',
        service: serviceName,
        code: pairingCode,
        ip: localIP,
        port: pairingPort,
        timestamp: Date.now()
      });
      
      // 设置过期时间（10分钟）
      const expiryTime = Date.now() + 10 * 60 * 1000;
      
      return {
        success: true,
        serviceName: serviceName,
        pairingCode: pairingCode,
        qrData: qrData,
        expiryTime: expiryTime,
        localIP: localIP,
        pairingPort: pairingPort,
        adbPort: 5555  // 默认ADB端口
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
      console.log('扫描无线设备:', ipRange);

      // 简化的扫描逻辑，实际应该扫描网络
      // 未来可以通过TKE内置ADB实现设备扫描
      return {
        success: true,
        devices: [],
        message: '扫描完成'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  });

  // 注意: adb-get-ui-xml 已移除，统一使用 tke-controller-capture

  // 使用TKE提取UI元素 - 用于XML overlay
  ipcMain.handle('execute-tke-extract-elements', async (event, options) => {
    try {
      const { deviceId, projectPath, screenWidth, screenHeight } = options;

      if (!deviceId) {
        return { success: false, error: '请提供设备ID' };
      }

      // 验证projectPath是否有效（不为空且为绝对路径）
      if (!projectPath || typeof projectPath !== 'string' || projectPath.trim() === '') {
        return { success: false, error: '请提供有效的项目路径' };
      }

      if (!path.isAbsolute(projectPath)) {
        return { success: false, error: '项目路径必须是绝对路径' };
      }

      // 获取 TKE 可执行文件路径
      const tkePath = getTkePath(app);
      if (!fs.existsSync(tkePath)) {
        return { success: false, error: 'TKE可执行文件未找到' };
      }

      // XML文件路径
      const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
      
      // 检查XML文件是否存在
      if (!fs.existsSync(xmlPath)) {
        return { success: false, error: 'UI XML文件不存在，请先截图' };
      }

      // 构建 TKE 命令：tke fetcher extract-ui-elements --width <width> --height <height> < xml_file
      const args = [
        'fetcher', 'extract-ui-elements', 
        '--width', screenWidth.toString(), 
        '--height', screenHeight.toString()
      ];
      
      // 执行 TKE 命令，通过stdin传入XML文件内容
      const xmlContent = fs.readFileSync(xmlPath, 'utf8');
      const child = spawn(tkePath, args, { 
        stdio: ['pipe', 'pipe', 'pipe'] 
      });
      
      // 写入XML内容到stdin
      child.stdin.write(xmlContent);
      child.stdin.end();
      
      let stdout = '';
      let stderr = '';
      
      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });
      
      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });
      
      const exitCode = await new Promise((resolve) => {
        child.on('close', resolve);
      });
      
      if (exitCode !== 0) {
        console.error('TKE extract-ui-elements 命令失败:', stderr);
        return { success: false, error: `TKE命令失败: ${stderr}` };
      }
      
      try {
        // 解析JSON输出
        const rawElements = JSON.parse(stdout);
        console.log(`TKE成功提取了 ${rawElements.length} 个UI元素`);
        
        // 转换格式以匹配前端期望的格式
        const elements = rawElements.map(el => ({
          index: el.index,
          className: el.class_name || '',
          bounds: el.bounds ? [el.bounds.x1, el.bounds.y1, el.bounds.x2, el.bounds.y2] : [0, 0, 0, 0],
          text: el.text || '',
          contentDesc: el.content_desc || '',
          resourceId: el.resource_id || '',
          hint: el.hint || '',
          clickable: el.clickable || false,
          checkable: el.checkable || false,
          checked: el.checked || false,
          focusable: el.focusable || false,
          focused: el.focused || false,
          scrollable: el.scrollable || false,
          selected: el.selected || false,
          enabled: el.enabled || false,
          xpath: el.xpath || ''
        }));
        
        return { 
          success: true, 
          elements: elements,
          count: elements.length
        };
        
      } catch (parseError) {
        console.error('解析TKE输出JSON失败:', parseError, 'stdout:', stdout);
        return { success: false, error: `解析TKE输出失败: ${parseError.message}` };
      }
      
    } catch (error) {
      console.error('执行TKE extract-ui-elements失败:', error);
      return { success: false, error: error.message };
    }
  });
}

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

// 构建 TKE ADB 命令
function buildTkeAdbCommand(tkePath, deviceId, adbArgs) {
  const args = ['"' + tkePath + '"'];
  
  // 如果有设备ID，添加 --device 参数
  if (deviceId) {
    args.push('--device', deviceId);
  }
  
  // 添加 adb 子命令
  args.push('adb');
  
  // 添加 ADB 参数
  if (Array.isArray(adbArgs)) {
    args.push(...adbArgs);
  } else {
    args.push(adbArgs);
  }
  
  return args.join(' ');
}

// 辅助函数：执行 TKE ADB 命令替代原来的 ADB 命令
async function execTkeAdbCommand(app, deviceId, adbArgs) {
  const tkePath = getTkePath(app);
  if (!fs.existsSync(tkePath)) {
    throw new Error('Toolkit Engine未找到');
  }

  // 构建参数数组
  const args = [];
  if (deviceId) {
    args.push('--device', deviceId);
  }
  args.push('adb');

  if (Array.isArray(adbArgs)) {
    args.push(...adbArgs);
  } else {
    args.push(adbArgs);
  }

  // 使用spawn而不是execPromise来正确处理参数中的空格
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
        const error = new Error(`TKE ADB命令失败 (exit code ${code})`);
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

module.exports = {
  registerAdbHandlers,
  getTkePath,
  buildTkeAdbCommand,
  execTkeAdbCommand
};
