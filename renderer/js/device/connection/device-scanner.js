// 设备扫描和状态检测模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：loadSavedDevices, refreshDeviceList, setupDragAndDropForDevice

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 刷新连接的设备（统一显示 Android 和 iOS 设备）
async function refreshConnectedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();

    // 同时获取 Android 和 iOS 设备
    const [androidResult, iosResult] = await Promise.all([
        ipcRenderer.invoke('adb-devices'),
        ipcRenderer.invoke('get-ios-devices')
    ]);

    const connectedDevicesGrid = document.getElementById('connectedDevicesGrid');
    if (!connectedDevicesGrid) return;

    connectedDevicesGrid.innerHTML = '';

    // 合并所有设备
    const allDevices = [];
    if (androidResult.success) {
        allDevices.push(...androidResult.devices.map(device => ({ ...device, platform: 'android' })));
    }
    if (iosResult.success) {
        allDevices.push(...iosResult.devices.map(device => ({ ...device, platform: 'ios' })));
    }

    if (allDevices.length > 0) {
        // 获取保存的设备以检查哪些已经保存
        let savedDeviceIds = [];
        let savedDeviceConfigs = {};
        if (window.AppGlobals.currentProject) {
            try {
                const devicesPath = path.join(window.AppGlobals.currentProject, 'devices');
                const files = await fs.readdir(devicesPath);
                for (const file of files) {
                    if (file.endsWith('.yaml')) {
                        const content = await fs.readFile(path.join(devicesPath, file), 'utf-8');
                        const config = yaml.load(content);
                        if (config.deviceId) {
                            savedDeviceIds.push(config.deviceId);
                            savedDeviceConfigs[config.deviceId] = config;
                        }
                    }
                }
            } catch (error) {
                window.rError('Error loading saved devices:', error);
            }
        }

        // 为每个连接的设备创建卡片
        for (const device of allDevices) {
            // 跳过 Android 设备状态检查（Android 设备）或确认 iOS 设备已连接
            if (device.platform === 'android' && device.status !== 'device') continue;

            // 对于 iOS 设备，检测 WDA 连接状态
            if (device.platform === 'ios') {
                await checkIosDeviceStatus(device);
            }

            const isSaved = savedDeviceIds.includes(device.id);
            const savedConfig = savedDeviceConfigs[device.id];

            // 判断连接类型
            let isWifi = false;
            let connectionType = 'USB';

            if (device.platform === 'android') {
                // Android: 判断是否为无线设备：包含冒号或者是mDNS格式
                isWifi = device.id.includes(':') || device.id.includes('._adb-tls-connect._tcp');
                connectionType = isWifi ? 'WiFi' : 'USB';
            } else if (device.platform === 'ios') {
                // iOS: 根据 WDA 检测结果显示连接状态
                if (device.wdaStatus === 'connected') {
                    connectionType = device.connectionType === 'wifi' ? 'WDA-WiFi' : 'USB';
                    isWifi = device.connectionType === 'wifi';
                } else {
                    connectionType = 'Off';
                }
            }

            const item = document.createElement('div');
            item.className = 'device-phone-mockup';

            // 设置设备状态
            let deviceStatus = 'connected';
            let statusTitle = 'Connected';

            if (device.platform === 'ios' && device.wdaStatus !== 'connected') {
                deviceStatus = 'disconnected';
                statusTitle = 'iOS Device Found - WDA Not Available';
            }

            // 获取平台图标和连接图标
            const platformIcon = device.platform === 'ios' ?
                '../../assets/icons/device-page/apple.svg' :
                '../../assets/icons/device-page/android.svg';
            const connectionIcon = isWifi ?
                '../../assets/icons/device-page/wifi.svg' :
                '../../assets/icons/device-page/usb.svg';

            let cardContent = `
                <div class="device-status-indicator ${deviceStatus}" title="${statusTitle}"></div>
                ${isSaved ? '<div class="device-saved-label">已保存</div>' : ''}
                <div class="device-screen-content">
                    <img src="${platformIcon}" class="device-platform-icon" alt="${device.platform}">
                    <img src="${connectionIcon}" class="device-connection-icon" alt="${isWifi ? 'WiFi' : 'USB'}">
                    <div class="device-info-text">
                        <div class="device-id">${device.id}</div>
            `;


            cardContent += `
                    </div>
                </div>`;

            // 获取并显示当前App信息作为hover浮层
            let appInfoHtml = '';
            try {
                const appResult = await ipcRenderer.invoke('get-current-app', device.id);
                if (appResult.success) {
                    const deviceIdClean = device.id.replace(/[^a-zA-Z0-9]/g, '_');
                    appInfoHtml = `
                        <div class="device-app-info">
                            <div class="device-app-info-title">正在运行:</div>
                            <div class="device-app-field">
                                <span class="device-app-label">包名</span>
                                <span class="device-app-value" title="${appResult.packageName}">${appResult.packageName}</span>
                                <button class="device-app-copy-btn" onclick="copyToClipboard('${deviceIdClean}_package')" title="复制包名">
                                    <svg viewBox="0 0 24 24">
                                        <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                    </svg>
                                </button>
                                <span id="${deviceIdClean}_package" style="display: none;">${appResult.packageName}</span>
                            </div>
                            <div class="device-app-field">
                                <span class="device-app-label">Activity</span>
                                <span class="device-app-value" title="${appResult.activityName}">${appResult.activityName}</span>
                                <button class="device-app-copy-btn" onclick="copyToClipboard('${deviceIdClean}_activity')" title="复制Activity">
                                    <svg viewBox="0 0 24 24">
                                        <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                    </svg>
                                </button>
                                <span id="${deviceIdClean}_activity" style="display: none;">${appResult.activityName}</span>
                            </div>
                        </div>
                    `;
                }
            } catch (error) {
                // 不显示错误信息,只是不显示hover浮层
            }

            cardContent += appInfoHtml;

            cardContent += `
                <div class="device-actions">
                    ${!isSaved ? `
                        <button class="btn btn-primary btn-small" onclick="createDeviceFromConnected('${device.id}')">保存配置</button>
                    ` : ''}
                    ${device.platform === 'ios' && device.wdaStatus !== 'connected' ? `
                        <button class="btn btn-secondary btn-small" onclick="showWdaSetupGuide('${device.id}')">WDA设置</button>
                    ` : ''}
                    ${isWifi ? `
                        <button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${device.id.split(':')[0]}', ${device.id.split(':')[1] || 5555})">断开</button>
                    ` : ''}
                </div>
            `;

            item.innerHTML = cardContent;

            // 添加拖拽功能
            if (window.setupDragAndDropForDevice) {
                window.setupDragAndDropForDevice(item, device.id);
            }

            connectedDevicesGrid.appendChild(item);
        }
    } else {
        connectedDevicesGrid.innerHTML = '<div class="text-muted">No devices connected</div>';
    }

    // 同样重新加载保存的设备以更新连接状态
    if (window.DeviceManagerModule?.loadSavedDevices) {
        await window.DeviceManagerModule.loadSavedDevices();
    }

    // 更新设备选择下拉框
    if (window.DeviceManagerModule?.refreshDeviceList) {
        window.DeviceManagerModule.refreshDeviceList();
    }
}

