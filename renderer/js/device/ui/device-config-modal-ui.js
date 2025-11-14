// 设备配置模态窗口UI控制器
// 负责设备配置模态窗口的显示、隐藏和数据处理
// 依赖：renderer-logger.js, window.AppGlobals, window.AppNotifications

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// 显示设备配置模态窗口（新增模式）
function showDeviceConfigModal() {
    const modal = document.getElementById('deviceConfigModal');
    const form = document.getElementById('deviceConfigForm');
    const title = document.getElementById('deviceConfigModalTitle');

    if (!modal || !form) {
        window.rError('找不到设备配置模态窗口元素');
        return;
    }

    // 重置表单
    form.reset();

    // 设置标题为新增模式
    if (title) {
        title.textContent = 'Add New Device';
    }

    // 清除编辑模式标记
    delete modal.dataset.mode;
    delete modal.dataset.filename;

    // 更新表单字段显示
    if (window.updatePlatformFields) {
        window.updatePlatformFields();
    }

    // 显示模态窗口
    modal.style.display = 'block';
}

// 显示设备配置模态窗口（编辑模式）
function showDeviceConfigModalForEdit(filename, config) {
    const modal = document.getElementById('deviceConfigModal');
    const form = document.getElementById('deviceConfigForm');
    const title = document.getElementById('deviceConfigModalTitle');

    if (!modal || !form) {
        window.rError('找不到设备配置模态窗口元素');
        return;
    }

    // 设置标题为编辑模式
    if (title) {
        title.textContent = 'Edit Device';
    }

    // 设置编辑模式标记
    modal.dataset.mode = 'edit';
    modal.dataset.filename = filename;

    // 填充表单数据
    fillFormWithConfig(form, config);

    // 显示模态窗口
    modal.style.display = 'block';
}

