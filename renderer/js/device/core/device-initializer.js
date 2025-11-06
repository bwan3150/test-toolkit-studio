// 设备页面初始化模块
// 负责初始化设备管理页面的所有功能和事件监听
// 依赖：
// - ../guide/connection-guide-ui.js - 连接向导UI
// - ../wifi/connection-pairing.js - 无线配对功能
// - ../wifi/connection-qr.js - QR码配对功能

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
                if (window.DeviceFormModule && window.DeviceFormModule.updatePlatformFields) {
                    window.DeviceFormModule.updatePlatformFields();
                }
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
        scanDevicesBtn.addEventListener('click', () => {
            if (window.DeviceListModule && window.DeviceListModule.refreshConnectedDevices) {
                window.DeviceListModule.refreshConnectedDevices();
            }
        });
    }


    // 平台选择事件
    const platformRadios = document.querySelectorAll('input[name="platform"]');
    platformRadios.forEach(radio => {
        radio.addEventListener('change', () => {
            if (window.DeviceFormModule && window.DeviceFormModule.updatePlatformFields) {
                window.DeviceFormModule.updatePlatformFields();
            }
        });
    });

    // 连接类型选择事件
    const connectionRadios = document.querySelectorAll('input[name="connectionType"]');
    connectionRadios.forEach(radio => {
        radio.addEventListener('change', () => {
            if (window.DeviceFormModule && window.DeviceFormModule.updatePlatformFields) {
                window.DeviceFormModule.updatePlatformFields();
            }
        });
    });

    // 连接设备向导
    if (connectDeviceBtn) {
        connectDeviceBtn.addEventListener('click', () => {
            if (window.ConnectionGuideUI && window.ConnectionGuideUI.showConnectionGuide) {
                window.ConnectionGuideUI.showConnectionGuide();
            }
        });
    }

    if (closeGuideBtn) {
        closeGuideBtn.addEventListener('click', () => {
            if (window.ConnectionGuideUI && window.ConnectionGuideUI.hideConnectionGuide) {
                window.ConnectionGuideUI.hideConnectionGuide();
            }
        });
    }

    // 点击模态框外部关闭
    if (connectionGuideModal) {
        connectionGuideModal.addEventListener('click', (e) => {
            if (e.target === connectionGuideModal) {
                if (window.ConnectionGuideUI && window.ConnectionGuideUI.hideConnectionGuide) {
                    window.ConnectionGuideUI.hideConnectionGuide();
                }
            }
        });
    }

    // 配对方式选择
    if (showPairingCodeBtn) {
        showPairingCodeBtn.addEventListener('click', () => {
            if (window.ConnectionGuideUI && window.ConnectionGuideUI.showPairingMethod) {
                window.ConnectionGuideUI.showPairingMethod('code');
            }
        });
    }

    if (showQrCodeBtn) {
        showQrCodeBtn.addEventListener('click', () => {
            if (window.ConnectionGuideUI && window.ConnectionGuideUI.showPairingMethod) {
                window.ConnectionGuideUI.showPairingMethod('qr');
            }
        });
    }

    // 配对码连接
    if (connectWithPairingCodeBtn) {
        connectWithPairingCodeBtn.addEventListener('click', () => {
            if (window.ConnectionPairing && window.ConnectionPairing.connectWithPairingCode) {
                window.ConnectionPairing.connectWithPairingCode();
            }
        });
    }

    // QR码生成
    if (generateQrBtn) {
        generateQrBtn.addEventListener('click', () => {
            if (window.ConnectionQR && window.ConnectionQR.generateQRCode) {
                window.ConnectionQR.generateQRCode();
            }
        });
    }

    if (refreshQrBtn) {
        refreshQrBtn.addEventListener('click', () => {
            if (window.ConnectionQR && window.ConnectionQR.generateQRCode) {
                window.ConnectionQR.generateQRCode();
            }
        });
    }

    // 连接方式切换
    if (window.ConnectionGuideUI && window.ConnectionGuideUI.initializeMethodSwitcher) {
        window.ConnectionGuideUI.initializeMethodSwitcher();
    }

    // 连接向导标签页切换
    if (window.ConnectionGuideUI && window.ConnectionGuideUI.initializeConnectionTabs) {
        window.ConnectionGuideUI.initializeConnectionTabs();
    }

    // IP地址同步
    if (window.ConnectionGuideUI && window.ConnectionGuideUI.initializeIpSync) {
        window.ConnectionGuideUI.initializeIpSync();
    }

    if (newDeviceForm) {
        newDeviceForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            window.rLog('表单提交事件触发');

            if (!window.AppGlobals.currentProject) {
                window.AppNotifications?.warn('Please open a project first');
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
                window.AppNotifications?.success(
                    mode === 'edit' ? 'Device updated successfully' : 'Device saved successfully'
                );

                deviceForm.style.display = 'none';
                newDeviceForm.reset();
                delete deviceForm.dataset.mode;
                delete deviceForm.dataset.filename;

                // 重新加载设备列表
                if (window.DeviceListModule) {
                    if (window.DeviceListModule.loadSavedDevices) {
                        await window.DeviceListModule.loadSavedDevices();
                    }
                    if (window.DeviceListModule.refreshDeviceList) {
                        await window.DeviceListModule.refreshDeviceList();
                    }
                }
            } catch (error) {
                window.AppNotifications?.error(`Failed to save device: ${error.message}`);
            }
        });
    }

    // 加载保存的设备
    if (window.DeviceListModule && window.DeviceListModule.loadSavedDevices) {
        window.DeviceListModule.loadSavedDevices();
    }

    // 确保页面加载时字段显示正确
    if (window.DeviceFormModule && window.DeviceFormModule.updatePlatformFields) {
        window.DeviceFormModule.updatePlatformFields();
    }

    // 监听配对成功事件
    const { ipcRenderer } = getGlobals();
    ipcRenderer.on('pairing-success', (event, data) => {
        window.rLog('收到配对成功事件:', data);
        window.AppNotifications?.success(`配对成功！来自设备: ${data.remoteAddress}`);

        // 重置QR码显示
        setTimeout(() => {
            if (window.ConnectionQR && window.ConnectionQR.resetQRDisplay) {
                window.ConnectionQR.resetQRDisplay();
            }
            if (window.DeviceListModule && window.DeviceListModule.refreshConnectedDevices) {
                window.DeviceListModule.refreshConnectedDevices();
            }
        }, 2000);
    });
}

// 导出模块
window.DeviceInitializer = {
    loadConnectionGuideModal,
    initializeDevicePage
};

// 注册全局函数
window.loadConnectionGuideModal = loadConnectionGuideModal;
window.initializeDevicePage = initializeDevicePage;
