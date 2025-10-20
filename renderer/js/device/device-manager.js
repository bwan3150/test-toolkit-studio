// 设备管理模块

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 动态加载连接向导模态框
async function loadConnectionGuideModal() {
    const container = document.getElementById('connectionGuideModalContainer');
    if (!container) return;
    
    try {
        const response = await fetch('modals/connection-guide-modal.html');
        if (response.ok) {
            const html = await response.text();
            container.innerHTML = html;
        }
    } catch (error) {
        window.rError('Failed to load connection guide modal:', error);
    }
}

// 初始化设备页面
async function initializeDevicePage() {
    const { fs, yaml } = getGlobals();
    
    // 动态加载连接向导模态框
    await loadConnectionGuideModal();
    
    const addDeviceBtn = document.getElementById('addDeviceBtn');
    const scanDevicesBtn = document.getElementById('scanDevicesBtn');
    const deviceForm = document.getElementById('deviceForm');
    const newDeviceForm = document.getElementById('newDeviceForm');
    const cancelDeviceBtn = document.getElementById('cancelDeviceBtn');
    
    // 连接向导相关元素（加载后才能获取）
    const connectDeviceBtn = document.getElementById('connectDeviceBtn');
    const connectionGuideModal = document.getElementById('connectionGuideModal');
    const closeGuideBtn = document.getElementById('closeGuideBtn');
    
    // 配对相关元素
    const generateQrBtn = document.getElementById('generateQrBtn');
    const refreshQrBtn = document.getElementById('refreshQrBtn');
    const connectWithPairingCodeBtn = document.getElementById('connectWithPairingCodeBtn');
    const showPairingCodeBtn = document.getElementById('showPairingCodeBtn');
    const showQrCodeBtn = document.getElementById('showQrCodeBtn');
    
    if (addDeviceBtn) {
        addDeviceBtn.addEventListener('click', () => {
            const isHidden = deviceForm.style.display === 'none' || deviceForm.style.display === '';
            deviceForm.style.display = isHidden ? 'block' : 'none';
            // 重置表单并设置默认值
            if (isHidden) {
                newDeviceForm.reset();
                delete deviceForm.dataset.mode;
                delete deviceForm.dataset.filename;
                deviceForm.querySelector('h3').textContent = 'Add New Device';
                updatePlatformFields();
                // 滚动到表单顶部
                deviceForm.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });
    }
    
    if (cancelDeviceBtn) {
        cancelDeviceBtn.addEventListener('click', () => {
            deviceForm.style.display = 'none';
            newDeviceForm.reset();
        });
    }
    
    if (scanDevicesBtn) {
        scanDevicesBtn.addEventListener('click', refreshConnectedDevices);
    }
    
    
    // 平台选择事件
    const platformRadios = document.querySelectorAll('input[name="platform"]');
    platformRadios.forEach(radio => {
        radio.addEventListener('change', updatePlatformFields);
    });
    
    // 连接类型选择事件
    const connectionRadios = document.querySelectorAll('input[name="connectionType"]');
    connectionRadios.forEach(radio => {
        radio.addEventListener('change', updatePlatformFields);
    });
    
    // 连接设备向导
    if (connectDeviceBtn) {
        connectDeviceBtn.addEventListener('click', showConnectionGuide);
    }
    
    if (closeGuideBtn) {
        closeGuideBtn.addEventListener('click', hideConnectionGuide);
    }
    
    // 点击模态框外部关闭
    if (connectionGuideModal) {
        connectionGuideModal.addEventListener('click', (e) => {
            if (e.target === connectionGuideModal) {
                hideConnectionGuide();
            }
        });
    }
    
    // 配对方式选择
    if (showPairingCodeBtn) {
        showPairingCodeBtn.addEventListener('click', () => showPairingMethod('code'));
    }
    
    if (showQrCodeBtn) {
        showQrCodeBtn.addEventListener('click', () => showPairingMethod('qr'));
    }
    
    // 配对码连接
    if (connectWithPairingCodeBtn) {
        connectWithPairingCodeBtn.addEventListener('click', connectWithPairingCode);
    }
    
    // QR码生成
    if (generateQrBtn) {
        generateQrBtn.addEventListener('click', generateQRCode);
    }
    
    if (refreshQrBtn) {
        refreshQrBtn.addEventListener('click', generateQRCode);
    }
    
    // 连接方式切换
    initializeMethodSwitcher();
    
    // 连接向导标签页切换
    initializeConnectionTabs();
    
    // IP地址同步
    initializeIpSync();
    
    if (newDeviceForm) {
        newDeviceForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            window.rLog('表单提交事件触发');
            
            if (!window.AppGlobals.currentProject) {
                window.NotificationModule.showNotification('Please open a project first', 'warning');
                return;
            }
            
            const formData = new FormData(newDeviceForm);
            const deviceConfig = {};
            
            for (const [key, value] of formData.entries()) {
                deviceConfig[key] = value === 'true' ? true : value === 'false' ? false : value;
            }
            
            window.rLog('收集到的表单数据:', deviceConfig);
            
            // 根据平台处理连接信息
            if (deviceConfig.connectionType === 'wifi') {
                if (deviceConfig.platform === 'ios') {
                    // iOS WiFi: 使用 wdaIpAddress 和 wdaPort
                    if (deviceConfig.wdaIpAddress) {
                        deviceConfig.ipAddress = deviceConfig.wdaIpAddress;
                        deviceConfig.port = deviceConfig.wdaPort;
                        const port = deviceConfig.wdaPort || '';
                        if (port) {
                            deviceConfig.deviceId = `${deviceConfig.wdaIpAddress}:${port}`;
                        } else {
                            deviceConfig.deviceId = deviceConfig.wdaIpAddress;
                        }
                    }
                } else {
                    // Android WiFi: 使用 ipAddress 和 port
                    if (deviceConfig.ipAddress) {
                        const port = deviceConfig.port || '';
                        if (port) {
                            deviceConfig.deviceId = `${deviceConfig.ipAddress}:${port}`;
                        } else {
                            deviceConfig.deviceId = deviceConfig.ipAddress;
                        }
                    }
                }
            }
            
            // 根据平台自动设置platformName
            deviceConfig.platformName = deviceConfig.platform === 'ios' ? 'iOS' : 'Android';
            
            // 检查是否在编辑模式
            const deviceForm = document.getElementById('deviceForm');
            const mode = deviceForm.dataset.mode;
            let devicePath;
            
            if (mode === 'edit' && deviceForm.dataset.filename) {
                // 编辑模式 - 使用现有文件名
                devicePath = path.join(window.AppGlobals.currentProject, 'devices', deviceForm.dataset.filename);
            } else {
                // 创建模式 - 生成新文件名
                const timestamp = Date.now();
                const deviceFileName = `device_${timestamp}.yaml`;
                devicePath = path.join(window.AppGlobals.currentProject, 'devices', deviceFileName);
            }
            
            try {
                await fs.writeFile(devicePath, yaml.dump(deviceConfig));
                window.NotificationModule.showNotification(
                    mode === 'edit' ? 'Device updated successfully' : 'Device saved successfully',
                    'success'
                );
                
                deviceForm.style.display = 'none';
                newDeviceForm.reset();
                delete deviceForm.dataset.mode;
                delete deviceForm.dataset.filename;
                
                await loadSavedDevices();
                await refreshDeviceList();
            } catch (error) {
                window.NotificationModule.showNotification(`Failed to save device: ${error.message}`, 'error');
            }
        });
    }
    
    // 加载保存的设备
    loadSavedDevices();
    
    // 确保页面加载时字段显示正确
    updatePlatformFields();
    
    // 监听配对成功事件
    const { ipcRenderer } = getGlobals();
    ipcRenderer.on('pairing-success', (event, data) => {
        window.rLog('收到配对成功事件:', data);
        window.NotificationModule.showNotification(
            `配对成功！来自设备: ${data.remoteAddress}`, 
            'success'
        );
        
        // 重置QR码显示
        setTimeout(() => {
            resetQRDisplay();
            refreshConnectedDevices();
        }, 2000);
    });
}

