// 设备编辑模块
// 负责编辑和删除已保存的设备配置（使用数据库）
// device_name 作为主键
// 依赖：window.AppGlobals, window.AppNotifications, window.DeviceLoader

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 编辑设备配置（使用设备名作为主键）
async function editDevice(deviceName) {
    const { ipcRenderer } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    try {
        // 从数据库获取设备配置
        const result = await ipcRenderer.invoke('db-device-get', projectPath, deviceName);

        if (!result.success) {
            window.AppNotifications?.error(`Failed to load device: ${result.error}`);
            return;
        }

        const config = result.data;

        // 显示设备配置模态窗口（编辑模式）
        if (window.DeviceConfigModalUI && window.DeviceConfigModalUI.showDeviceConfigModalForEdit) {
            window.DeviceConfigModalUI.showDeviceConfigModalForEdit(deviceName, config);
        } else {
            window.rError('设备配置模态窗口UI控制器未加载');
        }
    } catch (error) {
        window.AppNotifications?.error(`Failed to load device: ${error.message}`);
    }
}

// 删除设备配置（使用设备名作为主键）
async function deleteDevice(deviceName) {
    const { ipcRenderer } = getGlobals();
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    if (confirm(`Are you sure you want to delete device "${deviceName}"?`)) {
        try {
            const result = await ipcRenderer.invoke('db-device-delete', projectPath, deviceName);

            if (result.success) {
                window.AppNotifications?.success('Device configuration deleted');

                // 刷新设备页面
                if (window.DeviceScanner && window.DeviceScanner.refreshConnectedDevices) {
                    await window.DeviceScanner.refreshConnectedDevices();
                }
            } else {
                window.AppNotifications?.error(`Failed to delete device: ${result.error}`);
            }
        } catch (error) {
            window.AppNotifications?.error(`Failed to delete device: ${error.message}`);
        }
    }
}

// 导出模块
window.DeviceEditor = {
    editDevice,
    deleteDevice
};

// 全局函数（为了保持向后兼容）
window.editDevice = editDevice;
window.deleteDevice = deleteDevice;
