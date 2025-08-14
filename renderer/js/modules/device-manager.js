// 设备管理模块

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 初始化设备页面
function initializeDevicePage() {
    const { fs, yaml } = getGlobals();
    const addDeviceBtn = document.getElementById('addDeviceBtn');
    const scanDevicesBtn = document.getElementById('scanDevicesBtn');
    const deviceForm = document.getElementById('deviceForm');
    const newDeviceForm = document.getElementById('newDeviceForm');
    const cancelDeviceBtn = document.getElementById('cancelDeviceBtn');
    const connectionTypeSelect = document.getElementById('connectionTypeSelect');
    
    // 连接向导相关元素
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
            deviceForm.style.display = deviceForm.style.display === 'none' ? 'block' : 'none';
            // 重置表单并设置默认值
            if (deviceForm.style.display === 'block') {
                newDeviceForm.reset();
                delete deviceForm.dataset.mode;
                delete deviceForm.dataset.filename;
                deviceForm.querySelector('h3').textContent = 'Add New Device';
                updateConnectionTypeFields();
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
    
    // 连接类型切换事件
    if (connectionTypeSelect) {
        connectionTypeSelect.addEventListener('change', updateConnectionTypeFields);
    }
    
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
            
            if (!window.AppGlobals.currentProject) {
                window.NotificationModule.showNotification('Please open a project first', 'warning');
                return;
            }
            
            const formData = new FormData(newDeviceForm);
            const deviceConfig = {};
            
            for (const [key, value] of formData.entries()) {
                deviceConfig[key] = value === 'true' ? true : value === 'false' ? false : value;
            }
            
            // 如果是WiFi连接，将IP和端口组合为deviceId
            if (deviceConfig.connectionType === 'wifi' && deviceConfig.ipAddress) {
                deviceConfig.deviceId = `${deviceConfig.ipAddress}:${deviceConfig.port || 5555}`;
            }
            
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
    
    // 监听配对成功事件
    const { ipcRenderer } = getGlobals();
    ipcRenderer.on('pairing-success', (event, data) => {
        console.log('收到配对成功事件:', data);
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

// 更新连接类型字段显示
function updateConnectionTypeFields() {
    const connectionType = document.getElementById('connectionTypeSelect').value;
    const deviceIdGroup = document.getElementById('deviceIdGroup');
    const wifiAddressRow = document.getElementById('wifiAddressRow');
    const connectionHelpRow = document.getElementById('connectionHelpRow');
    
    if (connectionType === 'wifi') {
        // WiFi 连接
        if (deviceIdGroup) deviceIdGroup.parentElement.style.display = 'none';
        if (wifiAddressRow) wifiAddressRow.style.display = 'flex';
        if (connectionHelpRow) connectionHelpRow.style.display = 'flex';
    } else {
        // USB 连接
        if (deviceIdGroup) deviceIdGroup.parentElement.style.display = 'flex';
        if (wifiAddressRow) wifiAddressRow.style.display = 'none';
        if (connectionHelpRow) connectionHelpRow.style.display = 'none';
    }
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
                
                savedDevicesGrid.appendChild(card);
            }
        }
    } catch (error) {
        console.error('Failed to load saved devices:', error);
    }
}

// 刷新连接的设备（统一显示有线和无线设备）
async function refreshConnectedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();
    const result = await ipcRenderer.invoke('adb-devices');
    const connectedDevicesGrid = document.getElementById('connectedDevicesGrid');
    
    if (!connectedDevicesGrid) return;
    
    connectedDevicesGrid.innerHTML = '';
    
    if (result.success && result.devices.length > 0) {
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
                console.error('Error loading saved devices:', error);
            }
        }
        
        // 为每个连接的设备创建卡片
        for (const device of result.devices) {
            if (device.status !== 'device') continue;
            
            const isSaved = savedDeviceIds.includes(device.id);
            const savedConfig = savedDeviceConfigs[device.id];
            // 判断是否为无线设备：包含冒号或者是mDNS格式（包含._adb-tls-connect._tcp）
            const isWifi = device.id.includes(':') || device.id.includes('._adb-tls-connect._tcp');
            
            const item = document.createElement('div');
            item.className = 'device-card';
            
            // 创建卡片内容
            const deviceName = savedConfig ? savedConfig.deviceName : '未知设备';
            
            let cardContent = `
                <div class="device-card-header">
                    <div class="device-status connected" title="Connected"></div>
                    <div class="device-name">${deviceName}</div>
                    <span class="connection-type-badge ${isWifi ? 'wifi' : 'usb'}">${isWifi ? 'WiFi' : 'USB'}</span>
                </div>
                <div class="device-info">
                    <div class="device-id">ID: ${device.id}</div>
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
                console.error('Error loading device configs:', error);
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
    deviceForm.dataset.mode = 'create';
    
    // 预填充表单
    if (newDeviceForm) {
        // 判断是否为WiFi设备
        const isWifi = deviceId.includes(':');
        
        if (isWifi) {
            const [ip, port] = deviceId.split(':');
            newDeviceForm.querySelector('select[name="connectionType"]').value = 'wifi';
            newDeviceForm.querySelector('input[name="ipAddress"]').value = ip;
            newDeviceForm.querySelector('input[name="port"]').value = port || '5555';
            newDeviceForm.querySelector('input[name="deviceName"]').value = `WiFi Device (${ip})`;
        } else {
            newDeviceForm.querySelector('select[name="connectionType"]').value = 'usb';
            newDeviceForm.querySelector('input[name="deviceId"]').value = deviceId;
            newDeviceForm.querySelector('input[name="deviceName"]').value = `Device ${deviceId.substring(0, 8)}`;
        }
        
        newDeviceForm.querySelector('input[name="platformName"]').value = 'Android';
        newDeviceForm.querySelector('input[name="automationName"]').value = 'UiAutomator2';
        newDeviceForm.querySelector('select[name="noReset"]').value = 'true';
        newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = '6000';
        
        // 更新表单字段显示
        updateConnectionTypeFields();
    }
    
    // 更新表单标题
    deviceForm.querySelector('h3').textContent = 'Add New Device';
    deviceForm.style.display = 'block';
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
            
            // 设置连接类型
            const connectionType = config.connectionType || (config.ipAddress ? 'wifi' : 'usb');
            newDeviceForm.querySelector('select[name="connectionType"]').value = connectionType;
            
            // 根据连接类型设置相应字段
            if (connectionType === 'wifi') {
                newDeviceForm.querySelector('input[name="ipAddress"]').value = config.ipAddress || '';
                newDeviceForm.querySelector('input[name="port"]').value = config.port || '5555';
            } else {
                newDeviceForm.querySelector('input[name="deviceId"]').value = config.deviceId || '';
            }
            
            newDeviceForm.querySelector('input[name="platformName"]').value = config.platformName || 'Android';
            newDeviceForm.querySelector('input[name="platformVersion"]').value = config.platformVersion || '';
            newDeviceForm.querySelector('input[name="automationName"]').value = config.automationName || 'UiAutomator2';
            newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = config.newCommandTimeout || '6000';
            newDeviceForm.querySelector('select[name="noReset"]').value = config.noReset ? 'true' : 'false';
            
            // 更新表单字段显示
            updateConnectionTypeFields();
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
        
        // 重置到USB连接指导
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
    
    if (method === 'code') {
        codeSection.style.display = 'block';
        qrSection.style.display = 'none';
        showCodeBtn.classList.add('active');
        showQrBtn.classList.remove('active');
    } else {
        codeSection.style.display = 'none';
        qrSection.style.display = 'block';
        showCodeBtn.classList.remove('active');
        showQrBtn.classList.add('active');
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
        const pairResult = await ipcRenderer.invoke('pair-wireless-device', deviceIp, pairingPortNum, pairingCode);
        
        if (!pairResult.success) {
            window.NotificationModule.showNotification(`Pairing failed: ${pairResult.error}`, 'error');
            return;
        }
        
        window.NotificationModule.showNotification('Device paired successfully!', 'success');
        
        // 连接到设备
        const connectResult = await ipcRenderer.invoke('connect-wireless-device', deviceIp, adbPortNum);
        
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
        console.error('Pairing with code failed:', error);
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
        console.error('生成QR码失败:', error);
        window.NotificationModule.showNotification(`生成QR码失败: ${error.message}`, 'error');
    }
}

// 显示QR码
function displayQRCode(dataURL, serviceName, pairingCode, expiryTime, localIP, pairingPort) {
    const qrDisplay = document.getElementById('qrDisplay');
    const qrCanvas = document.getElementById('qrCanvas');
    const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');
    const timerEl = document.getElementById('qrTimer');
    
    // 隐藏占位符，显示QR码
    qrPlaceholder.style.display = 'none';
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
            <label>配对码:</label>
            <span class="highlight-code">${pairingCode}</span>
        </div>
        <div class="info-item">
            <label>配对端口:</label>
            <span class="highlight-code">${pairingPort}</span>
        </div>
        <div class="info-item">
            <label>本机IP:</label>
            <span>${localIP}</span>
        </div>
        <div class="info-item">
            <label>服务名:</label>
            <span>${serviceName}</span>
        </div>
        <div class="manual-pairing-tip">
            <strong>手动配对：</strong>在手机"无线调试"→"使用配对码配对设备"中输入配对码 <strong>${pairingCode}</strong> 和端口 <strong>${pairingPort}</strong>
        </div>
    `;
    qrInfo.innerHTML = qrInfoHTML;
    qrInfo.style.display = 'block';
    
    // 切换按钮状态
    generateBtn.style.display = 'none';
    refreshBtn.style.display = 'block';
    timerEl.style.display = 'block';
    
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
    const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');
    const timerEl = document.getElementById('qrTimer');
    
    // 显示占位符，隐藏QR码
    qrPlaceholder.style.display = 'block';
    qrCanvas.style.display = 'none';
    qrInfo.style.display = 'none';
    
    // 重置按钮状态
    generateBtn.style.display = 'block';
    refreshBtn.style.display = 'none';
    timerEl.style.display = 'none';
    
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
                console.log('配对状态检查超时');
            }
        } catch (error) {
            console.error('检查配对状态失败:', error);
        }
    }, 5000); // 每5秒检查一次
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