// 更新连接类型字段显示（简化版本，主要逻辑已移至 updatePlatformFields）
function updateConnectionTypeFields() {
    // 触发平台字段更新，因为连接类型会影响字段显示
    updatePlatformFields();
}

// 更新平台字段显示 - 使用组合键方式
function updatePlatformFields() {
    const platformRadio = document.querySelector('input[name="platform"]:checked');
    const connectionRadio = document.querySelector('input[name="connectionType"]:checked');
    
    if (!platformRadio || !connectionRadio) return;
    
    const platform = platformRadio.value;
    const connectionType = connectionRadio.value;
    
    // 生成组合键
    const configKey = `${platform}-${connectionType}`;
    
    // 隐藏所有配置组并禁用required属性
    document.querySelectorAll('.config-group').forEach(group => {
        group.style.display = 'none';
        // 禁用隐藏组中的required字段
        const requiredInputs = group.querySelectorAll('input[required]');
        requiredInputs.forEach(input => {
            input.disabled = true;
        });
    });
    
    // 显示对应的配置组并启用required属性
    const targetConfig = document.getElementById(`${configKey}-config`);
    if (targetConfig) {
        targetConfig.style.display = 'block';
        // 启用显示组中的required字段
        const requiredInputs = targetConfig.querySelectorAll('input[required]');
        requiredInputs.forEach(input => {
            input.disabled = false;
        });
        window.rLog(`显示配置组: ${configKey}-config`);
    } else {
        console.warn(`未找到配置组: ${configKey}-config`);
    }
    
    window.rLog(`更新字段显示: ${configKey}`);
}

