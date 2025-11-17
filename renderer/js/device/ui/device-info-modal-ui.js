// 设备详细信息模态窗口UI控制器
// 负责显示设备的完整信息（通过 tke device info 获取）
// 依赖：renderer-logger.js, window.AppGlobals

let currentDeviceId = null;

// 显示设备信息模态窗口
async function showDeviceInfoModal(deviceId) {
    const modal = document.getElementById('deviceInfoModal');
    const title = document.getElementById('deviceInfoModalTitle');

    if (!modal) {
        window.rError('找不到设备信息模态窗口元素');
        return;
    }

    // 保存当前设备ID
    currentDeviceId = deviceId;

    // 设置标题
    if (title) {
        title.textContent = `Device Information - ${deviceId}`;
    }

    // 显示模态窗口
    modal.style.display = 'block';

    // 加载设备信息
    await loadDeviceInfo(deviceId);
}

// 加载设备信息
async function loadDeviceInfo(deviceId) {
    const loadingEl = document.getElementById('deviceInfoLoading');
    const errorEl = document.getElementById('deviceInfoError');
    const dataEl = document.getElementById('deviceInfoData');

    // 显示加载状态
    if (loadingEl) loadingEl.style.display = 'flex';
    if (errorEl) errorEl.style.display = 'none';
    if (dataEl) dataEl.style.display = 'none';

    try {
        const { ipcRenderer } = window.AppGlobals;
        if (!ipcRenderer) {
            throw new Error('ipcRenderer 未定义');
        }

        window.rLog('正在获取设备详细信息...', { deviceId });

        // 调用 tke-device-info handler
        const result = await ipcRenderer.invoke('tke-device-info', {
            deviceId: deviceId
        });

        if (result.success && result.output) {
            const deviceInfo = JSON.parse(result.output);
            window.rLog('成功获取设备详细信息:', deviceInfo);

            // 填充设备信息
            fillDeviceInfo(deviceInfo);

            // 隐藏加载状态，显示数据
            if (loadingEl) loadingEl.style.display = 'none';
            if (dataEl) dataEl.style.display = 'block';
        } else {
            throw new Error(result.error || '获取设备信息失败');
        }
    } catch (error) {
        window.rError('获取设备信息时出错:', error);

        // 显示错误状态
        if (loadingEl) loadingEl.style.display = 'none';
        if (errorEl) {
            errorEl.style.display = 'flex';
            const errorMessageEl = document.getElementById('deviceInfoErrorMessage');
            if (errorMessageEl) {
                errorMessageEl.textContent = `Failed to load device information: ${error.message}`;
            }
        }
    }
}

// 填充设备信息到UI
function fillDeviceInfo(deviceInfo) {
    // 基础信息
    setInfoValue('info-device-id', deviceInfo.id);
    setInfoValue('info-model', deviceInfo.model);
    setInfoValue('info-manufacturer', deviceInfo.manufacturer);
    setInfoValue('info-android-version', deviceInfo.android_version);
    setInfoValue('info-screen-resolution',
        deviceInfo.screen_width && deviceInfo.screen_height
            ? `${deviceInfo.screen_width} x ${deviceInfo.screen_height}`
            : '-'
    );

    // 硬件信息
    if (deviceInfo.hardware) {
        const hw = deviceInfo.hardware;
        setInfoValue('info-cpu-model', hw.cpu_model);
        setInfoValue('info-cpu-cores', hw.cpu_cores ? `${hw.cpu_cores} 核心` : '-');
        setInfoValue('info-cpu-abi', hw.cpu_abi);
        setInfoValue('info-total-memory', hw.total_memory_mb ? `${(hw.total_memory_mb / 1024).toFixed(2)} GB` : '-');
        setInfoValue('info-available-memory', hw.available_memory_mb ? `${(hw.available_memory_mb / 1024).toFixed(2)} GB` : '-');
        setInfoValue('info-total-storage', hw.total_storage_gb ? `${hw.total_storage_gb.toFixed(2)} GB` : '-');
        setInfoValue('info-available-storage', hw.available_storage_gb ? `${hw.available_storage_gb.toFixed(2)} GB` : '-');
    }

    // 电池信息
    if (deviceInfo.battery) {
        const bat = deviceInfo.battery;
        setInfoValue('info-battery-level', bat.level !== null && bat.level !== undefined ? `${bat.level}%` : '-');
        setInfoValue('info-battery-temperature', bat.temperature ? `${bat.temperature.toFixed(1)}°C` : '-');
        setInfoValue('info-battery-health', bat.health);
        setInfoValue('info-battery-status', bat.status);
        setInfoValue('info-battery-power-source', bat.power_source);
    }

    // 网络信息
    if (deviceInfo.network) {
        const net = deviceInfo.network;
        setInfoValue('info-wifi-enabled', net.wifi_enabled !== null && net.wifi_enabled !== undefined
            ? (net.wifi_enabled ? '已启用' : '已禁用')
            : '-'
        );
        setInfoValue('info-wifi-ssid', net.wifi_ssid || '-');
        setInfoValue('info-mobile-network-type', net.mobile_network_type || '-');
        setInfoValue('info-operator-name', net.operator_name || '-');
        setInfoValue('info-country-iso', net.country_iso || '-');
    }
}

// 设置信息值的辅助函数
function setInfoValue(elementId, value) {
    const el = document.getElementById(elementId);
    if (el) {
        el.textContent = value || '-';
    }
}

// 隐藏设备信息模态窗口
function hideDeviceInfoModal() {
    const modal = document.getElementById('deviceInfoModal');
    if (modal) {
        modal.style.display = 'none';
        currentDeviceId = null;
    }
}

// 刷新设备信息
async function refreshDeviceInfo() {
    if (currentDeviceId) {
        await loadDeviceInfo(currentDeviceId);
    }
}

// 初始化设备信息模态窗口的事件监听
function initializeDeviceInfoModal() {
    const modal = document.getElementById('deviceInfoModal');
    const closeBtn = document.getElementById('closeDeviceInfoModal');
    const closeBtn2 = document.getElementById('closeDeviceInfoModalBtn');
    const refreshBtn = document.getElementById('refreshDeviceInfo');

    // 关闭按钮
    if (closeBtn) {
        closeBtn.addEventListener('click', hideDeviceInfoModal);
    }

    if (closeBtn2) {
        closeBtn2.addEventListener('click', hideDeviceInfoModal);
    }

    // 刷新按钮
    if (refreshBtn) {
        refreshBtn.addEventListener('click', refreshDeviceInfo);
    }

    // 点击模态窗口外部关闭
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                hideDeviceInfoModal();
            }
        });
    }
}

// 导出模块
window.DeviceInfoModalUI = {
    showDeviceInfoModal,
    hideDeviceInfoModal,
    initializeDeviceInfoModal,
    refreshDeviceInfo
};

// 全局函数（为了方便在HTML中调用）
window.showDeviceInfoModal = showDeviceInfoModal;
