// 设备编辑模块
// 负责编辑和删除已保存的设备配置
// 依赖：window.AppGlobals, window.AppNotifications, window.DeviceLoader

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 编辑设备配置
async function editDevice(filename) {
    const { path, fs, yaml } = getGlobals();
    if (!window.AppGlobals.currentProject) return;

    try {
        const devicePath = path.join(window.AppGlobals.currentProject, 'devices', filename);
        const content = await fs.readFile(devicePath, 'utf-8');
        const config = yaml.load(content);

        // 显示设备配置模态窗口（编辑模式）
        if (window.DeviceConfigModalUI && window.DeviceConfigModalUI.showDeviceConfigModalForEdit) {
            window.DeviceConfigModalUI.showDeviceConfigModalForEdit(filename, config);
        } else {
            window.rError('设备配置模态窗口UI控制器未加载');
        }
    } catch (error) {
        window.AppNotifications?.error(`Failed to load device: ${error.message}`);
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
            window.AppNotifications?.success('Device configuration deleted');

            // 刷新设备页面
            if (window.DeviceScanner && window.DeviceScanner.refreshConnectedDevices) {
                await window.DeviceScanner.refreshConnectedDevices();
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