// 切换高级设置显示
function toggleAdvancedSettings() {
    const content = document.getElementById('advancedSettingsContent');
    const toggle = document.querySelector('.advanced-toggle');
    const icon = toggle.querySelector('.toggle-icon');
    
    if (content.style.display === 'none') {
        content.style.display = 'block';
        icon.style.transform = 'rotate(180deg)';
    } else {
        content.style.display = 'none';
        icon.style.transform = 'rotate(0deg)';
    }
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
                
                const card = document.createElement('div');
                card.className = 'device-card';
                card.innerHTML = `
                    <div class="device-card-header">
                        <div class="device-status ${isConnected ? 'connected' : ''}" title="${isConnected ? 'Connected' : 'Not Connected'}"></div>
                        <div class="device-name">${config.deviceName}</div>
                        <span class="connection-type-badge ${isWifi ? 'wifi' : 'usb'}">${isWifi ? 'WiFi' : 'USB'}</span>
                    </div>
                    <div class="device-info">
                        <div>${config.platformName} ${config.platformVersion}</div>
                        ${isWifi ? 
                            `<div class="device-id">IP: ${config.ipAddress}:${config.port || 5555}</div>` : 
                            (config.deviceId ? `<div class="device-id">ID: ${config.deviceId}</div>` : '')
                        }
                    </div>
                    <div class="device-actions">
                        ${isWifi && !isConnected ? `
                            <button class="btn btn-primary btn-small" onclick="connectWirelessDevice('${config.ipAddress}', ${config.port || 5555})">
                                <svg viewBox="0 0 24 24" width="14" height="14">
                                    <path fill="currentColor" d="M8 3a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8z"/>
                                </svg>
                                Connect
                            </button>
                        ` : ''}
                        ${isWifi && isConnected ? `
                            <button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${config.ipAddress}', ${config.port || 5555})">
                                <svg viewBox="0 0 24 24" width="14" height="14">
                                    <path fill="currentColor" d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                                </svg>
                                Disconnect
                            </button>
                        ` : ''}
                        <button class="btn btn-secondary btn-small" onclick="editDevice('${file}')">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                            </svg>
                            Edit
                        </button>
                        <button class="btn btn-outline btn-small btn-icon-only" onclick="deleteDevice('${file}')" title="Delete">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
                            </svg>
                        </button>
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
                    setupDragAndDropForDevice(card, deviceIdentifier);
                    window.rLog('为保存的设备添加拖拽功能:', config.deviceName, deviceIdentifier);
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
            item.className = 'device-card';
            
            // 创建卡片内容
            const deviceName = savedConfig ? savedConfig.deviceName : (device.name || '未知设备');
            const platformBadge = device.platform === 'ios' ? 'iOS' : 'Android';
            
            // 设置设备状态
            let deviceStatus = 'connected';
            let statusTitle = 'Connected';
            
            if (device.platform === 'ios' && device.wdaStatus !== 'connected') {
                deviceStatus = 'disconnected';
                statusTitle = 'iOS Device Found - WDA Not Available';
            }

            let cardContent = `
                <div class="device-card-header">
                    <div class="device-status ${deviceStatus}" title="${statusTitle}"></div>
                    <div class="device-name">${deviceName}</div>
                    <span class="platform-badge ${device.platform}">${platformBadge}</span>
                    <span class="connection-type-badge ${isWifi ? 'wifi' : 'usb'} ${device.platform === 'ios' && device.wdaStatus !== 'connected' ? 'offline' : ''}">${connectionType}</span>
                </div>
                <div class="device-info">
                    <div class="device-id">ID: ${device.id}</div>
                    ${device.model ? `<div class="device-model">Model: ${device.model}</div>` : ''}
                    ${device.version ? `<div class="device-version">Version: ${device.version}</div>` : ''}
                    ${device.platform === 'ios' && device.wdaStatus !== 'connected' ? 
                        `<div class="device-status-info">WDA连接失败: ${device.wdaError || 'WDA service not available'}</div>` : ''}
                </div>
            `;
            
            // 获取并显示当前App信息
            try {
                const appResult = await ipcRenderer.invoke('get-current-app', device.id);
                if (appResult.success) {
                    cardContent += `
                        <div class="device-card-body">
                            <div class="app-section">
                                <div class="app-title">当前运行应用</div>
                                <div class="app-details">
                                    <div class="app-field">
                                        <span class="field-label">PACKAGE</span>
                                        <div class="field-value-group">
                                            <span class="field-value">${appResult.packageName}</span>
                                            <button class="copy-btn" onclick="copyToClipboard('package-${device.id.replace(/[^a-zA-Z0-9]/g, '_')}')" title="复制">
                                                <svg viewBox="0 0 24 24" width="14" height="14">
                                                    <path fill="currentColor" d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                                </svg>
                                            </button>
                                            <span id="package-${device.id.replace(/[^a-zA-Z0-9]/g, '_')}" style="display: none;">${appResult.packageName}</span>
                                        </div>
                                    </div>
                                    <div class="app-field">
                                        <span class="field-label">Activity</span>
                                        <div class="field-value-group">
                                            <span class="field-value">${appResult.activityName}</span>
                                            <button class="copy-btn" onclick="copyToClipboard('activity-${device.id.replace(/[^a-zA-Z0-9]/g, '_')}')" title="复制">
                                                <svg viewBox="0 0 24 24" width="14" height="14">
                                                    <path fill="currentColor" d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                                </svg>
                                            </button>
                                            <span id="activity-${device.id.replace(/[^a-zA-Z0-9]/g, '_')}" style="display: none;">${appResult.activityName}</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    `;
                } else {
                    cardContent += `
                        <div class="device-card-body">
                            <div class="app-section">
                                <div class="no-app-info">无法获取应用信息</div>
                            </div>
                        </div>
                    `;
                }
            } catch (error) {
                cardContent += `
                    <div class="device-card-body">
                        <div class="app-section">
                            <div class="no-app-info">获取应用信息失败</div>
                        </div>
                    </div>
                `;
            }
            
            // 添加操作按钮
            cardContent += `
                <div class="device-actions">
                    ${!isSaved ? `
                        <button class="btn btn-primary btn-small" onclick="createDeviceFromConnected('${device.id}')">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                            </svg>
                            Save Config
                        </button>
                    ` : ''}
                    ${device.platform === 'ios' && device.wdaStatus !== 'connected' ? `
                        <button class="btn btn-secondary btn-small" onclick="showWdaSetupGuide('${device.id}')">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z"/>
                            </svg>
                            WDA Guide
                        </button>
                    ` : ''}
                    ${isWifi ? `
                        <button class="btn btn-secondary btn-small" onclick="disconnectWirelessDevice('${device.id.split(':')[0]}', ${device.id.split(':')[1] || 5555})">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                            </svg>
                            Disconnect
                        </button>
                    ` : ''}
                </div>
            `;
            
            item.innerHTML = cardContent;
            
            // 添加拖拽功能
            setupDragAndDropForDevice(item, device.id);
            
            connectedDevicesGrid.appendChild(item);
        }
    } else {
        connectedDevicesGrid.innerHTML = '<div class="text-muted">No devices connected</div>';
    }
    
    // 同样重新加载保存的设备以更新连接状态
    await loadSavedDevices();
    
    // 更新设备选择下拉框
    refreshDeviceList();
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
        }
    }
}

// 从连接的设备创建设备
async function createDeviceFromConnected(deviceId) {
    if (!window.AppGlobals.currentProject) {
        window.NotificationModule.showNotification('Please open a project first', 'warning');
        return;
    }
    
    // 显示预填充设备ID的设备表单
    const deviceForm = document.getElementById('deviceForm');
    const newDeviceForm = document.getElementById('newDeviceForm');
    
    // 重置表单并设置模式
    newDeviceForm.reset();
    delete deviceForm.dataset.mode;
    delete deviceForm.dataset.filename;
    
    // 预填充表单
    if (newDeviceForm) {
        // 判断是否为WiFi设备
        const isWifi = deviceId.includes(':');
        
        // 设置平台（默认Android）
        const androidRadio = newDeviceForm.querySelector('input[name="platform"][value="android"]');
        if (androidRadio) {
            androidRadio.checked = true;
        }
        
        // 设置连接类型
        const connectionType = isWifi ? 'wifi' : 'usb';
        const connectionRadio = newDeviceForm.querySelector(`input[name="connectionType"][value="${connectionType}"]`);
        if (connectionRadio) {
            connectionRadio.checked = true;
        }
        
        if (isWifi) {
            const [ip, port] = deviceId.split(':');
            newDeviceForm.querySelector('input[name="deviceName"]').value = `WiFi Device (${ip})`;
        } else {
            newDeviceForm.querySelector('input[name="deviceName"]').value = `Device ${deviceId.substring(0, 8)}`;
        }
        
        // 设置平台版本默认值
        newDeviceForm.querySelector('input[name="platformVersion"]').value = '14';
        
        // 填充高级设置
        const automationNameInput = newDeviceForm.querySelector('input[name="automationName"]');
        if (automationNameInput) {
            automationNameInput.value = 'UiAutomator2';
        }
        
        const timeoutInput = newDeviceForm.querySelector('input[name="newCommandTimeout"]');
        if (timeoutInput) {
            timeoutInput.value = '6000';
        }
        
        const noResetSelect = newDeviceForm.querySelector('select[name="noReset"]');
        if (noResetSelect) {
            noResetSelect.value = 'true';
        }
        
        // 更新表单字段显示
        updatePlatformFields();
        
        // 延迟填充具体字段
        setTimeout(() => {
            const configKey = `android-${connectionType}`;
            const targetConfig = document.getElementById(`${configKey}-config`);
            
            if (targetConfig && isWifi) {
                const [ip, port] = deviceId.split(':');
                // Android WiFi 使用 ipAddress 和 port
                const ipInput = targetConfig.querySelector('input[name="ipAddress"]');
                if (ipInput) ipInput.value = ip;
                
                const portInput = targetConfig.querySelector('input[name="port"]');
                if (portInput) portInput.value = port || '';
            } else if (targetConfig && !isWifi) {
                const deviceIdInput = targetConfig.querySelector('input[name="deviceId"]');
                if (deviceIdInput) deviceIdInput.value = deviceId;
            }
        }, 100);
    }
    
    // 更新表单标题
    deviceForm.querySelector('h3').textContent = 'Add New Device';
    deviceForm.style.display = 'block';
    deviceForm.scrollIntoView({ behavior: 'smooth', block: 'start' });
    window.NotificationModule.showNotification('Please complete the device configuration', 'info');
}

// 编辑设备配置
async function editDevice(filename) {
    const { path, fs, yaml } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    try {
        const devicePath = path.join(window.AppGlobals.currentProject, 'devices', filename);
        const content = await fs.readFile(devicePath, 'utf-8');
        const config = yaml.load(content);
        
        // 显示带有现有数据的设备表单
        const deviceForm = document.getElementById('deviceForm');
        const newDeviceForm = document.getElementById('newDeviceForm');
        
        // 设置模式并存储文件名
        deviceForm.dataset.mode = 'edit';
        deviceForm.dataset.filename = filename;
        
        // 用现有数据填充表单
        if (newDeviceForm) {
            newDeviceForm.querySelector('input[name="deviceName"]').value = config.deviceName || '';
            newDeviceForm.querySelector('input[name="platformVersion"]').value = config.platformVersion || '';
            
            // 设置平台
            const platform = config.platform || 'android';
            const platformRadio = newDeviceForm.querySelector(`input[name="platform"][value="${platform}"]`);
            if (platformRadio) {
                platformRadio.checked = true;
            }
            
            // 设置连接类型
            const connectionType = config.connectionType || (config.ipAddress ? 'wifi' : 'usb');
            const connectionRadio = newDeviceForm.querySelector(`input[name="connectionType"][value="${connectionType}"]`);
            if (connectionRadio) {
                connectionRadio.checked = true;
            }
            
            // 填充高级设置
            const automationNameInput = newDeviceForm.querySelector('input[name="automationName"]');
            if (automationNameInput) {
                automationNameInput.value = config.automationName || 'UiAutomator2';
            }
            
            const timeoutInput = newDeviceForm.querySelector('input[name="newCommandTimeout"]');
            if (timeoutInput) {
                timeoutInput.value = config.newCommandTimeout || '6000';
            }
            
            const noResetSelect = newDeviceForm.querySelector('select[name="noReset"]');
            if (noResetSelect) {
                noResetSelect.value = config.noReset ? 'true' : 'false';
            }
            
            // 更新表单字段显示
            updatePlatformFields();
            
            // 延迟填充具体配置字段，确保对应的配置组已显示
            setTimeout(() => {
                const configKey = `${platform}-${connectionType}`;
                const targetConfig = document.getElementById(`${configKey}-config`);
                
                if (targetConfig) {
                    // 根据配置类型填充特定字段
                    if (connectionType === 'wifi') {
                        if (platform === 'ios') {
                            // iOS WiFi: 填充 wdaIpAddress 和 wdaPort
                            const wdaIpInput = targetConfig.querySelector('input[name="wdaIpAddress"]');
                            if (wdaIpInput) {
                                wdaIpInput.value = config.ipAddress || '';
                            }
                            
                            const wdaPortInput = targetConfig.querySelector('input[name="wdaPort"]');
                            if (wdaPortInput) {
                                wdaPortInput.value = config.port !== undefined ? String(config.port) : '';
                            }
                        } else {
                            // Android WiFi: 填充 ipAddress 和 port
                            const ipInput = targetConfig.querySelector('input[name="ipAddress"]');
                            if (ipInput) {
                                ipInput.value = config.ipAddress || '';
                            }
                            
                            const portInput = targetConfig.querySelector('input[name="port"]');
                            if (portInput) {
                                portInput.value = config.port !== undefined ? String(config.port) : '';
                            }
                        }
                    } else {
                        const deviceIdInput = targetConfig.querySelector('input[name="deviceId"]');
                        if (deviceIdInput) {
                            deviceIdInput.value = config.deviceId || '';
                        }
                    }
                    
                    // iOS特定字段
                    if (platform === 'ios') {
                        const udidInput = targetConfig.querySelector('input[name="udid"]');
                        if (udidInput) {
                            udidInput.value = config.udid || '';
                        }
                        
                        const bundleIdInput = targetConfig.querySelector('input[name="bundleId"]');
                        if (bundleIdInput) {
                            bundleIdInput.value = config.bundleId || '';
                        }
                    }
                }
            }, 100);
        }
        
        // 更新表单标题
        deviceForm.querySelector('h3').textContent = 'Edit Device';
        deviceForm.style.display = 'block';
    } catch (error) {
        window.NotificationModule.showNotification(`Failed to load device: ${error.message}`, 'error');
    }
}

// 删除设备配置
async function deleteDevice(filename) {
    const { path, fs } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    if (confirm('Are you sure you want to delete this device configuration?')) {
        try {
            const devicePath = path.join(window.AppGlobals.currentProject, 'devices', filename);
            await fs.unlink(devicePath);
            window.NotificationModule.showNotification('Device configuration deleted', 'success');
            await loadSavedDevices();
            await refreshDeviceList();
        } catch (error) {
            window.NotificationModule.showNotification(`Failed to delete device: ${error.message}`, 'error');
        }
    }
}


// 复制到剪贴板的辅助函数
function copyToClipboard(elementId) {
    const element = document.getElementById(elementId);
    if (element && element.textContent && element.textContent !== '-') {
        navigator.clipboard.writeText(element.textContent).then(() => {
            window.NotificationModule.showNotification('已复制到剪贴板', 'success');
        }).catch(err => {
            window.NotificationModule.showNotification('复制失败', 'error');
        });
    }
}

// 全局函数用于设备管理
window.createDeviceFromConnected = createDeviceFromConnected;
window.editDevice = editDevice;
window.deleteDevice = deleteDevice;
window.copyToClipboard = copyToClipboard;
window.toggleAdvancedSettings = toggleAdvancedSettings;
window.showWdaSetupGuide = showWdaSetupGuide;
window.hideWdaGuide = hideWdaGuide;
window.retryWdaConnection = retryWdaConnection;

// 已移除 pairWirelessDevice 函数，现在使用连接向导进行配对

// 无线设备连接功能
async function connectWirelessDevice(ipAddress, port = 5555) {
    if (!ipAddress) {
        window.NotificationModule.showNotification('请输入设备IP地址', 'warning');
        return;
    }
    
    const { ipcRenderer } = getGlobals();
    
    // 显示连接中状态
    window.NotificationModule.showNotification('正在连接无线设备...', 'info');
    
    try {
        const result = await ipcRenderer.invoke('adb-connect-wireless', ipAddress, port);
        
        if (result.success) {
            window.NotificationModule.showNotification(result.message || '连接成功', 'success');
            // 连接成功后刷新设备列表
            await refreshConnectedDevices();
        } else {
            // 如果连接失败，可能需要先配对
            if (result.error && result.error.includes('unauthorized')) {
                window.NotificationModule.showNotification('设备未授权，请先使用配对码进行配对', 'warning');
            } else {
                window.NotificationModule.showNotification(result.error || '连接失败', 'error');
            }
        }
        
        return result;
    } catch (error) {
        window.NotificationModule.showNotification(`连接失败: ${error.message}`, 'error');
        return { success: false, error: error.message };
    }
}

// 显示WDA设置指导
function showWdaSetupGuide(deviceId) {
    const guideContent = `
        <div class="wda-guide-modal">
            <h3>iOS WebDriverAgent 设置指导</h3>
            <div class="guide-content">
                <p><strong>设备ID:</strong> ${deviceId}</p>
                
                <h4>WDA服务未连接的可能原因：</h4>
                <ul>
                    <li>iOS设备上的WebDriverAgent应用未安装或未启动</li>
                    <li>WDA服务端口配置错误（默认8100）</li>
                    <li>网络连接问题或防火墙阻止</li>
                    <li>iOS设备和电脑不在同一网络</li>
                </ul>
                
                <h4>解决步骤：</h4>
                <ol>
                    <li><strong>确认WDA安装：</strong>确保iOS设备上已安装WebDriverAgent应用</li>
                    <li><strong>启动WDA服务：</strong>在iOS设备上启动WebDriverAgent应用</li>
                    <li><strong>检查端口：</strong>确认WDA服务运行在正确端口（通常是8100）</li>
                    <li><strong>网络连接：</strong>确保iOS设备和电脑在同一WiFi网络</li>
                    <li><strong>防火墙设置：</strong>检查电脑和iOS设备的防火墙设置</li>
                </ol>
                
                <div class="guide-note">
                    <strong>提示：</strong>可以在iOS设备的Safari浏览器中访问 http://[设备IP]:8100/status 来测试WDA服务是否正常运行。
                </div>
            </div>
            <div class="guide-actions">
                <button class="btn btn-primary" onclick="hideWdaGuide()">知道了</button>
                <button class="btn btn-secondary" onclick="retryWdaConnection('${deviceId}')">重试连接</button>
            </div>
        </div>
        <div class="modal-overlay" onclick="hideWdaGuide()"></div>
    `;
    
    document.body.insertAdjacentHTML('beforeend', guideContent);
}

// 隐藏WDA设置指导
function hideWdaGuide() {
    const guide = document.querySelector('.wda-guide-modal');
    const overlay = document.querySelector('.modal-overlay');
    if (guide) guide.remove();
    if (overlay) overlay.remove();
}

// 重试WDA连接
async function retryWdaConnection(deviceId) {
    hideWdaGuide();
    window.NotificationModule.showNotification('正在重试WDA连接...', 'info');
    await refreshConnectedDevices();
}

// 断开无线设备连接
async function disconnectWirelessDevice(ipAddress, port = 5555) {
    if (!ipAddress) {
        window.NotificationModule.showNotification('请提供设备IP地址', 'warning');
        return;
    }
    
    const { ipcRenderer } = getGlobals();
    
    try {
        const result = await ipcRenderer.invoke('adb-disconnect-wireless', ipAddress, port);
        
        if (result.success) {
            window.NotificationModule.showNotification(result.message, 'success');
            // 断开连接后刷新设备列表
            await refreshConnectedDevices();
        } else {
            window.NotificationModule.showNotification(result.error, 'error');
        }
        
        return result;
    } catch (error) {
        window.NotificationModule.showNotification(`断开连接失败: ${error.message}`, 'error');
        return { success: false, error: error.message };
    }
}



// ==================== 连接向导功能 ====================

// 显示连接向导
function showConnectionGuide() {
    const modal = document.getElementById('connectionGuideModal');
    const deviceForm = document.getElementById('deviceForm');
    
    // 隐藏添加设备表单
    if (deviceForm) {
        deviceForm.style.display = 'none';
    }
    
    // 显示连接向导模态框
    if (modal) {
        modal.style.display = 'flex';
        
        // 重置到USB连接指导（恢复原来的行为）
        showConnectionMethod('usb');
        
        // 重置配对状态
        resetPairingStatus();
    }
}


// 隐藏连接向导
function hideConnectionGuide() {
    const modal = document.getElementById('connectionGuideModal');
    if (modal) {
        modal.style.display = 'none';
    }
    
    // 重置配对状态
    resetPairingStatus();
}

// 初始化连接方式切换器
function initializeMethodSwitcher() {
    const methodBtns = document.querySelectorAll('.method-btn');
    const methodContents = document.querySelectorAll('.method-content');
    
    methodBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const method = btn.dataset.method;
            
            // 切换按钮状态
            methodBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // 切换内容显示
            methodContents.forEach(content => {
                if (content.id === method + 'Method') {
                    content.classList.add('active');
                } else {
                    content.classList.remove('active');
                }
            });
            
            // 重置配对状态
            resetPairingStatus();
        });
    });
}

// 初始化连接向导标签页切换
function initializeConnectionTabs() {
    const tabButtons = document.querySelectorAll('.tab-button');
    const tabContents = document.querySelectorAll('.tab-content');
    
    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabName = btn.dataset.tab;
            
            // 切换按钮状态
            tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // 切换内容显示
            tabContents.forEach(content => {
                content.classList.remove('active');
                if (content.id === tabName + 'Tab') {
                    content.classList.add('active');
                }
            });
            
            // 重置配对状态
            resetPairingStatus();
        });
    });
}

// 显示连接方法
function showConnectionMethod(method) {
    const tabButtons = document.querySelectorAll('.tab-button');
    const tabContents = document.querySelectorAll('.tab-content');
    
    // 切换按钮状态
    tabButtons.forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.tab === method) {
            btn.classList.add('active');
        }
    });
    
    // 切换内容显示
    tabContents.forEach(content => {
        content.classList.remove('active');
        if (content.id === method + 'Tab') {
            content.classList.add('active');
        }
    });
}

// 初始化IP地址同步
function initializeIpSync() {
    const deviceIpInput = document.getElementById('deviceIpInput');
    const deviceIpInput2 = document.getElementById('deviceIpInput2');
    
    if (deviceIpInput && deviceIpInput2) {
        deviceIpInput.addEventListener('input', (e) => {
            deviceIpInput2.value = e.target.value;
        });
        
        // 初始化时也同步一次
        deviceIpInput2.value = deviceIpInput.value;
    }
}

// 显示配对方式
function showPairingMethod(method) {
    const codeSection = document.getElementById('pairingCodeSection');
    const qrSection = document.getElementById('qrCodeSection');
    const showCodeBtn = document.getElementById('showPairingCodeBtn');
    const showQrBtn = document.getElementById('showQrCodeBtn');
    
    if (!codeSection || !qrSection) {
        window.rError('配对区域元素未找到');
        return;
    }
    
    if (method === 'code') {
        codeSection.style.display = 'block';
        qrSection.style.display = 'none';
        if (showCodeBtn) showCodeBtn.classList.add('active');
        if (showQrBtn) showQrBtn.classList.remove('active');
    } else {
        codeSection.style.display = 'none';
        qrSection.style.display = 'block';
        if (showCodeBtn) showCodeBtn.classList.remove('active');
        if (showQrBtn) showQrBtn.classList.add('active');
    }
}

// 使用配对码连接设备
async function connectWithPairingCode() {
    const { ipcRenderer } = getGlobals();
    
    try {
        // 获取用户输入的信息
        const deviceIp = document.getElementById('deviceIpInput').value.trim();
        const adbPort = document.getElementById('deviceAdbPortInput').value.trim();
        const pairingPort = document.getElementById('devicePairingPortInput').value.trim();
        const pairingCode = document.getElementById('devicePairingCodeInput').value.trim();
        
        // 验证输入
        if (!deviceIp || !adbPort || !pairingPort || !pairingCode) {
            window.NotificationModule.showNotification('Please fill in all fields', 'warning');
            return;
        }
        
        // 验证IP地址格式
        const ipRegex = /^(\d{1,3}\.){3}\d{1,3}$/;
        if (!ipRegex.test(deviceIp)) {
            window.NotificationModule.showNotification('Invalid IP address format', 'error');
            return;
        }
        
        // 验证端口号
        const adbPortNum = parseInt(adbPort);
        const pairingPortNum = parseInt(pairingPort);
        if (isNaN(adbPortNum) || isNaN(pairingPortNum) || adbPortNum < 1 || adbPortNum > 65535 || pairingPortNum < 1 || pairingPortNum > 65535) {
            window.NotificationModule.showNotification('Invalid port number (must be 1-65535)', 'error');
            return;
        }
        
        window.NotificationModule.showNotification('Connecting with pairing code...', 'info');
        
        // 使用配对码连接
        const pairResult = await ipcRenderer.invoke('adb-pair-wireless', deviceIp, pairingPortNum, pairingCode);
        
        if (!pairResult.success) {
            window.NotificationModule.showNotification(`Pairing failed: ${pairResult.error}`, 'error');
            return;
        }
        
        window.NotificationModule.showNotification('Device paired successfully!', 'success');
        
        // 连接到设备
        const connectResult = await ipcRenderer.invoke('adb-connect-wireless', deviceIp, adbPortNum);
        
        if (connectResult.success) {
            window.NotificationModule.showNotification('Device connected successfully!', 'success');
            
            // 刷新设备列表
            await refreshConnectedDevices();
            
            // 清空输入框
            document.getElementById('deviceIpInput').value = '';
            document.getElementById('deviceIpInput2').value = '';
            document.getElementById('deviceAdbPortInput').value = '';
            document.getElementById('devicePairingPortInput').value = '';
            document.getElementById('devicePairingCodeInput').value = '';
            
            // 关闭弹窗
            hideConnectionGuide();
        } else {
            window.NotificationModule.showNotification(`Connection failed: ${connectResult.error}`, 'error');
        }
        
    } catch (error) {
        window.rError('Pairing with code failed:', error);
        window.NotificationModule.showNotification(`Pairing failed: ${error.message}`, 'error');
    }
}

// 重置配对状态
function resetPairingStatus() {
    // 清空配对码输入框
    const pairingInputs = ['deviceIpInput', 'deviceIpInput2', 'deviceAdbPortInput', 'devicePairingPortInput', 'devicePairingCodeInput'];
    pairingInputs.forEach(id => {
        const element = document.getElementById(id);
        if (element) {
            element.value = '';
        }
    });
    
    // 重置QR码显示
    resetQRDisplay();
    
    // 隐藏配对区域
    const codeSection = document.getElementById('pairingCodeSection');
    const qrSection = document.getElementById('qrCodeSection');
    if (codeSection) codeSection.style.display = 'none';
    if (qrSection) qrSection.style.display = 'none';
    
    // 重置按钮状态
    const showCodeBtn = document.getElementById('showPairingCodeBtn');
    const showQrBtn = document.getElementById('showQrCodeBtn');
    if (showCodeBtn) showCodeBtn.classList.remove('active');
    if (showQrBtn) showQrBtn.classList.remove('active');
}

// ==================== QR码配对功能 ====================

let qrTimer = null;
let qrExpiryTime = null;

// 切换QR码区域显示
function toggleQrSection() {
    const toggleBtn = document.getElementById('toggleQrSectionBtn');
    const qrSection = document.getElementById('qrPairingSection');
    const icon = toggleBtn.querySelector('.btn-icon');
    
    if (qrSection.style.display === 'none') {
        qrSection.style.display = 'block';
        toggleBtn.textContent = '收起';
        toggleBtn.appendChild(icon);
        toggleBtn.classList.add('expanded');
    } else {
        qrSection.style.display = 'none';
        toggleBtn.textContent = '展开';
        toggleBtn.appendChild(icon);
        toggleBtn.classList.remove('expanded');
    }
}

// 生成QR码
async function generateQRCode() {
    const { ipcRenderer } = getGlobals();
    
    try {
        // 显示加载状态
        window.NotificationModule.showNotification('正在生成QR码...', 'info');
        
        // 生成配对数据
        const pairingDataResult = await ipcRenderer.invoke('generate-qr-pairing-data');
        
        if (!pairingDataResult.success) {
            window.NotificationModule.showNotification(`生成配对数据失败: ${pairingDataResult.error}`, 'error');
            return;
        }
        
        const { serviceName, pairingCode, qrData, expiryTime, localIP, pairingPort, adbPort } = pairingDataResult;
        
        // 生成QR码图片
        const qrResult = await ipcRenderer.invoke('generate-qr-code', qrData, { width: 200 });
        
        if (!qrResult.success) {
            window.NotificationModule.showNotification(`生成QR码失败: ${qrResult.error}`, 'error');
            return;
        }
        
        // 显示QR码
        displayQRCode(qrResult.dataURL, serviceName, pairingCode, expiryTime, localIP, pairingPort);
        
        // 启动配对服务
        const serviceResult = await ipcRenderer.invoke('start-adb-pairing-service', serviceName, pairingCode, localIP, pairingPort);
        
        if (serviceResult.success) {
            window.NotificationModule.showNotification('QR码生成成功，请在设备上扫描', 'success');
            
            // 开始检查配对状态
            startPairingStatusCheck();
        } else {
            window.NotificationModule.showNotification(`启动配对服务失败: ${serviceResult.error}`, 'warning');
        }
        
    } catch (error) {
        window.rError('生成QR码失败:', error);
        window.NotificationModule.showNotification(`生成QR码失败: ${error.message}`, 'error');
    }
}

// 显示QR码
function displayQRCode(dataURL, serviceName, pairingCode, expiryTime, localIP, pairingPort) {
    const qrDisplay = document.getElementById('qrDisplay');
    const qrCanvas = document.getElementById('qrCanvas');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');
    
    if (!qrDisplay || !qrCanvas || !qrInfo) {
        window.rError('QR code display elements not found');
        return;
    }
    
    const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');
    
    // 隐藏占位符，显示QR码
    if (qrPlaceholder) qrPlaceholder.style.display = 'none';
    qrCanvas.style.display = 'block';
    
    // 设置QR码图片
    const img = new Image();
    img.onload = function() {
        const ctx = qrCanvas.getContext('2d');
        qrCanvas.width = img.width;
        qrCanvas.height = img.height;
        ctx.drawImage(img, 0, 0);
    };
    img.src = dataURL;
    
    // 更新配对信息显示
    const qrInfoHTML = `
        <div class="info-item">
            <label>Pairing Code:</label>
            <span class="highlight-code">${pairingCode}</span>
        </div>
        <div class="info-item">
            <label>Pairing Port:</label>
            <span class="highlight-code">${pairingPort}</span>
        </div>
        <div class="info-item">
            <label>Local IP:</label>
            <span>${localIP}</span>
        </div>
        <div class="info-item">
            <label>Service Name:</label>
            <span class="highlight-code">${serviceName}</span>
        </div>
    `;
    qrInfo.innerHTML = qrInfoHTML;
    qrInfo.style.display = 'block';
    
    // 切换按钮状态
    if (generateBtn) generateBtn.style.display = 'none';
    if (refreshBtn) refreshBtn.style.display = 'block';
    
    // 启动倒计时
    qrExpiryTime = expiryTime;
    startQRTimer();
}

// 启动QR码倒计时
function startQRTimer() {
    if (qrTimer) {
        clearInterval(qrTimer);
    }
    
    qrTimer = setInterval(() => {
        const now = Date.now();
        const timeLeft = qrExpiryTime - now;
        
        if (timeLeft <= 0) {
            // 过期
            clearInterval(qrTimer);
            resetQRDisplay();
            window.NotificationModule.showNotification('QR码已过期，请重新生成', 'warning');
            return;
        }
        
        // 更新倒计时显示
        const minutes = Math.floor(timeLeft / 60000);
        const seconds = Math.floor((timeLeft % 60000) / 1000);
        const timerValue = document.getElementById('timerValue');
        if (timerValue) {
            timerValue.textContent = `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
        }
    }, 1000);
}

