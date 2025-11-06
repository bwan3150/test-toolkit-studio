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
        if (window.updatePlatformFields) {
            window.updatePlatformFields();
        }

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
    window.AppNotifications?.info('Please complete the device configuration');
}

// 导出模块
window.DeviceCreator = {
    createDeviceFromConnected
};

// 全局函数（为了保持向后兼容）
window.createDeviceFromConnected = createDeviceFromConnected;