// 用配置数据填充表单
function fillFormWithConfig(form, config) {
    if (!form || !config) return;

    // 填充基础字段
    const deviceNameInput = form.querySelector('input[name="deviceName"]');
    if (deviceNameInput) deviceNameInput.value = config.deviceName || '';

    const platformVersionInput = form.querySelector('input[name="platformVersion"]');
    if (platformVersionInput) platformVersionInput.value = config.platformVersion || '';

    // 设置平台
    const platform = config.platform || 'android';
    const platformRadio = form.querySelector(`input[name="platform"][value="${platform}"]`);
    if (platformRadio) {
        platformRadio.checked = true;
    }

    // 设置连接类型
    const connectionType = config.connectionType || (config.ipAddress ? 'wifi' : 'usb');
    const connectionRadio = form.querySelector(`input[name="connectionType"][value="${connectionType}"]`);
    if (connectionRadio) {
        connectionRadio.checked = true;
    }

    // 填充高级设置
    const automationNameInput = form.querySelector('input[name="automationName"]');
    if (automationNameInput) {
        automationNameInput.value = config.automationName || 'UiAutomator2';
    }

    const timeoutInput = form.querySelector('input[name="newCommandTimeout"]');
    if (timeoutInput) {
        timeoutInput.value = config.newCommandTimeout || '6000';
    }

    const noResetSelect = form.querySelector('select[name="noReset"]');
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

// 隐藏设备配置模态窗口
function hideDeviceConfigModal() {
    const modal = document.getElementById('deviceConfigModal');
    if (modal) {
        modal.style.display = 'none';

        // 清除表单和标记
        const form = document.getElementById('deviceConfigForm');
        if (form) {
            form.reset();
        }
        delete modal.dataset.mode;
        delete modal.dataset.filename;
    }
}

// 从连接的设备预填充表单
function prefillDeviceConfigForm(deviceId) {
    const form = document.getElementById('deviceConfigForm');
    if (!form) return;

    // 判断是否为WiFi设备
    const isWifi = deviceId.includes(':');

    // 设置平台（默认Android）
    const androidRadio = form.querySelector('input[name="platform"][value="android"]');
    if (androidRadio) {
        androidRadio.checked = true;
    }

    // 设置连接类型
    const connectionType = isWifi ? 'wifi' : 'usb';
    const connectionRadio = form.querySelector(`input[name="connectionType"][value="${connectionType}"]`);
    if (connectionRadio) {
        connectionRadio.checked = true;
    }

    if (isWifi) {
        const [ip, port] = deviceId.split(':');
        form.querySelector('input[name="deviceName"]').value = `WiFi Device (${ip})`;
    } else {
        form.querySelector('input[name="deviceName"]').value = `Device ${deviceId.substring(0, 8)}`;
    }

    // 设置平台版本默认值
    form.querySelector('input[name="platformVersion"]').value = '14';

    // 填充高级设置
    const automationNameInput = form.querySelector('input[name="automationName"]');
    if (automationNameInput) {
        automationNameInput.value = 'UiAutomator2';
    }

    const timeoutInput = form.querySelector('input[name="newCommandTimeout"]');
    if (timeoutInput) {
        timeoutInput.value = '6000';
    }

    const noResetSelect = form.querySelector('select[name="noReset"]');
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

// 初始化设备配置模态窗口的事件监听
function initializeDeviceConfigModal() {
    const modal = document.getElementById('deviceConfigModal');
    const closeBtn = document.getElementById('closeDeviceConfigModal');
    const confirmBtn = document.getElementById('confirmDeviceConfig');
    const cancelBtn = document.getElementById('cancelDeviceConfig');
    const form = document.getElementById('deviceConfigForm');
    const advancedToggle = document.getElementById('modalAdvancedToggle');

    // 关闭按钮
    if (closeBtn) {
        closeBtn.addEventListener('click', hideDeviceConfigModal);
    }

    // 取消按钮
    if (cancelBtn) {
        cancelBtn.addEventListener('click', hideDeviceConfigModal);
    }

    // 点击模态窗口外部关闭
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                hideDeviceConfigModal();
            }
        });
    }

    // 确认按钮 - 保存设备配置
    if (confirmBtn) {
        confirmBtn.addEventListener('click', async () => {
            if (form) {
                // 触发表单验证
                if (!form.checkValidity()) {
                    form.reportValidity();
                    return;
                }

                // 提交表单
                await saveDeviceConfig();
            }
        });
    }

    // 高级设置切换
    if (advancedToggle) {
        advancedToggle.addEventListener('click', () => {
            const content = document.getElementById('advancedSettingsContent');
            const icon = advancedToggle.querySelector('.toggle-icon');

            if (content && icon) {
                if (content.style.display === 'none' || content.style.display === '') {
                    content.style.display = 'block';
                    icon.style.transform = 'rotate(180deg)';
                } else {
                    content.style.display = 'none';
                    icon.style.transform = 'rotate(0deg)';
                }
            }
        });
    }

    // 平台和连接类型变化时更新字段
    const platformRadios = form?.querySelectorAll('input[name="platform"]');
    platformRadios?.forEach(radio => {
        radio.addEventListener('change', () => {
            if (window.updatePlatformFields) {
                window.updatePlatformFields();
            }
        });
    });

    const connectionRadios = form?.querySelectorAll('input[name="connectionType"]');
    connectionRadios?.forEach(radio => {
        radio.addEventListener('change', () => {
            if (window.updatePlatformFields) {
                window.updatePlatformFields();
            }
        });
    });
}

// 保存设备配置
async function saveDeviceConfig() {
    const { fs, yaml, path } = getGlobals();
    const modal = document.getElementById('deviceConfigModal');
    const form = document.getElementById('deviceConfigForm');

    if (!window.AppGlobals.currentProject) {
        window.AppNotifications?.warn('Please open a project first');
        return;
    }

    if (!form) return;

    const formData = new FormData(form);
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
    const mode = modal?.dataset.mode;
    let devicePath;

    if (mode === 'edit' && modal?.dataset.filename) {
        // 编辑模式 - 使用现有文件名
        devicePath = path.join(window.AppGlobals.currentProject, 'devices', modal.dataset.filename);
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

        // 隐藏模态窗口
        hideDeviceConfigModal();

        // 重新加载设备列表
        if (window.DeviceLoader) {
            if (window.DeviceLoader.loadSavedDevices) {
                await window.DeviceLoader.loadSavedDevices();
            }
            if (window.DeviceLoader.refreshDeviceList) {
                await window.DeviceLoader.refreshDeviceList();
            }
        }
    } catch (error) {
        window.AppNotifications?.error(`Failed to save device: ${error.message}`);
    }
}

// 导出模块
window.DeviceConfigModalUI = {
    showDeviceConfigModal,
    showDeviceConfigModalForEdit,
    hideDeviceConfigModal,
    prefillDeviceConfigForm,
    initializeDeviceConfigModal,
    fillFormWithConfig,
    saveDeviceConfig
};