// 重置QR码显示
function resetQRDisplay() {
    const qrDisplay = document.getElementById('qrDisplay');
    const qrCanvas = document.getElementById('qrCanvas');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');
    
    if (qrDisplay) {
        const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');
        if (qrPlaceholder) {
            qrPlaceholder.style.display = 'block';
        }
    }
    
    // 显示占位符，隐藏QR码
    if (qrCanvas) qrCanvas.style.display = 'none';
    if (qrInfo) qrInfo.style.display = 'none';
    
    // 重置按钮状态
    if (generateBtn) generateBtn.style.display = 'block';
    if (refreshBtn) refreshBtn.style.display = 'none';
    
    // 清理倒计时
    if (qrTimer) {
        clearInterval(qrTimer);
        qrTimer = null;
    }
}

// 检查配对状态
async function startPairingStatusCheck() {
    const { ipcRenderer } = getGlobals();
    let checkCount = 0;
    const maxChecks = 60; // 最多检查5分钟
    
    const checkInterval = setInterval(async () => {
        checkCount++;
        
        try {
            const statusResult = await ipcRenderer.invoke('check-pairing-status');
            
            if (statusResult.success && statusResult.hasNewDevices) {
                // 发现新设备，配对可能成功
                clearInterval(checkInterval);
                window.NotificationModule.showNotification('检测到新设备连接，配对可能成功！', 'success');
                
                // 刷新设备列表
                await refreshConnectedDevices();
                
                // 重置QR码显示
                resetQRDisplay();
                return;
            }
            
            if (checkCount >= maxChecks) {
                // 检查超时
                clearInterval(checkInterval);
                window.rLog('配对状态检查超时');
            }
        } catch (error) {
            window.rError('检查配对状态失败:', error);
        }
    }, 5000); // 每5秒检查一次
}

