// 设备扫描和状态检测模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：loadSavedDevices, refreshDeviceList, setupDragAndDropForDevice

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 刷新连接的设备（按平台分组显示 Android 和 iOS 设备）
async function refreshConnectedDevices() {
    const { ipcRenderer } = getGlobals();

    const iosDevicesGrid = document.getElementById('iosDevicesGrid');
    const androidDevicesGrid = document.getElementById('androidDevicesGrid');

    if (!iosDevicesGrid || !androidDevicesGrid) return;

    // 清空两个容器
    iosDevicesGrid.innerHTML = '';
    androidDevicesGrid.innerHTML = '';

    // 使用新的 TKE categorized API 获取分类后的设备列表
    // 如果有打开的项目,传递项目路径以加载已保存的设备配置
    // 如果没有项目,只显示已连接的设备
    try {
        const categorizedResult = await ipcRenderer.invoke('tke-controller-devices', window.AppGlobals.currentProject);

        if (!categorizedResult.success) {
            window.rError('获取分类设备列表失败:', categorizedResult.error);
            return;
        }

        // 解析返回的 JSON
        const categorized = JSON.parse(categorizedResult.output);
        window.rLog('获取到分类设备:', categorized);

        // 从数据库加载保存的设备配置
        let savedDeviceConfigs = {};
        if (window.AppGlobals.currentProject) {
            try {
                const dbResult = await ipcRenderer.invoke('db-device-getAll', window.AppGlobals.currentProject);
                if (dbResult.success && dbResult.data) {
                    for (const config of dbResult.data) {
                        // Android 用 deviceId, iOS 用 udid
                        if (config.deviceId) {
                            savedDeviceConfigs[config.deviceId] = config;
                        }
                        if (config.udid) {
                            savedDeviceConfigs[config.udid] = config;
                        }
                    }
                }
            } catch (error) {
                window.rError('加载保存的设备配置失败:', error);
            }
        }

        // 获取完整的设备信息（用于获取 model 等详细信息）
        const [androidResult, iosResult] = await Promise.all([
            ipcRenderer.invoke('adb-devices'),
            ipcRenderer.invoke('get-ios-devices')
        ]);

        const androidDevicesMap = new Map();
        const iosDevicesMap = new Map();

        if (androidResult.success) {
            androidResult.devices.forEach(device => {
                androidDevicesMap.set(device.id, device);
            });
        }

        if (iosResult.success) {
            iosResult.devices.forEach(device => {
                iosDevicesMap.set(device.udid, device);
            });
        }

        // 按顺序渲染 Android 设备: unsaved_connected - saved_connected - saved_disconnected
        await renderCategorizedDevices(
            categorized.android,
            'android',
            androidDevicesGrid,
            androidDevicesMap,
            savedDeviceConfigs
        );

        // 按顺序渲染 iOS 设备: unsaved_connected - saved_connected - saved_disconnected
        await renderCategorizedDevices(
            categorized.ios,
            'ios',
            iosDevicesGrid,
            iosDevicesMap,
            savedDeviceConfigs
        );

    } catch (error) {
        window.rError('刷新设备列表失败:', error);
    }

    // 更新设备选择下拉框
    if (window.DeviceLoader?.refreshDeviceList) {
        window.DeviceLoader.refreshDeviceList();
    }
}

// 渲染分类后的设备列表（按顺序: unsaved_connected - saved_connected - saved_disconnected）
async function renderCategorizedDevices(categorizedGroup, platform, gridElement, devicesMap, savedDeviceConfigs) {
    const { ipcRenderer } = getGlobals();

    // 按顺序渲染三个分组
    const groups = [
        { devices: categorizedGroup.unsaved_connected, isSaved: false, isConnected: true },
        { devices: categorizedGroup.saved_connected, isSaved: true, isConnected: true },
        { devices: categorizedGroup.saved_disconnected, isSaved: true, isConnected: false }
    ];

    for (const group of groups) {
        for (const deviceId of group.devices) {
            // 从完整设备列表中获取详细信息
            const deviceInfo = devicesMap.get(deviceId);
            const savedConfig = savedDeviceConfigs[deviceId];

            // 判断连接类型
            let isWifi = false;
            if (platform === 'android' && deviceId.includes(':')) {
                isWifi = true;
            } else if (platform === 'ios' && savedConfig?.connectionType === 'wifi') {
                isWifi = true;
            }

            await renderDeviceCard(
                deviceId,
                platform,
                group.isConnected,
                group.isSaved,
                isWifi,
                gridElement,
                deviceInfo,
                savedConfig
            );
        }
    }
}

