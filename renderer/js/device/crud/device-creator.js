// 设备创建模块
// 负责从连接的设备创建新的设备配置
// 依赖：window.AppGlobals, window.AppNotifications

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 从连接的设备创建设备
async function createDeviceFromConnected(deviceId) {
    if (!window.AppGlobals.currentProject) {
        window.AppNotifications?.warn('Please open a project first');
        return;
    }

    // 显示设备配置模态窗口
    if (window.DeviceConfigModalUI && window.DeviceConfigModalUI.showDeviceConfigModal) {
        window.DeviceConfigModalUI.showDeviceConfigModal();

        // 预填充设备ID到表单（包括自动获取 Android 版本）
        if (window.DeviceConfigModalUI.prefillDeviceConfigForm) {
            await window.DeviceConfigModalUI.prefillDeviceConfigForm(deviceId);
        }
    } else {
        window.rError('设备配置模态窗口UI控制器未加载');
    }
}

// 导出模块
window.DeviceCreator = {
    createDeviceFromConnected
};

// 全局函数（为了保持向后兼容）
window.createDeviceFromConnected = createDeviceFromConnected;