// 设置设备卡片的拖拽功能
function setupDragAndDropForDevice(deviceCard, deviceId) {
    // 防止默认拖拽行为
    deviceCard.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.add('drag-over');
    });
    
    deviceCard.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');
    });
    
    deviceCard.addEventListener('drop', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');
        
        const files = e.dataTransfer.files;
        
        // 检查是否有文件
        if (files.length === 0) {
            return;
        }
        
        // 检查是否为APK文件
        const file = files[0];
        if (!file.name.toLowerCase().endsWith('.apk')) {
            window.NotificationModule.showNotification('请拖入APK文件', 'warning');
            return;
        }
        
        // 获取文件路径 - 使用Electron的webUtils.getPathForFile()方法
        let filePath = null;
        
        try {
            const { webUtils } = require('electron');
            filePath = webUtils.getPathForFile(file);
        } catch (error) {
            window.rError('无法获取文件路径:', error.message);
            window.NotificationModule.showNotification('无法获取文件路径，请确保应用有文件访问权限', 'error');
            return;
        }
        
        if (!filePath) {
            window.rError('文件路径为空');
            window.NotificationModule.showNotification('无法获取文件路径', 'error');
            return;
        }
        
        // 安装APK
        await installApkToDevice(deviceId, filePath);
    });
}

