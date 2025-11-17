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

// 动态加载设备配置模态框
async function loadDeviceConfigModal() {
    const container = document.getElementById('deviceConfigModalContainer');
    if (!container) return;

    try {
        const response = await fetch('modals/device-config-modal.html');
        if (response.ok) {
            const html = await response.text();
            container.innerHTML = html;

            // 加载完成后初始化模态窗口事件
            if (window.DeviceConfigModalUI && window.DeviceConfigModalUI.initializeDeviceConfigModal) {
                window.DeviceConfigModalUI.initializeDeviceConfigModal();
            }
        }
    } catch (error) {
        window.rError('Failed to load device config modal:', error);
    }
}

// 动态加载设备详细信息模态框
async function loadDeviceInfoModal() {
    const container = document.getElementById('deviceInfoModalContainer');
    if (!container) return;

    try {
        const response = await fetch('modals/device-info-modal.html');
        if (response.ok) {
            const html = await response.text();
            container.innerHTML = html;

            // 加载完成后初始化模态窗口事件
            if (window.DeviceInfoModalUI && window.DeviceInfoModalUI.initializeDeviceInfoModal) {
                window.DeviceInfoModalUI.initializeDeviceInfoModal();
            }
        }
    } catch (error) {
        window.rError('Failed to load device info modal:', error);
    }
}

// 初始化设备页面
async function initializeDevicePage() {
    const { fs, yaml } = getGlobals();

    // 动态加载模态框
    await loadConnectionGuideModal();
    await loadDeviceConfigModal();
    await loadDeviceInfoModal();

    const addDeviceBtn = document.getElementById('addDeviceBtn');
    const scanDevicesBtn = document.getElementById('scanDevicesBtn');

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

    // 添加设备按钮 - 打开设备配置模态窗口
    if (addDeviceBtn) {
        addDeviceBtn.addEventListener('click', () => {
            if (window.DeviceConfigModalUI && window.DeviceConfigModalUI.showDeviceConfigModal) {
                window.DeviceConfigModalUI.showDeviceConfigModal();
            }
        });
    }

    // 刷新设备列表按钮
    if (scanDevicesBtn) {
        scanDevicesBtn.addEventListener('click', () => {
            if (window.DeviceScanner && window.DeviceScanner.refreshConnectedDevices) {
                window.DeviceScanner.refreshConnectedDevices();
            }
        });
    }

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

    // 加载保存的设备
    if (window.DeviceLoader && window.DeviceLoader.loadSavedDevices) {
        window.DeviceLoader.loadSavedDevices();
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
            if (window.DeviceScanner && window.DeviceScanner.refreshConnectedDevices) {
                window.DeviceScanner.refreshConnectedDevices();
            }
        }, 2000);
    });
}

// 导出模块
window.DeviceInitializer = {
    loadConnectionGuideModal,
    loadDeviceConfigModal,
    loadDeviceInfoModal,
    initializeDevicePage
};

// 注册全局函数
window.loadConnectionGuideModal = loadConnectionGuideModal;
window.loadDeviceConfigModal = loadDeviceConfigModal;
window.loadDeviceInfoModal = loadDeviceInfoModal;
window.initializeDevicePage = initializeDevicePage;
