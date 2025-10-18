// iOS 相关的 IPC 处理器
const { ipcMain } = require('electron');
const { spawn, exec } = require('child_process');
const { promisify } = require('util');
const execPromise = promisify(exec);

// 存储活动的 iOS 进程
const activeProcesses = new Map();

// 注册所有 iOS 相关的 IPC 处理器
function registerIosHandlers(app) {
    console.log('注册iOS处理器...');

    // 启动 iOS USB 端口转发
    ipcMain.handle('start-ios-usb-forwarding', async (event, { deviceUDID, localPort, devicePort }) => {
        try {
            console.log(`启动 iOS USB 转发: ${deviceUDID} ${localPort}:${devicePort}`);
            
            // 检查是否已有转发进程
            const processKey = `${deviceUDID}_${localPort}`;
            if (activeProcesses.has(processKey)) {
                console.log('USB 转发已存在');
                return { success: true, message: 'USB 转发已存在' };
            }
            
            // 启动 iproxy 进程
            const iproxyProcess = spawn('iproxy', [localPort.toString(), devicePort.toString(), deviceUDID], {
                stdio: ['ignore', 'pipe', 'pipe']
            });
            
            return new Promise((resolve) => {
                let resolved = false;
                
                // 设置超时
                const timeout = setTimeout(() => {
                    if (!resolved) {
                        resolved = true;
                        iproxyProcess.kill();
                        resolve({ success: false, error: 'USB 转发启动超时' });
                    }
                }, 10000);
                
                iproxyProcess.stdout.on('data', (data) => {
                    const output = data.toString();
                    console.log(`iproxy 输出: ${output}`);
                    
                    if (!resolved && (output.includes('waiting for connection') || output.includes('accepted connection'))) {
                        resolved = true;
                        clearTimeout(timeout);
                        activeProcesses.set(processKey, iproxyProcess);
                        resolve({ success: true, message: 'USB 转发启动成功' });
                    }
                });
                
                iproxyProcess.stderr.on('data', (data) => {
                    const error = data.toString();
                    console.error(`iproxy 错误: ${error}`);
                    
                    if (!resolved) {
                        resolved = true;
                        clearTimeout(timeout);
                        resolve({ success: false, error: `iproxy 错误: ${error}` });
                    }
                });
                
                iproxyProcess.on('error', (error) => {
                    console.error('iproxy 进程错误:', error);
                    if (!resolved) {
                        resolved = true;
                        clearTimeout(timeout);
                        resolve({ success: false, error: `iproxy 进程错误: ${error.message}` });
                    }
                });
                
                iproxyProcess.on('close', (code) => {
                    console.log(`iproxy 进程退出，代码: ${code}`);
                    activeProcesses.delete(processKey);
                    if (!resolved) {
                        resolved = true;
                        clearTimeout(timeout);
                        resolve({ success: false, error: `iproxy 进程意外退出，代码: ${code}` });
                    }
                });
                
                // 2秒后如果没有输出，认为启动成功
                setTimeout(() => {
                    if (!resolved) {
                        resolved = true;
                        clearTimeout(timeout);
                        activeProcesses.set(processKey, iproxyProcess);
                        resolve({ success: true, message: 'USB 转发启动成功' });
                    }
                }, 2000);
            });
            
        } catch (error) {
            console.error('启动 iOS USB 转发失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 停止 iOS USB 端口转发
    ipcMain.handle('stop-ios-usb-forwarding', async (event, deviceUDID) => {
        try {
            console.log(`停止 iOS USB 转发: ${deviceUDID}`);
            
            // 查找并停止相关的转发进程
            let stopped = false;
            for (const [key, process] of activeProcesses.entries()) {
                if (key.startsWith(deviceUDID)) {
                    process.kill();
                    activeProcesses.delete(key);
                    stopped = true;
                    console.log(`已停止转发进程: ${key}`);
                }
            }
            
            return { 
                success: true, 
                message: stopped ? 'USB 转发已停止' : '没有找到活动的转发进程' 
            };
        } catch (error) {
            console.error('停止 iOS USB 转发失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 获取 iOS 设备列表（通过 libimobiledevice）
    ipcMain.handle('get-ios-devices', async () => {
        try {
            console.log('获取 iOS 设备列表...');
            
            const { stdout } = await execPromise('idevice_id -l');
            const deviceIds = stdout.trim().split('\n').filter(id => id.length > 0);
            
            const devices = [];
            for (const deviceId of deviceIds) {
                try {
                    // 获取设备信息
                    const { stdout: deviceInfo } = await execPromise(`ideviceinfo -u ${deviceId} -k DeviceName,ProductType,ProductVersion`);
                    const info = {};
                    deviceInfo.split('\n').forEach(line => {
                        const [key, value] = line.split(': ');
                        if (key && value) {
                            info[key.trim()] = value.trim();
                        }
                    });
                    
                    devices.push({
                        id: deviceId,
                        udid: deviceId,
                        name: info.DeviceName || 'Unknown iOS Device',
                        model: info.ProductType || 'Unknown',
                        version: info.ProductVersion || 'Unknown',
                        platform: 'ios',
                        status: 'connected'
                    });
                } catch (infoError) {
                    console.warn(`获取设备 ${deviceId} 信息失败:`, infoError.message);
                    devices.push({
                        id: deviceId,
                        udid: deviceId,
                        name: 'iOS Device',
                        model: 'Unknown',
                        version: 'Unknown',
                        platform: 'ios',
                        status: 'connected'
                    });
                }
            }
            
            console.log(`找到 ${devices.length} 个 iOS 设备`);
            return { success: true, devices };
        } catch (error) {
            console.log('没有找到 iOS 设备或 libimobiledevice 未安装');
            return { success: true, devices: [] };
        }
    });

    // 获取 iOS 设备日志
    ipcMain.handle('get-ios-device-logs', async (event, deviceUDID) => {
        try {
            console.log(`获取 iOS 设备日志: ${deviceUDID}`);
            
            const { stdout } = await execPromise(`idevicesyslog -u ${deviceUDID} | head -100`);
            return { success: true, logs: stdout };
        } catch (error) {
            console.error('获取 iOS 设备日志失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 安装 iOS 应用
    ipcMain.handle('install-ios-app', async (event, { deviceUDID, ipaPath }) => {
        try {
            console.log(`安装 iOS 应用: ${deviceUDID} -> ${ipaPath}`);
            
            const { stdout, stderr } = await execPromise(`ideviceinstaller -u ${deviceUDID} -i "${ipaPath}"`);
            
            if (stderr && stderr.includes('error')) {
                throw new Error(stderr);
            }
            
            return { success: true, message: '应用安装成功' };
        } catch (error) {
            console.error('安装 iOS 应用失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 卸载 iOS 应用
    ipcMain.handle('uninstall-ios-app', async (event, { deviceUDID, bundleId }) => {
        try {
            console.log(`卸载 iOS 应用: ${deviceUDID} -> ${bundleId}`);
            
            const { stdout, stderr } = await execPromise(`ideviceinstaller -u ${deviceUDID} -U ${bundleId}`);
            
            if (stderr && stderr.includes('error')) {
                throw new Error(stderr);
            }
            
            return { success: true, message: '应用卸载成功' };
        } catch (error) {
            console.error('卸载 iOS 应用失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 检查 iOS 开发工具是否可用
    ipcMain.handle('check-ios-tools', async () => {
        const tools = {
            libimobiledevice: false,
            iproxy: false
        };
        
        try {
            await execPromise('idevice_id --version');
            tools.libimobiledevice = true;
        } catch (error) {
            console.log('libimobiledevice 不可用');
        }
        
        try {
            await execPromise('iproxy --help');
            tools.iproxy = true;
        } catch (error) {
            console.log('iproxy 不可用');
        }
        
        return { success: true, tools };
    });

    console.log('iOS处理器注册完成');
}

// 清理所有活动进程
function cleanupIosProcesses() {
    console.log('清理 iOS 进程...');
    for (const [key, process] of activeProcesses.entries()) {
        try {
            process.kill();
            console.log(`已终止进程: ${key}`);
        } catch (error) {
            console.error(`终止进程失败 ${key}:`, error);
        }
    }
    activeProcesses.clear();
}

module.exports = {
    registerIosHandlers,
    cleanupIosProcesses
};