// 安装APK到设备
async function installApkToDevice(deviceId, apkPath) {
    const { ipcRenderer } = getGlobals();

    if (!deviceId || !apkPath) {
        window.NotificationModule.showNotification('设备ID或APK路径无效', 'error');
        return;
    }

    try {
        // 第一步：先用 AAPT 获取包名和主Activity
        showApkInstallLoading('正在解析APK信息...', '');

        const packageInfo = await ipcRenderer.invoke('get-apk-package-name', apkPath);

        if (!packageInfo.success || !packageInfo.packageName) {
            hideApkInstallLoading();
            window.NotificationModule.showNotification('无法解析APK文件', 'error');
            return;
        }

        const packageName = packageInfo.packageName;
        window.rLog('通过 AAPT 获取到包名:', packageName);

        // 第二步：开始安装
        updateApkInstallLoading('正在安装APK...', '', packageName);

        const result = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, false);
        window.rLog('收到安装结果:', result);

        if (result.success) {
            // 安装成功，使用已获取的包名启动应用
            hideApkInstallLoading();
            await autoLaunchAppAfterInstall(deviceId, packageName);
            window.NotificationModule.showNotification('APK安装成功！', 'success');
            await refreshConnectedDevices();
            return;
        }

        // 第三步：安装失败，显示确认对话框询问是否卸载重装
        window.rLog('安装失败，显示卸载重装确认对话框');
        hideApkInstallLoading();
        showApkInstallModalWithPackage(deviceId, apkPath, packageName);

    } catch (error) {
        window.rError('安装APK失败:', error);
        hideApkInstallLoading();
        window.NotificationModule.showNotification(`安装失败: ${error.message}`, 'error');
    }
}

