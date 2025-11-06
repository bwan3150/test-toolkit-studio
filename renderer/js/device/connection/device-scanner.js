// 设备扫描和状态检测模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：loadSavedDevices, refreshDeviceList, setupDragAndDropForDevice

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 刷新连接的设备（按平台分组显示 Android 和 iOS 设备）
async function refreshConnectedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();

    // 同时获取 Android 和 iOS 设备
    const [androidResult, iosResult] = await Promise.all([
        ipcRenderer.invoke('adb-devices'),
        ipcRenderer.invoke('get-ios-devices')
    ]);

    const iosDevicesGrid = document.getElementById('iosDevicesGrid');
    const androidDevicesGrid = document.getElementById('androidDevicesGrid');

    if (!iosDevicesGrid || !androidDevicesGrid) return;

    // 清空两个容器
    iosDevicesGrid.innerHTML = '';
    androidDevicesGrid.innerHTML = '';

    // 获取Android和iOS设备列表
    const androidDevices = androidResult.success ? androidResult.devices.map(device => ({ ...device, platform: 'android' })) : [];
    const iosDevices = iosResult.success ? iosResult.devices.map(device => ({ ...device, platform: 'ios' })) : [];

    // 获取保存的设备配置
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
                        savedDeviceConfigs[config.deviceId] = config;
                    }
                }
            }
        } catch (error) {
            window.rError('加载保存的设备配置失败:', error);
        }
    }

    // 处理iOS设备
    await processDevicesByPlatform(iosDevices, 'ios', iosDevicesGrid, savedDeviceConfigs);

    // 处理Android设备
    await processDevicesByPlatform(androidDevices, 'android', androidDevicesGrid, savedDeviceConfigs);

    // 重新加载保存的设备以更新连接状态
    // 传入 skipConnected=true 以避免重复显示已连接的设备
    if (window.DeviceLoader?.loadSavedDevices) {
        await window.DeviceLoader.loadSavedDevices(true);
    }

    // 更新设备选择下拉框
    if (window.DeviceLoader?.refreshDeviceList) {
        window.DeviceLoader.refreshDeviceList();
    }
}

