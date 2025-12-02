// 设备加载模块
// 负责从数据库加载已保存的设备配置并显示在UI中
// 依赖：window.AppGlobals, window.AppNotifications

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 加载保存的设备（按平台分组显示）
// 显示所有保存的设备，包含已连接和未连接的状态
async function loadSavedDevices() {
    const { ipcRenderer } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    const iosDevicesGrid = document.getElementById('iosDevicesGrid');
    const androidDevicesGrid = document.getElementById('androidDevicesGrid');

    if (!iosDevicesGrid || !androidDevicesGrid) return;

    try {
        // 从数据库获取所有保存的设备
        const dbResult = await ipcRenderer.invoke('db-device-getAll', projectPath);
        if (!dbResult.success) {
            window.rError('从数据库获取设备失败:', dbResult.error);
            return;
        }

        const savedDevices = dbResult.data || [];

        // 获取连接的Android和iOS设备
        const [androidResult, iosResult] = await Promise.all([
            ipcRenderer.invoke('adb-devices'),
            ipcRenderer.invoke('get-ios-devices')
        ]);

        const connectedAndroidDevices = androidResult.success ? androidResult.devices : [];
        const connectedIosDevices = iosResult.success ? iosResult.devices : [];

        // 分类保存的设备
        const iosDevices = [];
        const androidDevices = [];

        for (const config of savedDevices) {
            // 获取平台类型
            const platform = config.platform?.toLowerCase() === 'ios' ? 'ios' : 'android';

            // 判断连接类型
            const connectionType = config.connectionType || 'usb';
            const isWifi = connectionType === 'wifi' ||
                           (config.ipAddress && config.port) ||
                           config.deviceId?.includes(':') ||
                           config.deviceId?.includes('._adb-tls-connect._tcp');

            // 检查设备是否已连接
            let isConnected = false;
            if (platform === 'android') {
                isConnected = config.deviceId && connectedAndroidDevices.some(d =>
                    d.id === config.deviceId && d.status === 'device'
                );
            } else if (platform === 'ios') {
                // iOS设备通过UDID匹配
                isConnected = config.udid && connectedIosDevices.some(d =>
                    d.udid === config.udid
                );
            }

            const deviceData = {
                id: config.id, // 数据库ID
                config,
                platform,
                isWifi,
                isConnected
            };

            // 按平台分类
            if (platform === 'ios') {
                iosDevices.push(deviceData);
            } else {
                androidDevices.push(deviceData);
            }
        }

        // 对设备进行排序：正在连接的设备排在最前面
        iosDevices.sort((a, b) => {
            if (a.isConnected && !b.isConnected) return -1;
            if (!a.isConnected && b.isConnected) return 1;
            return 0;
        });

        androidDevices.sort((a, b) => {
            if (a.isConnected && !b.isConnected) return -1;
            if (!a.isConnected && b.isConnected) return 1;
            return 0;
        });

        // 渲染所有保存的设备，无论是否已连接
        // 已连接的设备将显示"已连接"状态，未连接的显示"未连接"状态
        await renderSavedDevices(iosDevices, iosDevicesGrid, connectedIosDevices);
        await renderSavedDevices(androidDevices, androidDevicesGrid, connectedAndroidDevices);

        // 检查每个平台的设备总数，如果为空则显示空状态提示
        if (iosDevicesGrid.children.length === 0) {
            iosDevicesGrid.innerHTML = '<div class="empty-state">No devices found. Connect a device or add a configuration.</div>';
        }

        if (androidDevicesGrid.children.length === 0) {
            androidDevicesGrid.innerHTML = '<div class="empty-state">No devices found. Connect a device or add a configuration.</div>';
        }

    } catch (error) {
        window.rError('Failed to load saved devices:', error);
    }
}