// 渲染单个设备卡片
async function renderDeviceCard(deviceId, platform, isConnected, isSaved, isWifi, gridElement, deviceInfo, savedConfig) {
    const { ipcRenderer } = getGlobals();

    const item = document.createElement('div');
    item.className = 'device-phone-mockup';
    item.setAttribute('data-device-id', deviceId);

    // 获取平台图标 SVG
    const platformIconSvg = platform === 'ios' ?
        `<svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M17.05 20.28c-.98.95-2.05.8-3.08.35-1.09-.46-2.09-.48-3.24 0-1.44.62-2.2.44-3.06-.35C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.54 4.09l.01-.01zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z"/>
        </svg>` :
        `<svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M6 18c0 .55.45 1 1 1h1v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h2v3.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5V19h1c.55 0 1-.45 1-1V8H6v10zM3.5 8C2.67 8 2 8.67 2 9.5v7c0 .83.67 1.5 1.5 1.5S5 17.33 5 16.5v-7C5 8.67 4.33 8 3.5 8zm17 0c-.83 0-1.5.67-1.5 1.5v7c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5v-7c0-.83-.67-1.5-1.5-1.5zm-4.97-5.84l1.3-1.3c.2-.2.2-.51 0-.71-.2-.2-.51-.2-.71 0l-1.48 1.48C13.85 1.23 12.95 1 12 1c-.96 0-1.86.23-2.66.63L7.85.15c-.2-.2-.51-.2-.71 0-.2.2-.2.51 0 .71l1.31 1.31C6.97 3.26 6 5.01 6 7h12c0-1.99-.97-3.75-2.47-4.84zM10 5H9V4h1v1zm5 0h-1V4h1v1z"/>
        </svg>`;

    // 构建连接状态标签
    let connectionBadgeHtml = '';
    if (isConnected) {
        if (isWifi) {
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
    if (isSaved && savedConfig?.deviceName) {
        // 已保存：显示设备名称，下面显示ID和model
        const deviceIdPart = `<div class="device-id-small">${deviceId}</div>`;
        const modelPart = deviceInfo?.model ? `<div class="device-model-small">${deviceInfo.model}</div>` : '';
        deviceInfoHtml = `
            <div class="device-platform-icon">
                ${platformIconSvg}
            </div>
            <div class="device-name">${savedConfig.deviceName}</div>
            ${deviceIdPart}
            ${modelPart}
        `;
    } else {
        // 未保存：显示设备ID，下面显示model
        const modelPart = deviceInfo?.model ? `<div class="device-model-small">${deviceInfo.model}</div>` : '';
        deviceInfoHtml = `
            <div class="device-platform-icon">
                ${platformIconSvg}
            </div>
            <div class="device-name">${deviceId}</div>
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

    // 获取并显示当前App信息（仅已连接的Android设备）
    if (isConnected && platform === 'android') {
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

    // 添加操作按钮
    const actionButtons = [];
    const hasProject = !!window.AppGlobals.currentProject;

    if (!isSaved) {
        // 未保存的设备
        if (isWifi && isConnected) {
            // WiFi已连接/未保存: 断开 + (有项目时显示保存)
            const port = deviceId.split(':')[1] || 5555;
            const host = deviceId.split(':')[0];
            actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${host}', ${port})">断开</button>`);
        }
        // 只有打开项目时才显示保存按钮
        if (hasProject) {
            actionButtons.push(`<button class="btn btn-primary btn-small" onclick="createDeviceFromConnected('${deviceId}')">保存</button>`);
        }
        // 已连接的设备显示"信息"按钮
        if (isConnected && platform === 'android') {
            actionButtons.push(`<button class="btn btn-default btn-small" onclick="showDeviceInfoModal('${deviceId}')" title="查看详细信息">信息</button>`);
        }
    } else {
        // 已保存的设备
        if (isWifi && isConnected) {
            // WiFi已连接/已保存: 断开 + 编辑 + 删除
            const port = deviceId.split(':')[1] || 5555;
            const host = deviceId.split(':')[0];
            actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${host}', ${port})">断开</button>`);
        }
        // 已保存的设备都显示编辑和删除按钮（使用数据库ID）
        actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="editDevice('${savedConfig?.id || ''}')">编辑</button>`);
        actionButtons.push(`<button class="btn btn-outline btn-small" onclick="deleteDevice('${savedConfig?.id || ''}')" title="删除">删除</button>`);
        // 已连接的设备显示"信息"按钮
        if (isConnected && platform === 'android') {
            actionButtons.push(`<button class="btn btn-default btn-small" onclick="showDeviceInfoModal('${deviceId}')" title="查看详细信息">信息</button>`);
        }
    }

    if (platform === 'ios' && !isConnected && !isSaved && hasProject) {
        actionButtons.push(`<button class="btn btn-secondary btn-small" onclick="showWdaSetupGuide('${deviceId}')">WDA设置</button>`);
    }

    if (actionButtons.length > 0) {
        cardContent += `
            <div class="device-actions">
                ${actionButtons.join('\n')}
            </div>
        `;
    }

    item.innerHTML = cardContent;

    // 为已连接的Android设备添加hover加载当前App信息的功能
    if (isConnected && platform === 'android') {
        item.addEventListener('mouseover', async function(e) {
            // 检查事件是否来自浮层内部，避免DOM更新时触发无限循环
            const isFromAppInfo = e.target.closest('.device-app-info');
            if (isFromAppInfo) return;

            const appInfoDiv = item.querySelector('.device-app-info');
            if (!appInfoDiv) return;

            // 防止重复请求
            if (item.dataset.appLoading === 'true') return;

            const contentDiv = appInfoDiv.querySelector('.device-app-info-content');
            if (!contentDiv) return;

            // 标记为加载中
            item.dataset.appLoading = 'true';

            // 显示加载动画
            contentDiv.innerHTML = '<div class="device-app-loading"><div class="device-app-spinner"></div></div>';

            try {
                const { ipcRenderer } = getGlobals();
                window.rLog('开始加载设备当前App信息:', deviceId);
                const result = await ipcRenderer.invoke('tke-app-focus', { deviceId });
                window.rLog('tke-app-focus 返回结果:', result);

                if (result.success) {
                    const data = JSON.parse(result.output);
                    window.rLog('解析的数据:', data);
                    const deviceIdClean = deviceId.replace(/[^a-zA-Z0-9]/g, '_');

                    const fieldsHtml = `
                        <div class="device-app-field">
                            <span class="device-app-label">包名</span>
                            <span class="device-app-value" title="${data.package_name}">${data.package_name}</span>
                            <button class="device-app-copy-btn" onclick="copyToClipboard('${deviceIdClean}_package')" title="复制包名">
                                <svg viewBox="0 0 24 24">
                                    <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                </svg>
                            </button>
                            <span id="${deviceIdClean}_package" style="display: none;">${data.package_name}</span>
                        </div>
                        <div class="device-app-field">
                            <span class="device-app-label">Activity</span>
                            <span class="device-app-value" title="${data.activity_name}">${data.activity_name}</span>
                            <button class="device-app-copy-btn" onclick="copyToClipboard('${deviceIdClean}_activity')" title="复制Activity">
                                <svg viewBox="0 0 24 24">
                                    <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                </svg>
                            </button>
                            <span id="${deviceIdClean}_activity" style="display: none;">${data.activity_name}</span>
                        </div>
                    `;
                    contentDiv.innerHTML = fieldsHtml;
                } else {
                    window.rError('加载失败:', result.error);
                    contentDiv.innerHTML = '<div class="device-app-error">加载失败: ' + (result.error || '未知错误') + '</div>';
                }
            } catch (error) {
                window.rError('加载当前App信息异常:', error);
                contentDiv.innerHTML = '<div class="device-app-error">加载失败: ' + error.message + '</div>';
            } finally {
                // 清除loading标记
                item.dataset.appLoading = 'false';
            }
        });
    }

    // 添加拖拽功能（Android设备）
    if (platform === 'android' && window.setupDragAndDropForDevice) {
        window.setupDragAndDropForDevice(item, deviceId, isConnected, isWifi);
    }

    gridElement.appendChild(item);
}

// 检查 iOS 设备的 WDA 连接状态
async function checkIosDeviceStatus(device) {
    const { ipcRenderer } = getGlobals();

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
        // USB 连接失败，尝试从数据库获取 WiFi 配置
        try {
            if (window.AppGlobals.currentProject) {
                const dbResult = await ipcRenderer.invoke('db-device-getAll', window.AppGlobals.currentProject);
                if (dbResult.success && dbResult.data) {
                    for (const config of dbResult.data) {
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