// APK安装Loading状态管理
function showApkInstallLoading(title, message) {
    const modal = document.getElementById('apkInstallLoadingModal');
    const titleElement = document.getElementById('apkLoadingTitle');
    const messageElement = document.getElementById('apkLoadingMessage');
    const detailsElement = document.getElementById('apkLoadingDetails');
    
    if (modal && titleElement && messageElement) {
        titleElement.textContent = title;
        messageElement.textContent = message;
        detailsElement.style.display = 'none'; // 初始时隐藏详情
        modal.style.display = 'block';
    }
}

function updateApkInstallLoading(title, message, packageName = null) {
    const titleElement = document.getElementById('apkLoadingTitle');
    const messageElement = document.getElementById('apkLoadingMessage');
    const detailsElement = document.getElementById('apkLoadingDetails');
    const packageNameElement = document.getElementById('apkPackageName');
    
    if (titleElement && messageElement) {
        titleElement.textContent = title;
        messageElement.textContent = message;
        
        if (packageName && detailsElement && packageNameElement) {
            packageNameElement.textContent = packageName;
            detailsElement.style.display = 'block';
        }
    }
}

function hideApkInstallLoading() {
    const modal = document.getElementById('apkInstallLoadingModal');
    if (modal) {
        modal.style.display = 'none';
    }
}

// 显示APK安装确认模态框（带已知包名）
function showApkInstallModalWithPackage(deviceId, apkPath, packageName) {
    const modal = document.getElementById('apkInstallModal');
    if (!modal) {
        window.rError('找不到APK安装模态框');
        return;
    }
    
    // 更新模态框内容，显示具体的包名信息
    const messageElement = modal.querySelector('.modal-message');
    if (messageElement) {
        messageElement.innerHTML = `
            <p><strong>签名不匹配</strong></p>
            <p>应用：<code>${packageName}</code></p>
            <p>需要先卸载现有版本才能安装。</p>
        `;
    }
    
    // 设置确认按钮的事件处理器
    const confirmBtn = modal.querySelector('#confirmUninstallBtn');
    const cancelBtn = modal.querySelector('#cancelUninstallBtn');
    
    if (confirmBtn) {
        // 移除之前的事件监听器
        confirmBtn.replaceWith(confirmBtn.cloneNode(true));
        const newConfirmBtn = modal.querySelector('#confirmUninstallBtn');
        
        newConfirmBtn.addEventListener('click', async () => {
            modal.style.display = 'none';
            // 使用已知的包名直接进行卸载重装
            await uninstallAndReinstallWithKnownPackage(deviceId, apkPath, packageName);
        });
    }
    
    if (cancelBtn) {
        cancelBtn.replaceWith(cancelBtn.cloneNode(true));
        const newCancelBtn = modal.querySelector('#cancelUninstallBtn');
        
        newCancelBtn.addEventListener('click', () => {
            modal.style.display = 'none';
            // 确保loading modal也被隐藏
            hideApkInstallLoading();
        });
    }
    
    modal.style.display = 'block';
}

// 显示APK安装确认模态框（原版，保留兼容性）
function showApkInstallModal(deviceId, apkPath) {
    const modal = document.getElementById('apkInstallModal');
    
    if (modal) {
        modal.style.display = 'flex';
        
        // 保存当前的设备ID和APK路径
        window.pendingApkInstall = {
            deviceId: deviceId,
            apkPath: apkPath
        };
    } else {
        // 如果模态框不存在，使用confirm作为备用
        if (confirm('APK签名冲突，是否先卸载原应用再安装？\n\n注意：卸载会清除应用数据！')) {
            uninstallAndReinstallApk(deviceId, apkPath);
        }
    }
}

// 隐藏APK安装确认模态框
function hideApkInstallModal() {
    const modal = document.getElementById('apkInstallModal');
    if (modal) {
        modal.style.display = 'none';
    }
    window.pendingApkInstall = null;
}

// 取消APK安装
window.cancelApkInstall = function() {
    hideApkInstallModal();
    window.NotificationModule.showNotification('已取消安装', 'info');
};

// 确认卸载并安装
window.confirmApkUninstall = async function() {
    if (!window.pendingApkInstall) {
        hideApkInstallModal();
        return;
    }
    
    const { deviceId, apkPath } = window.pendingApkInstall;
    hideApkInstallModal();
    
    // 执行卸载并重新安装
    await uninstallAndReinstallApk(deviceId, apkPath);
};

// 卸载并重新安装APK
async function uninstallAndReinstallApk(deviceId, apkPath) {
    const { ipcRenderer } = getGlobals();
    
    try {
        // 先尝试获取APK的包名
        window.NotificationModule.showNotification('正在获取APK信息...', 'info');
        
        // 尝试通过aapt获取包名
        let packageName = null;
        const packageInfo = await ipcRenderer.invoke('get-apk-package-name', apkPath);
        
        if (packageInfo.success && packageInfo.packageName) {
            packageName = packageInfo.packageName;
        } else {
            // 如果无法自动获取包名，尝试通过安装错误信息获取
            window.NotificationModule.showNotification('无法自动获取包名，尝试直接安装...', 'warning');
            
            // 直接尝试强制安装，不卸载
            const directInstallResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);
            if (directInstallResult.success) {
                window.NotificationModule.showNotification('APK安装成功！', 'success');
                await refreshConnectedDevices();
                return; // 安装成功，直接返回
            } else if (directInstallResult.packageName) {
                // 从安装错误中获取到了包名，使用该包名继续卸载重装流程
                packageName = directInstallResult.packageName;
                window.NotificationModule.showNotification(`从错误信息中获取到包名: ${packageName}，继续卸载重装...`, 'info');
                // 不return，继续执行后面的卸载重装逻辑
            } else {
                window.NotificationModule.showNotification('无法获取包名，请手动卸载原应用后重试', 'error');
                return;
            }
        }
        
        // 卸载应用（卸载的是与APK相同包名的已安装版本）
        window.NotificationModule.showNotification(`正在卸载已安装的 ${packageName}...`, 'info');
        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);
        
        if (!uninstallResult.success) {
            // 如果卸载失败，可能是应用不存在，直接尝试安装
            window.NotificationModule.showNotification('原应用可能不存在，尝试直接安装...', 'info');
        }
        
        // 安装新的APK
        window.NotificationModule.showNotification('正在安装APK...', 'info');
        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);
        
        if (installResult.success) {
            // 尝试自动启动应用
            if (packageName) {
                await autoLaunchAppAfterInstall(deviceId, packageName);
            }
            
            window.NotificationModule.showNotification('APK安装成功！', 'success');
            await refreshConnectedDevices();
        } else {
            window.NotificationModule.showNotification(`安装失败: ${installResult.error}`, 'error');
        }
        
    } catch (error) {
        window.rError('卸载并重装失败:', error);
        window.NotificationModule.showNotification(`操作失败: ${error.message}`, 'error');
    }
}

