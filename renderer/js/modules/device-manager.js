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
    
    if (addDeviceBtn) {
        addDeviceBtn.addEventListener('click', () => {
            deviceForm.style.display = deviceForm.style.display === 'none' ? 'block' : 'none';
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
}

// 加载保存的设备
async function loadSavedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    const deviceGrid = document.getElementById('deviceGrid');
    deviceGrid.innerHTML = '';
    
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
                
                const card = document.createElement('div');
                card.className = 'device-card';
                card.innerHTML = `
                    <div class="device-status ${isConnected ? 'connected' : ''}" title="${isConnected ? 'Connected' : 'Not Connected'}"></div>
                    <div class="device-name">${config.deviceName}</div>
                    <div class="device-info">
                        <span title="${config.platformName} ${config.platformVersion}">Platform: ${config.platformName} ${config.platformVersion}</span>
                        <span title="${config.appPackage}">Package: ${config.appPackage}</span>
                        <span title="${config.automationName}">Automation: ${config.automationName}</span>
                        ${config.deviceId ? `<span title="${config.deviceId}">ID: ${config.deviceId}</span>` : ''}
                    </div>
                    <div class="device-actions">
                        <button class="btn btn-secondary btn-small" onclick="editDevice('${file}')">
                            <svg viewBox="0 0 24 24" width="14" height="14" style="vertical-align: middle; margin-right: 4px;">
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
                
                deviceGrid.appendChild(card);
            }
        }
    } catch (error) {
        console.error('Failed to load saved devices:', error);
    }
}

// 刷新连接的设备
async function refreshConnectedDevices() {
    const { ipcRenderer, path, fs, yaml } = getGlobals();
    const result = await ipcRenderer.invoke('adb-devices');
    const connectedList = document.getElementById('connectedList');
    
    connectedList.innerHTML = '';
    
    if (result.success && result.devices.length > 0) {
        // 获取保存的设备以检查哪些已经保存
        let savedDeviceIds = [];
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
                        }
                    }
                }
            } catch (error) {
                console.error('Error loading saved devices:', error);
            }
        }
        
        result.devices.forEach(device => {
            const isSaved = savedDeviceIds.includes(device.id);
            const item = document.createElement('div');
            item.className = 'connected-item';
            item.innerHTML = `
                <span class="device-id">${device.id}</span>
                <span class="device-status-text">${device.status}</span>
                ${!isSaved && device.status === 'device' ? `
                    <button class="btn btn-primary" onclick="createDeviceFromConnected('${device.id}')">
                        <svg viewBox="0 0 24 24" width="14" height="14" style="vertical-align: middle; margin-right: 4px;">
                            <path fill="currentColor" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                        </svg>
                        Save
                    </button>
                ` : isSaved ? '<span class="text-muted" style="font-size: 12px;">Already saved</span>' : ''}
            `;
            connectedList.appendChild(item);
        });
    } else {
        connectedList.innerHTML = '<div class="text-muted">No devices connected</div>';
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
        newDeviceForm.querySelector('input[name="deviceId"]').value = deviceId;
        newDeviceForm.querySelector('input[name="deviceName"]').value = `Device ${deviceId.substring(0, 8)}`;
        newDeviceForm.querySelector('input[name="platformName"]').value = 'Android';
        newDeviceForm.querySelector('input[name="automationName"]').value = 'UiAutomator2';
        newDeviceForm.querySelector('select[name="noReset"]').value = 'true';
        newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = '6000';
    }
    
    // 更新表单标题
    deviceForm.querySelector('h3').textContent = 'Add New Device';
    deviceForm.style.display = 'block';
    window.NotificationModule.showNotification('Please complete the device configuration', 'info');
}

// 编辑设备配置
async function editDevice(filename) {
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
            newDeviceForm.querySelector('input[name="deviceId"]').value = config.deviceId || '';
            newDeviceForm.querySelector('input[name="platformName"]').value = config.platformName || 'Android';
            newDeviceForm.querySelector('input[name="platformVersion"]').value = config.platformVersion || '';
            newDeviceForm.querySelector('input[name="appPackage"]').value = config.appPackage || '';
            newDeviceForm.querySelector('input[name="appActivity"]').value = config.appActivity || '';
            newDeviceForm.querySelector('input[name="automationName"]').value = config.automationName || 'UiAutomator2';
            newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = config.newCommandTimeout || '6000';
            newDeviceForm.querySelector('select[name="noReset"]').value = config.noReset ? 'true' : 'false';
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

// 全局函数用于设备管理
window.createDeviceFromConnected = createDeviceFromConnected;
window.editDevice = editDevice;
window.deleteDevice = deleteDevice;

// 导出函数
window.DeviceManagerModule = {
    initializeDevicePage,
    loadSavedDevices,
    refreshConnectedDevices,
    refreshDeviceList,
    createDeviceFromConnected,
    editDevice,
    deleteDevice
};