// 检查 iOS 设备的 WDA 连接状态
async function checkIosDeviceStatus(device) {
    try {
        // 尝试通过 USB 连接检查 WDA 状态
        const response = await fetch(`http://localhost:8100/status`, {
            method: 'GET',
            timeout: 3000
        });

        if (response.ok) {
            const data = await response.json();
            device.wdaStatus = 'connected';
            device.wdaInfo = data;
            window.rLog('iOS 设备 WDA 状态检测成功（USB）:', device.id);
        } else {
            device.wdaStatus = 'disconnected';
        }
    } catch (error) {
        // USB 连接失败，尝试检查保存的 WiFi 配置
        try {
            const { path, fs, yaml } = getGlobals();
            if (window.AppGlobals.currentProject) {
                const devicesPath = path.join(window.AppGlobals.currentProject, 'devices');
                const files = await fs.readdir(devicesPath);

                for (const file of files) {
                    if (file.endsWith('.yaml')) {
                        const content = await fs.readFile(path.join(devicesPath, file), 'utf-8');
                        const config = yaml.load(content);

                        // 检查是否是对应的 iOS 设备配置
                        if (config.platform === 'ios' && config.udid === device.udid && config.connectionType === 'wifi' && config.ipAddress) {
                            const wifiResponse = await fetch(`http://${config.ipAddress}:${config.port || 8100}/status`, {
                                method: 'GET',
                                timeout: 3000
                            });

                            if (wifiResponse.ok) {
                                device.wdaStatus = 'connected';
                                device.connectionType = 'wifi';
                                device.ipAddress = config.ipAddress;
                                device.port = config.port || 8100;
                                window.rLog('iOS 设备 WDA 状态检测成功（WiFi）:', device.id);
                                return;
                            }
                        }
                    }
                }
            }
        } catch (wifiError) {
            window.rLog('WiFi WDA 状态检测失败:', wifiError.message);
            device.wdaError = wifiError.message;
        }

        device.wdaStatus = 'disconnected';
        window.rLog('iOS 设备 WDA 状态检测失败:', device.id);
    }
}

// 导出模块
window.DeviceScanner = {
    refreshConnectedDevices,
    checkIosDeviceStatus
};