// 按平台处理设备列表
async function processDevicesByPlatform(devices, platform, gridElement, savedDeviceConfigs) {
    const { ipcRenderer } = getGlobals();

    // 不在这里显示"暂无设备"，因为可能有保存的设备
    // 空状态的显示应该由 loadSavedDevices 统一判断
    if (devices.length === 0) {
        return;
    }

    // 过滤掉无效的Android设备
    let validDevices = devices;
    if (platform === 'android') {
        validDevices = devices.filter(device => device.status === 'device');
    }

    // 对于iOS设备，检测WDA连接状态
    if (platform === 'ios') {
        for (const device of validDevices) {
            await checkIosDeviceStatus(device);
        }
    }

    // 判断设备的连接状态
    const devicesWithConnectionStatus = validDevices.map(device => {
        let isConnected = false;
        let isWifi = false;

        if (platform === 'android') {
            // Android: 判断是否为无线设备
            isWifi = device.id.includes(':') || device.id.includes('._adb-tls-connect._tcp');
            isConnected = true; // Android设备如果在列表中就是已连接
        } else if (platform === 'ios') {
            // iOS: 根据WDA检测结果判断连接状态
            isConnected = device.wdaStatus === 'connected';
            isWifi = device.connectionType === 'wifi';
        }

        return {
            ...device,
            isConnected,
            isWifi
        };
    });

    // 排序：已连接的设备排在前面
    devicesWithConnectionStatus.sort((a, b) => {
        if (a.isConnected && !b.isConnected) return -1;
        if (!a.isConnected && b.isConnected) return 1;
        return 0;
    });

    // 为每个设备创建卡片
    for (const device of devicesWithConnectionStatus) {
        const savedConfig = savedDeviceConfigs[device.id];
        const isSaved = !!savedConfig;

        const item = document.createElement('div');
        item.className = 'device-phone-mockup';

        // 获取平台图标 SVG
        const platformIconSvg = platform === 'ios' ?
            `<svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.54 4.09l.01-.01zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"/>
            </svg>` :
            `<svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 18c0 .55.45 1 1 1h1v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h2v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h1c.55 0 1-.45 1-1V8H6v10zM3.5 8C2.67 8 2 8.67 2 9.5v7c0 .83.67 1.5 1.5 1.5S5 17.33 5 16.5v-7C5 8.67 4.33 8 3.5 8zm17 0c-.83 0-1.5.67-1.5 1.5v7c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5v-7c0-.83-.67-1.5-1.5-1.5zm-4.97-5.84l1.3-1.3c.2-.2.2-.51 0-.71-.2-.2-.51-.2-.71 0l-1.48 1.48C13.85 1.23 12.95 1 12 1c-.96 0-1.86.23-2.66.63L7.85.15c-.2-.2-.51-.2-.71 0-.2.2-.2.51 0 .71l1.31 1.31C6.97 3.26 6 5.01 6 7h12c0-1.99-.97-3.75-2.47-4.84zM10 5H9V4h1v1zm5 0h-1V4h1v1z"/>
            </svg>`;

        // 构建连接状态标签HTML
        let connectionBadgeHtml = '';
        if (device.isConnected) {
            if (device.isWifi) {
                // WiFi已连接
                connectionBadgeHtml = `
                    <div class="device-connection-badge wifi-connected">
                        <svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 20h.01"/>
                            <path d="M2 8.82a15 15 0 0 1 20 0"/>
                            <path d="M5 12.859a10 10 0 0 1 14 0"/>
                            <path d="M8.5 16.429a5 5 0 0 1 7 0"/>
                        </svg>
                        <span class="badge-dot"></span>
                        <span class="badge-text">Wi-Fi</span>
                    </div>
                `;
            } else {
                // USB已连接
                connectionBadgeHtml = `
                    <div class="device-connection-badge usb-connected">
                        <svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M9 17H7A5 5 0 0 1 7 7h2"/>
                            <path d="M15 7h2a5 5 0 1 1 0 10h-2"/>
                            <line x1="8" x2="16" y1="12" y2="12"/>
                        </svg>
                        <span class="badge-dot"></span>
                        <span class="badge-text">USB</span>
                    </div>
                `;
            }
        } else {
            if (device.isWifi) {
                // WiFi未连接
                connectionBadgeHtml = `
                    <div class="device-connection-badge wifi-disconnected">
                        <svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 20h.01"/>
                            <path d="M8.5 16.429a5 5 0 0 1 7 0"/>
                            <path d="M5 12.859a10 10 0 0 1 5.17-2.69"/>
                            <path d="M19 12.859a10 10 0 0 0-2.007-1.523"/>
                            <path d="M2 8.82a15 15 0 0 1 4.177-2.643"/>
                            <path d="M22 8.82a15 15 0 0 0-11.288-3.764"/>
                            <path d="m2 2 20 20"/>
                        </svg>
                        <span class="badge-dot"></span>
                        <span class="badge-text">Disconnected</span>
                    </div>
                `;
            } else {
                // USB未连接
                connectionBadgeHtml = `
                    <div class="device-connection-badge usb-disconnected">
                        <svg class="badge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M9 17H7A5 5 0 0 1 7 7"/>
                            <path d="M15 7h2a5 5 0 0 1 4 8"/>
                            <line x1="8" x2="12" y1="12" y2="12"/>
                            <line x1="2" x2="22" y1="2" y2="22"/>
                        </svg>
                        <span class="badge-dot"></span>
                        <span class="badge-text">Disconnected</span>
                    </div>
                `;
            }
        }

        // 构建设备信息显示
        let deviceInfoHtml = '';
        if (isSaved && savedConfig.deviceName) {
            // 已保存：显示设备名称作为昵称，下面显示ID和model
            const deviceIdPart = `<div class="device-id-small">${device.id}</div>`;
            const modelPart = device.model ? `<div class="device-model-small">${device.model}</div>` : '';
            deviceInfoHtml = `
                <div class="device-platform-icon">
                    ${platformIconSvg}
                </div>
                <div class="device-name">${savedConfig.deviceName}</div>
                ${deviceIdPart}
                ${modelPart}
            `;
        } else {
            // 未保存：显示设备ID作为昵称，下面显示model
            const modelPart = device.model ? `<div class="device-model-small">${device.model}</div>` : '';
            deviceInfoHtml = `
                <div class="device-platform-icon">
                    ${platformIconSvg}
                </div>
                <div class="device-name">${device.id}</div>
                ${modelPart}
            `;
        }

        let cardContent = `
            ${connectionBadgeHtml}
            <div class="device-screen-content">
                <div class="device-info-text">
                    ${deviceInfoHtml}
                </div>
            </div>
        `;

        // 获取并显示当前App信息作为hover浮层（仅Android设备且已连接）
        if (platform === 'android' && device.isConnected) {
            try {
                const appResult = await ipcRenderer.invoke('get-current-app', device.id);
                if (appResult.success) {
                    const deviceIdClean = device.id.replace(/[^a-zA-Z0-9]/g, '_');
                    cardContent += `
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
        }

        // 添加操作按钮
        const actionButtons = [];

        if (!isSaved) {
            actionButtons.push(`<button class="btn btn-primary btn-small" onclick="createDeviceFromConnected('${device.id}')">保存配置</button>`);
        }

        if (platform === 'ios' && !device.isConnected) {
            actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="showWdaSetupGuide('${device.id}')">WDA设置</button>`);
        }

        if (device.isWifi && device.isConnected) {
            const port = device.id.split(':')[1] || 5555;
            const host = device.id.split(':')[0];
            actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${host}', ${port})">断开</button>`);
        }

        if (actionButtons.length > 0) {
            cardContent += `
                <div class="device-actions">
                    ${actionButtons.join('\n')}
                </div>
            `;
        }

        item.innerHTML = cardContent;

        // 添加拖拽功能（仅Android设备且已连接）
        if (platform === 'android' && device.isConnected && window.setupDragAndDropForDevice) {
            window.setupDragAndDropForDevice(item, device.id);
        }

        gridElement.appendChild(item);
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
