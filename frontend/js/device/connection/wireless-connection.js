// 无线设备连接管理模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：refreshConnectedDevices (来自 device-scanner.js)

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 无线设备连接功能
async function connectWirelessDevice(ipAddress, port = 5555) {
    if (!ipAddress) {
        window.AppNotifications?.warn('请输入设备IP地址');
        return;
    }

    const { ipcRenderer } = getGlobals();

    // 显示连接中状态
    window.AppNotifications?.info('正在连接无线设备...');

    try {
        const result = await ipcRenderer.invoke('adb-connect-wireless', ipAddress, port);

        if (result.success) {
            window.AppNotifications?.success(result.message || '连接成功');
            // 连接成功后刷新设备列表
            if (window.DeviceScanner?.refreshConnectedDevices) {
                await window.DeviceScanner.refreshConnectedDevices();
            }
        } else {
            // 如果连接失败，可能需要先配对
            if (result.error && result.error.includes('unauthorized')) {
                window.AppNotifications?.warn('设备未授权，请先使用配对码进行配对');
            } else {
                window.AppNotifications?.error(result.error || '连接失败');
            }
        }

        return result;
    } catch (error) {
        window.AppNotifications?.error(`连接失败: ${error.message}`);
        return { success: false, error: error.message };
    }
}

// 断开无线设备连接
async function disconnectWirelessDevice(ipAddress, port = 5555) {
    if (!ipAddress) {
        window.AppNotifications?.warn('请提供设备IP地址');
        return;
    }

    const { ipcRenderer } = getGlobals();

    try {
        const result = await ipcRenderer.invoke('adb-disconnect-wireless', ipAddress, port);

        if (result.success) {
            window.AppNotifications?.success(result.message);
            // 断开连接后刷新设备列表
            if (window.DeviceScanner?.refreshConnectedDevices) {
                await window.DeviceScanner.refreshConnectedDevices();
            }
        } else {
            window.AppNotifications?.error(result.error);
        }

        return result;
    } catch (error) {
        window.AppNotifications?.error(`断开连接失败: ${error.message}`);
        return { success: false, error: error.message };
    }
}

// 全局函数
window.connectWirelessDevice = connectWirelessDevice;
window.disconnectWirelessDevice = disconnectWirelessDevice;

// 导出模块
window.WirelessConnection = {
    connectWirelessDevice,
    disconnectWirelessDevice
};
