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
            if (window.updatePlatformFields) {
                window.updatePlatformFields();
            }

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

            // 重新加载设备列表
            if (window.DeviceLoader) {
                await window.DeviceLoader.loadSavedDevices();
                await window.DeviceLoader.refreshDeviceList();
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
