// 设备加载模块
// 负责从项目文件夹加载已保存的设备配置并显示在UI中
// 依赖：window.AppGlobals, window.AppNotifications

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 加载保存的设备（统一管理有线和无线设备）
async function loadSavedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();
    if (!window.AppGlobals.currentProject) return;

    const savedDevicesGrid = document.getElementById('savedDevicesGrid');
    if (!savedDevicesGrid) return;

    savedDevicesGrid.innerHTML = '';

    const devicesPath = path.join(window.AppGlobals.currentProject, 'devices');

    try {
        // 先获取连接的设备
        const connectedResult = await ipcRenderer.invoke('adb-devices');
        const connectedDevices = connectedResult.success ? connectedResult.devices : [];

        const files = await fs.readdir(devicesPath);

        for (const file of files) {
            if (file.endsWith('.yaml')) {
                const filePath = path.join(devicesPath, file);
                const content = await fs.readFile(filePath, 'utf-8');
                const config = yaml.load(content);

                // 通过匹配deviceId检查此设备是否已连接
                const isConnected = config.deviceId && connectedDevices.some(d =>
                    d.id === config.deviceId && d.status === 'device'
                );

                // 判断连接类型：包含冒号或mDNS格式为无线设备
                const connectionType = config.connectionType || 'usb';
                const isWifi = connectionType === 'wifi' ||
                               (config.ipAddress && config.port) ||
                               config.deviceId?.includes(':') ||
                               config.deviceId?.includes('._adb-tls-connect._tcp');

                // 获取平台类型 (android/ios)
                const platform = config.platformName?.toLowerCase() === 'ios' ? 'ios' : 'android';
                const platformIcon = platform === 'ios' ?
                    '../../assets/icons/device-page/apple.svg' :
                    '../../assets/icons/device-page/android.svg';
                const connectionIcon = isWifi ?
                    '../../assets/icons/device-page/wifi.svg' :
                    '../../assets/icons/device-page/usb.svg';

                const card = document.createElement('div');
                card.className = 'device-phone-mockup';
                card.innerHTML = `
                    <div class="device-status-indicator ${isConnected ? 'connected' : ''}" title="${isConnected ? '已连接' : '未连接'}"></div>
                    <div class="device-saved-label">已保存</div>
                    <div class="device-screen-content">
                        <img src="${platformIcon}" class="device-platform-icon" alt="${platform}">
                        <img src="${connectionIcon}" class="device-connection-icon" alt="${isWifi ? 'WiFi' : 'USB'}">
                        <div class="device-info-text">
                            <div class="device-id">${isWifi ?
                                `${config.ipAddress}:${config.port || 5555}` :
                                (config.deviceId || config.deviceName)
                            }</div>
                            <div class="no-app-info">${config.platformName} ${config.platformVersion}</div>
                        </div>
                    </div>
                    <div class="device-actions">
                        ${isWifi && !isConnected ? `
                            <button class="btn btn-primary btn-small" onclick="connectWirelessDevice('${config.ipAddress}', ${config.port || 5555})">连接</button>
                        ` : ''}
                        ${isWifi && isConnected ? `
                            <button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${config.ipAddress}', ${config.port || 5555})">断开</button>
                        ` : ''}
                        <button class="btn btn-secondary btn-small" onclick="editDevice('${file}')">编辑</button>
                        <button class="btn btn-outline btn-small" onclick="deleteDevice('${file}')" title="删除">删除</button>
                    </div>
                `;

                // 为保存的设备卡片也添加拖拽功能
                // 根据平台类型获取设备标识符
                let deviceIdentifier = null;

                if (config.platform === 'ios') {
                    // iOS设备使用udid
                    deviceIdentifier = config.udid;
                } else {
                    // Android设备使用deviceId，如果没有deviceId，尝试使用其他标识
                    deviceIdentifier = config.deviceId;

                    // 如果没有deviceId但是连接设备，尝试从连接的设备中查找
                    if (!deviceIdentifier && isConnected && connectedDevices.length > 0) {
                        // 尝试找到对应的连接设备
                        const connectedDevice = connectedDevices.find(d =>
                            d.status === 'device' && config.deviceName &&
                            (config.deviceName.includes(d.id.substring(0, 8)) ||
                             (config.ipAddress && d.id.includes(config.ipAddress)))
                        );
                        if (connectedDevice) {
                            deviceIdentifier = connectedDevice.id;
                        }
                    }
                }

                if (deviceIdentifier) {
                    if (window.setupDragAndDropForDevice) {
                        window.setupDragAndDropForDevice(card, deviceIdentifier);
                        window.rLog('为保存的设备添加拖拽功能:', config.deviceName, deviceIdentifier);
                    }
                } else {
                    window.rLog('无法为设备添加拖拽功能（缺少设备标识）:', config.deviceName);
                }

                savedDevicesGrid.appendChild(card);
            }
        }
    } catch (error) {
        window.rError('Failed to load saved devices:', error);
    }
}

// 刷新下拉框中的设备列表
async function refreshDeviceList() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();
    const result = await ipcRenderer.invoke('adb-devices');
    const deviceSelect = document.getElementById('deviceSelect');

    if (!deviceSelect) return;

    // 获取保存的设备以恢复选择
    const savedSelection = await ipcRenderer.invoke('store-get', 'selected_device');

    deviceSelect.innerHTML = '<option value="">Select Device</option>';

    if (result.success && result.devices.length > 0) {
        // 加载保存的设备配置以显示名称而不是ID
        let deviceConfigs = {};
        if (window.AppGlobals.currentProject) {
            try {
                const devicesPath = path.join(window.AppGlobals.currentProject, 'devices');
                const files = await fs.readdir(devicesPath);
                for (const file of files) {
                    if (file.endsWith('.yaml')) {
                        const content = await fs.readFile(path.join(devicesPath, file), 'utf-8');
                        const config = yaml.load(content);
                        if (config.deviceId) {
                            deviceConfigs[config.deviceId] = config.deviceName;
                        }
                    }
                }
            } catch (error) {
                window.rError('Error loading device configs:', error);
            }
        }

        result.devices.forEach(device => {
            if (device.status === 'device') {
                const option = document.createElement('option');
                option.value = device.id;
                // 如果保存则使用设备名称，否则使用ID
                option.textContent = deviceConfigs[device.id] || device.id;
                deviceSelect.appendChild(option);
            }
        });

        // 如果设备仍然可用则恢复选择
        if (savedSelection && Array.from(deviceSelect.options).some(opt => opt.value === savedSelection)) {
            deviceSelect.value = savedSelection;
            // 通知 ScreenCoordinator 检查设备状态
            setTimeout(() => {
                if (window.ScreenCoordinator && window.ScreenCoordinator.checkDeviceStatusAndPrompt) {
                    window.ScreenCoordinator.checkDeviceStatusAndPrompt();
                }
            }, 200);
        }
    }

    // 如果没有选中任何设备，也通知 ScreenCoordinator 显示提示
    if (!deviceSelect.value) {
        setTimeout(() => {
            if (window.ScreenCoordinator && window.ScreenCoordinator.checkDeviceStatusAndPrompt) {
                window.ScreenCoordinator.checkDeviceStatusAndPrompt();
            }
        }, 200);
    }
}

// 导出模块
window.DeviceLoader = {
    loadSavedDevices,
    refreshDeviceList
};