// 渲染保存的设备列表
async function renderSavedDevices(devices, gridElement, connectedDevices) {
    // 注意：不清空grid，因为连接的设备也在同一个grid中
    // 但需要检查设备是否已存在，避免重复添加

    for (const deviceData of devices) {
        const { id, config, platform, isWifi, isConnected } = deviceData;

        // 检查设备是否已存在 - 通过检查是否有相同的data-db-id属性
        const existingCard = gridElement.querySelector(`[data-db-id="${id}"]`);
        if (existingCard) {
            window.rLog('设备已存在，跳过:', config.deviceName || id);
            continue; // 跳过已存在的设备
        }

        // 获取平台图标 SVG
        const platformIconSvg = platform === 'ios' ?
            `<svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.54 4.09l.01-.01zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"/>
            </svg>` :
            `<svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 18c0 .55.45 1 1 1h1v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h2v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h1c.55 0 1-.45 1-1V8H6v10zM3.5 8C2.67 8 2 8.67 2 9.5v7c0 .83.67 1.5 1.5 1.5S5 17.33 5 16.5v-7C5 8.67 4.33 8 3.5 8zm17 0c-.83 0-1.5.67-1.5 1.5v7c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5v-7c0-.83-.67-1.5-1.5-1.5zm-4.97-5.84l1.3-1.3c.2-.2.2-.51 0-.71-.2-.2-.51-.2-.71 0l-1.48 1.48C13.85 1.23 12.95 1 12 1c-.96 0-1.86.23-2.66.63L7.85.15c-.2-.2-.51-.2-.71 0-.2.2-.2.51 0 .71l1.31 1.31C6.97 3.26 6 5.01 6 7h12c0-1.99-.97-3.75-2.47-4.84zM10 5H9V4h1v1zm5 0h-1V4h1v1z"/>
            </svg>`;

        // 构建连接状态标签HTML（与device-scanner.js完全一致）
        let connectionBadgeHtml = '';
        if (isConnected) {
            if (isWifi) {
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
            if (isWifi) {
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
        const deviceIdText = platform === 'android' ?
            (config.deviceId || config.deviceName) :
            (config.udid || config.deviceName);

        let deviceInfoHtml = '';
        if (config.deviceName) {
            // 有设备名称：显示设备名称作为昵称，下面显示ID和model
            const deviceIdPart = `<div class="device-id-small">${deviceIdText}</div>`;
            const modelPart = config.model ? `<div class="device-model-small">${config.model}</div>` : '';
            deviceInfoHtml = `
                <div class="device-platform-icon">
                    ${platformIconSvg}
                </div>
                <div class="device-name">${config.deviceName}</div>
                ${deviceIdPart}
                ${modelPart}
            `;
        } else {
            // 无设备名称：显示设备ID作为昵称，下面显示model
            const modelPart = config.model ? `<div class="device-model-small">${config.model}</div>` : '';
            deviceInfoHtml = `
                <div class="device-platform-icon">
                    ${platformIconSvg}
                </div>
                <div class="device-name">${deviceIdText}</div>
                ${modelPart}
            `;
        }

        const card = document.createElement('div');
        card.className = 'device-phone-mockup';
        card.setAttribute('data-db-id', id); // 数据库ID用于去重检查
        // 同时设置 data-device-id 用于与未保存设备的去重
        const deviceKey = platform === 'ios' ? config.udid : config.deviceId;
        if (deviceKey) {
            card.setAttribute('data-device-id', deviceKey);
        }

        let cardContent = `
            ${connectionBadgeHtml}
            <div class="device-screen-content">
                <div class="device-info-text">
                    ${deviceInfoHtml}
                </div>
            </div>
        `;

        // 获取并显示当前App信息作为hover浮层（仅已连接的Android设备）
        if (isConnected && platform === 'android') {
            const deviceId = config.deviceId;
            if (deviceId) {
                const deviceIdClean = deviceId.replace(/[^a-zA-Z0-9]/g, '_');
                // 添加一个空的浮层容器,在hover时动态加载
                cardContent += `
                    <div class="device-app-info" id="device-app-info-${deviceIdClean}" data-device-id="${deviceId}">
                        <div class="device-app-info-title">
                            <span>当前运行应用</span>
                            <button class="btn-view-all-apps" onclick="openAppListModal('${deviceId}')" title="查看所有App">
                                全部应用
                            </button>
                        </div>
                        <div class="device-app-info-content">
                            <div class="device-app-loading">
                                <div class="device-app-spinner"></div>
                            </div>
                        </div>
                    </div>
                `;
            }
        }

        // 添加操作按钮（已保存的设备）
        const actionButtons = [];

        if (isWifi && isConnected) {
            // WiFi已连接/已保存: 断开 + 编辑 + 删除
            actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${config.ipAddress}', ${config.port || 5555})">断开</button>`);
        }

        // 已保存的设备都显示编辑和删除按钮
        // USB已连接/已保存: 编辑 + 删除
        // WiFi已连接/已保存: 断开 + 编辑 + 删除
        // USB未连接/已保存: 编辑 + 删除
        // WiFi未连接/已保存: 编辑 + 删除
        actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="editDevice('${id}')">编辑</button>`);
        actionButtons.push(`<button class="btn btn-outline btn-small" onclick="deleteDevice('${id}')" title="删除">删除</button>`);

        if (actionButtons.length > 0) {
            cardContent += `
                <div class="device-actions">
                    ${actionButtons.join('\n')}
                </div>
            `;
        }

        card.innerHTML = cardContent;

        // 为保存的设备卡片添加拖拽功能
        let deviceIdentifier = null;

        if (platform === 'ios') {
            // iOS设备使用udid
            deviceIdentifier = config.udid;
        } else {
            // Android设备使用deviceId
            deviceIdentifier = config.deviceId;

            // 如果没有deviceId但设备已连接，尝试从连接的设备中查找
            if (!deviceIdentifier && isConnected && connectedDevices.length > 0) {
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

        // 为Android设备添加拖拽功能（已连接和未连接都添加，但未连接会禁用）
        if (platform === 'android' && deviceIdentifier && window.setupDragAndDropForDevice) {
            window.setupDragAndDropForDevice(card, deviceIdentifier, isConnected, isWifi);
            window.rLog('为保存的设备添加拖拽功能:', config.deviceName, deviceIdentifier, '连接状态:', isConnected, 'WiFi:', isWifi);
        }

        gridElement.appendChild(card);
    }
}

// 刷新下拉框中的设备列表
async function refreshDeviceList() {
    const { ipcRenderer } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;
    const result = await ipcRenderer.invoke('adb-devices');
    const deviceSelect = document.getElementById('deviceSelect');

    if (!deviceSelect) return;

    // 获取保存的设备以恢复选择
    const savedSelection = await ipcRenderer.invoke('store-get', 'selected_device');

    deviceSelect.innerHTML = '<option value="">Select Device</option>';

    if (result.success && result.devices.length > 0) {
        // 从数据库加载保存的设备配置以显示名称而不是ID
        let deviceConfigs = {};
        if (projectPath) {
            try {
                const dbResult = await ipcRenderer.invoke('db-device-getAll', projectPath);
                if (dbResult.success && dbResult.data) {
                    for (const config of dbResult.data) {
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

// 全局函数：打开App列表模态窗
window.openAppListModal = function(deviceId) {
    window.rLog('打开App列表模态窗:', deviceId);
    // 触发打开模态窗事件
    if (window.AppListModal && window.AppListModal.open) {
        window.AppListModal.open(deviceId);
    }
};