// 使用已知包名进行卸载重装（简化版本，无需提取包名）
async function uninstallAndReinstallWithKnownPackage(deviceId, apkPath, packageName) {
    const { ipcRenderer } = getGlobals();
    
    try {
        window.rLog(`开始卸载重装流程，包名: ${packageName}`);
        
        // 显示卸载loading
        showApkInstallLoading('正在卸载...', '');
        // 然后更新显示包名
        updateApkInstallLoading('正在卸载...', '', packageName);
        
        // 第一步：卸载已安装的应用
        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);
        
        if (!uninstallResult.success) {
            // 如果卸载失败，可能是应用不存在，继续尝试安装
            updateApkInstallLoading('准备安装...', '', packageName);
        } else {
            updateApkInstallLoading('正在安装...', '', packageName);
        }
        
        // 第二步：安装新的APK
        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);
        
        // 隐藏loading
        hideApkInstallLoading();
        
        if (installResult.success) {
            // 尝试自动启动应用
            if (packageName) {
                await autoLaunchAppAfterInstall(deviceId, packageName);
            }
            
            window.NotificationModule.showNotification('APK安装成功！', 'success');
            await refreshConnectedDevices();
        } else {
            window.NotificationModule.showNotification(`安装失败: ${installResult.error}`, 'error');
        }
        
    } catch (error) {
        window.rError('卸载并重装失败:', error);
        hideApkInstallLoading();
        window.NotificationModule.showNotification(`操作失败: ${error.message}`, 'error');
    }
}

// 全局函数
window.connectWirelessDevice = connectWirelessDevice;
window.disconnectWirelessDevice = disconnectWirelessDevice;
window.showConnectionGuide = showConnectionGuide;
window.hideConnectionGuide = hideConnectionGuide;
window.showConnectionMethod = showConnectionMethod;
window.initializeConnectionTabs = initializeConnectionTabs;
window.initializeIpSync = initializeIpSync;
window.showPairingMethod = showPairingMethod;
window.connectWithPairingCode = connectWithPairingCode;
window.generateQRCode = generateQRCode;

// 自动启动刚安装的应用
async function autoLaunchAppAfterInstall(deviceId, packageName) {
    if (!deviceId || !packageName) {
        window.rLog('缺少设备ID或包名，跳过自动启动');
        return;
    }
    
    try {
        window.NotificationModule.showNotification('正在启动应用...', 'info');
        
        // 首先尝试获取应用的主Activity
        const mainActivity = await getMainActivity(deviceId, packageName);
        
        if (mainActivity) {
            // 使用TKE ADB启动应用
            await launchAppWithTKE(deviceId, packageName, mainActivity);
        } else {
            // 如果无法获取主Activity，使用monkey方式启动
            await launchAppWithMonkey(deviceId, packageName);
        }
        
    } catch (error) {
        window.rError('自动启动应用失败:', error);
        // 静默失败，不显示错误通知，因为安装已经成功了
    }
}

// 执行TKE ADB命令的辅助函数
async function executeTkeAdbCommand(deviceId, adbArgs) {
    const { ipcRenderer } = getGlobals();
    
    try {
        const result = await ipcRenderer.invoke('execute-tke-adb-command', deviceId, adbArgs);
        return result;
    } catch (error) {
        window.rError('执行TKE ADB命令失败:', error);
        return { success: false, error: error.message };
    }
}

// 获取应用的主Activity
async function getMainActivity(deviceId, packageName) {
    try {
        // 使用TKE ADB直通命令获取包信息
        const dumpsysResult = await executeTkeAdbCommand(deviceId, ['shell', 'dumpsys', 'package', packageName]);
        
        if (dumpsysResult.success && dumpsysResult.output) {
            // 从输出中解析主Activity
            const lines = dumpsysResult.output.split('\n');
            
            for (const line of lines) {
                // 查找intent-filter中的MAIN和LAUNCHER
                if (line.includes('android.intent.action.MAIN') || line.includes('MAIN')) {
                    // 往上查找对应的Activity
                    const activityMatch = dumpsysResult.output.match(new RegExp(`${packageName}/([^\\s]+).*?android\\.intent\\.action\\.MAIN`, 's'));
                    if (activityMatch) {
                        const activityName = activityMatch[1];
                        window.rLog('找到主Activity:', activityName);
                        return activityName.startsWith('.') ? packageName + activityName : activityName;
                    }
                }
            }
            
            // 尝试另一种解析方式
            const activityPattern = new RegExp(`Activity #\\d+.*?${packageName}/([^\\s}]+)`, 'g');
            const activities = [...dumpsysResult.output.matchAll(activityPattern)];
            
            if (activities.length > 0) {
                const activityName = activities[0][1];
                window.rLog('找到Activity:', activityName);
                return activityName.startsWith('.') ? packageName + activityName : activityName;
            }
        }
        
        // 尝试使用pm dump获取
        const pmResult = await executeTkeAdbCommand(deviceId, ['shell', 'pm', 'dump', packageName]);
        
        if (pmResult.success && pmResult.output) {
            // 查找主Activity
            const mainActivityMatch = pmResult.output.match(/android\.intent\.action\.MAIN.*?\n.*?([A-Za-z0-9_.]+)/);
            if (mainActivityMatch) {
                const activityName = mainActivityMatch[1];
                window.rLog('通过pm dump找到主Activity:', activityName);
                return activityName.startsWith('.') ? packageName + activityName : activityName;
            }
        }
        
        window.rLog('未找到主Activity');
        return null;
        
    } catch (error) {
        window.rError('获取主Activity失败:', error);
        return null;
    }
}

// 使用TKE ADB启动应用
async function launchAppWithTKE(deviceId, packageName, activityName) {
    const { ipcRenderer } = getGlobals();
    
    try {
        const componentName = `${packageName}/${activityName}`;
        window.rLog('使用TKE ADB启动应用:', componentName);
        
        const result = await executeTkeAdbCommand(deviceId, [
            'shell', 'am', 'start', '-n', componentName
        ]);
        
        if (result.success) {
            window.rLog('应用启动成功');
            window.NotificationModule.showNotification(`应用 ${packageName} 已启动`, 'success');
        } else {
            window.rError('TKE ADB启动失败:', result.error);
            // 尝试备用方案
            await launchAppWithMonkey(deviceId, packageName);
        }
        
    } catch (error) {
        window.rError('TKE ADB启动失败:', error);
        // 尝试备用方案
        await launchAppWithMonkey(deviceId, packageName);
    }
}

// 使用monkey方式启动应用（备用方案）
async function launchAppWithMonkey(deviceId, packageName) {
    const { ipcRenderer } = getGlobals();
    
    try {
        window.rLog('使用monkey方式启动应用:', packageName);
        
        const result = await executeTkeAdbCommand(deviceId, [
            'shell', 'monkey', '-p', packageName, '-c', 'android.intent.category.LAUNCHER', '1'
        ]);
        
        if (result.success) {
            window.rLog('应用启动成功（monkey方式）');
            window.NotificationModule.showNotification(`应用 ${packageName} 已启动`, 'success');
        } else {
            window.rError('Monkey启动失败:', result.error);
        }
        
    } catch (error) {
        window.rError('Monkey启动失败:', error);
    }
}

// 导出函数
window.DeviceManagerModule = {
    initializeDevicePage,
    loadSavedDevices,
    refreshConnectedDevices,
    refreshDeviceList,
    createDeviceFromConnected,
    editDevice,
    deleteDevice,
    copyToClipboard,
    connectWirelessDevice,
    disconnectWirelessDevice,
    showConnectionGuide,
    hideConnectionGuide,
    showConnectionMethod,
    initializeConnectionTabs,
    initializeIpSync,
    showPairingMethod,
    connectWithPairingCode,
    generateQRCode
};
