/**
 * 设备管理模块 - 主入口文件
 *
 * 这是设备管理的主入口文件，负责整合所有拆分后的子模块。
 * 所有功能已拆分为独立的子模块，本文件仅负责导入和组织这些模块。
 *
 * 子模块结构：
 * - core/device-utils.js - 工具函数
 * - core/device-globals.js - 全局变量获取
 * - ui/device-ui-fields.js - UI字段显示控制
 * - apk/apk-launcher.js - APK启动功能
 * - apk/apk-ui.js - APK安装UI
 * - ios/wda-helper.js - iOS WDA辅助
 * - device-loader.js - 设备加载
 * - device-editor.js - 设备编辑
 * - device-creator.js - 设备创建
 * - wireless/wireless-connection.js - 无线连接
 * - wireless/qr-pairing.js - QR码配对
 * - wireless/pairing-code.js - 配对码连接
 * - device-scanner.js - 设备扫描
 * - ui/connection-guide-ui.js - 连接向导UI
 * - apk/apk-installer.js - APK安装器
 * - device-initializer.js - 设备页面初始化
 */

// ==================== 模块组织 ====================

/**
 * 设备管理模块对象
 * 包含对所有子模块的引用
 */
window.DeviceManagerModule = {
    // 核心模块
    DeviceUtils: window.DeviceUtils,
    DeviceGlobals: window.DeviceGlobals,

    // UI模块
    DeviceUIFields: window.DeviceUIFields,
    ApkUI: window.ApkUI,
    ConnectionGuideUI: window.ConnectionGuideUI,

    // APK功能模块
    ApkLauncher: window.ApkLauncher,
    ApkInstaller: window.ApkInstaller,

    // iOS模块
    WdaHelper: window.WdaHelper,

    // 设备管理模块
    DeviceLoader: window.DeviceLoader,
    DeviceEditor: window.DeviceEditor,
    DeviceCreator: window.DeviceCreator,
    DeviceScanner: window.DeviceScanner,

    // 无线连接模块
    WirelessConnection: window.WirelessConnection,
    QRPairing: window.QRPairing,
    PairingCode: window.PairingCode,

    // 初始化模块
    DeviceInitializer: window.DeviceInitializer,

    // 便捷函数映射（直接暴露常用函数）
    refreshDeviceList: window.DeviceLoader?.refreshDeviceList,
    refreshConnectedDevices: window.DeviceScanner?.refreshConnectedDevices,
    loadSavedDevices: window.DeviceLoader?.loadSavedDevices,
    initializeDevicePage: window.DeviceInitializer?.initializeDevicePage
};

// ==================== 全局函数映射 ====================

// 设备管理函数
window.createDeviceFromConnected = window.DeviceCreator?.createDeviceFromConnected;
window.editDevice = window.DeviceEditor?.editDevice;
window.deleteDevice = window.DeviceEditor?.deleteDevice;
window.loadSavedDevices = window.DeviceLoader?.loadSavedDevices;
window.refreshDeviceList = window.DeviceLoader?.refreshDeviceList;
window.refreshConnectedDevices = window.DeviceScanner?.refreshConnectedDevices;

// UI函数
window.copyToClipboard = window.DeviceUtils?.copyToClipboard;
window.toggleAdvancedSettings = window.DeviceUIFields?.toggleAdvancedSettings;

// 无线连接函数
window.connectWirelessDevice = window.WirelessConnection?.connectWirelessDevice;
window.disconnectWirelessDevice = window.WirelessConnection?.disconnectWirelessDevice;

// 连接向导函数
window.showConnectionGuide = window.ConnectionGuideUI?.showConnectionGuide;
window.hideConnectionGuide = window.ConnectionGuideUI?.hideConnectionGuide;
window.showConnectionMethod = window.ConnectionGuideUI?.showConnectionMethod;
window.initializeConnectionTabs = window.ConnectionGuideUI?.initializeConnectionTabs;
window.initializeIpSync = window.ConnectionGuideUI?.initializeIpSync;

// 配对函数
window.showPairingMethod = window.PairingCode?.showPairingMethod;
window.connectWithPairingCode = window.PairingCode?.connectWithPairingCode;
window.generateQRCode = window.QRPairing?.generateQRCode;

// APK安装函数
window.cancelApkInstall = window.ApkUI?.cancelApkInstall;
window.confirmApkUninstall = window.ApkUI?.confirmApkUninstall;

// iOS WDA函数
window.showWdaSetupGuide = window.WdaHelper?.showWdaSetupGuide;
window.hideWdaGuide = window.WdaHelper?.hideWdaGuide;
window.retryWdaConnection = window.WdaHelper?.retryWdaConnection;

// ==================== 页面初始化 ====================

/**
 * 初始化设备页面
 * 委托给 DeviceInitializer 模块处理
 */
function initializeDevicePage() {
    if (window.DeviceInitializer && window.DeviceInitializer.initializeDevicePage) {
        window.DeviceInitializer.initializeDevicePage();
    } else {
        window.rError('DeviceInitializer 模块未加载');
    }
}

// 在 DOM 加载完成后初始化设备页面
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initializeDevicePage);
} else {
    // DOM 已经加载完成
    initializeDevicePage();
}
