// APK安装器模块
// 负责处理APK文件的拖拽和安装逻辑
// 依赖：
// - ./apk-ui.js - APK安装UI
// - ./apk-launcher.js - APK启动功能

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 显示APK安装图标提示
function showApkInstallIndicator(deviceCard) {
    // 检查是否已存在指示器
    let indicator = deviceCard.querySelector('.apk-install-indicator');
    if (indicator) {
        indicator.style.display = 'flex';
        return;
    }

    // 创建指示器
    indicator = document.createElement('div');
    indicator.className = 'apk-install-indicator';
    indicator.innerHTML = `
        <svg class="apk-install-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"/>
            <path d="M14 2v5a1 1 0 0 0 1 1h5"/>
            <path d="M12 18v-6"/>
            <path d="m9 15 3 3 3-3"/>
        </svg>
        <div class="apk-install-text">松手安装APK</div>
    `;

    deviceCard.appendChild(indicator);
}

// 隐藏APK安装图标提示
function hideApkInstallIndicator(deviceCard) {
    const indicator = deviceCard.querySelector('.apk-install-indicator');
    if (indicator) {
        indicator.style.display = 'none';
    }
}

// 显示设备未连接提示
function showDisconnectIndicator(deviceCard, isWifi = false) {
    // 检查是否已存在指示器
    let indicator = deviceCard.querySelector('.device-disconnect-indicator');
    if (indicator) {
        indicator.style.display = 'flex';
        return;
    }

    // 选择图标（WiFi或USB离线）
    const iconSvg = isWifi ? `
        <svg class="device-disconnect-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20h.01"/>
            <path d="M8.5 16.429a5 5 0 0 1 7 0"/>
            <path d="M5 12.859a10 10 0 0 1 5.17-2.69"/>
            <path d="M19 12.859a10 10 0 0 0-2.007-1.523"/>
            <path d="M2 8.82a15 15 0 0 1 4.177-2.643"/>
            <path d="M22 8.82a15 15 0 0 0-11.288-3.764"/>
            <path d="m2 2 20 20"/>
        </svg>
    ` : `
        <svg class="device-disconnect-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 17H7A5 5 0 0 1 7 7"/>
            <path d="M15 7h2a5 5 0 0 1 4 8"/>
            <line x1="8" x2="12" y1="12" y2="12"/>
            <line x1="2" x2="22" y1="2" y2="22"/>
        </svg>
    `;

    // 创建指示器
    indicator = document.createElement('div');
    indicator.className = 'device-disconnect-indicator';
    indicator.innerHTML = `
        ${iconSvg}
        <div>设备未连接</div>
    `;

    deviceCard.appendChild(indicator);
}

// 隐藏设备未连接提示
function hideDisconnectIndicator(deviceCard) {
    const indicator = deviceCard.querySelector('.device-disconnect-indicator');
    if (indicator) {
        indicator.style.display = 'none';
    }
}

// 设置设备卡片的拖拽功能
function setupDragAndDropForDevice(deviceCard, deviceId, isConnected = true, isWifi = false) {
    // 如果设备未连接，禁用拖放功能
    if (!isConnected) {
        deviceCard.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.stopPropagation();
            // 添加禁用样式
            deviceCard.classList.add('drag-disabled');
            // 显示未连接提示
            showDisconnectIndicator(deviceCard, isWifi);
        });

        deviceCard.addEventListener('dragleave', (e) => {
            e.preventDefault();
            e.stopPropagation();
            deviceCard.classList.remove('drag-disabled');
            // 隐藏未连接提示
            hideDisconnectIndicator(deviceCard);
        });

        deviceCard.addEventListener('drop', (e) => {
            e.preventDefault();
            e.stopPropagation();
            deviceCard.classList.remove('drag-disabled');
            // 隐藏未连接提示
            hideDisconnectIndicator(deviceCard);
            window.AppNotifications?.warn('设备未连接，无法安装APK');
        });

        return;
    }

    // 防止默认拖拽行为
    deviceCard.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.add('drag-over');

        // 显示安装图标提示
        showApkInstallIndicator(deviceCard);
    });

    deviceCard.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');

        // 隐藏安装图标提示
        hideApkInstallIndicator(deviceCard);
    });

    deviceCard.addEventListener('drop', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');

        // 隐藏安装图标提示
        hideApkInstallIndicator(deviceCard);

        const files = e.dataTransfer.files;

        // 检查是否有文件
        if (files.length === 0) {
            return;
        }

        // 检查是否为APK文件
        const file = files[0];
        if (!file.name.toLowerCase().endsWith('.apk')) {
            window.AppNotifications?.warn('请拖入APK文件');
            return;
        }

        // 获取文件路径 - 使用Electron的webUtils.getPathForFile()方法
        let filePath = null;

        try {
            const { webUtils } = require('electron');
            filePath = webUtils.getPathForFile(file);
        } catch (error) {
            window.rError('无法获取文件路径:', error.message);
            window.AppNotifications?.error('无法获取文件路径，请确保应用有文件访问权限');
            return;
        }

        if (!filePath) {
            window.rError('文件路径为空');
            window.AppNotifications?.error('无法获取文件路径');
            return;
        }

        // 安装APK
        await installApkToDevice(deviceId, filePath);
    });
}

// 安装APK到设备
async function installApkToDevice(deviceId, apkPath) {
    const { ipcRenderer } = getGlobals();

    if (!deviceId || !apkPath) {
        window.AppNotifications?.error('设备ID或APK路径无效');
        return;
    }

    try {
        // 第一步：尝试用 AAPT 获取包名（可选，失败不阻止安装）
        if (window.ApkUI && window.ApkUI.showApkInstallLoading) {
            window.ApkUI.showApkInstallLoading('正在解析APK信息...', '');
        }

        let packageName = null;
        const packageInfo = await ipcRenderer.invoke('get-apk-package-name', apkPath);

        if (packageInfo.success && packageInfo.packageName) {
            packageName = packageInfo.packageName;
            window.rLog('通过 AAPT 获取到包名:', packageName);
        } else {
            // AAPT解析失败，但继续安装
            window.rWarn('AAPT解析APK失败，将直接安装（无法获取包名）:', packageInfo.error);
        }

        // 第二步：开始安装
        if (window.ApkUI && window.ApkUI.updateApkInstallLoading) {
            window.ApkUI.updateApkInstallLoading('正在安装APK...', '', packageName || '');
        }

        const result = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, false);
        window.rLog('收到安装结果:', result);

        if (result.success) {
            // 安装成功
            if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
                window.ApkUI.hideApkInstallLoading();
            }

            // 如果有包名，尝试启动应用
            if (packageName && window.ApkLauncher && window.ApkLauncher.autoLaunchAppAfterInstall) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }
            window.AppNotifications?.success('APK安装成功！');

            // 刷新设备列表
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
            return;
        }

        // 第三步：安装失败
        window.rLog('安装失败，错误信息:', result.error);
        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }

        // 如果有包名，显示卸载重装确认对话框
        if (packageName && window.ApkUI && window.ApkUI.showApkInstallModalWithPackage) {
            window.ApkUI.showApkInstallModalWithPackage(deviceId, apkPath, packageName);
        } else {
            // 没有包名，直接显示错误
            window.AppNotifications?.error(`安装失败: ${result.error || '未知错误'}`);
        }

    } catch (error) {
        window.rError('安装APK失败:', error);
        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }
        window.AppNotifications?.error(`安装失败: ${error.message}`);
    }
}

// 卸载并重新安装APK（从UI调用）
async function uninstallAndReinstallApk() {
    const { ipcRenderer } = getGlobals();

    try {
        // 获取模态框中存储的设备ID和APK路径
        const modal = document.getElementById('apkInstallModal');
        const deviceId = modal?.dataset?.deviceId;
        const apkPath = modal?.dataset?.apkPath;

        if (!deviceId || !apkPath) {
            window.AppNotifications?.error('设备ID或APK路径无效');
            return;
        }

        // 关闭模态框
        if (window.ApkUI && window.ApkUI.hideApkInstallModal) {
            window.ApkUI.hideApkInstallModal();
        }

        // 显示加载状态
        if (window.ApkUI && window.ApkUI.showApkInstallLoading) {
            window.ApkUI.showApkInstallLoading('正在解析APK信息...', '');
        }

        // 第一步：先用 AAPT 获取包名
        const packageInfo = await ipcRenderer.invoke('get-apk-package-name', apkPath);

        if (!packageInfo.success || !packageInfo.packageName) {
            if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
                window.ApkUI.hideApkInstallLoading();
            }
            window.AppNotifications?.error('无法解析APK文件');
            return;
        }

        const packageName = packageInfo.packageName;
        window.rLog('通过 AAPT 获取到包名:', packageName);

        // 第二步：卸载现有应用
        if (window.ApkUI && window.ApkUI.updateApkInstallLoading) {
            window.ApkUI.updateApkInstallLoading('正在卸载旧版本...', '', packageName);
        }

        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);
        window.rLog('卸载结果:', uninstallResult);

        // 第三步：安装新APK
        if (window.ApkUI && window.ApkUI.updateApkInstallLoading) {
            window.ApkUI.updateApkInstallLoading('正在安装APK...', '', packageName);
        }

        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, false);
        window.rLog('安装结果:', installResult);

        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }

        if (installResult.success) {
            window.AppNotifications?.success('APK卸载并重新安装成功！');

            // 启动应用
            if (window.ApkLauncher && window.ApkLauncher.autoLaunchAppAfterInstall) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }

            // 刷新设备列表
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
        } else {
            window.AppNotifications?.error(`安装失败: ${installResult.error || '未知错误'}`);
        }

    } catch (error) {
        window.rError('卸载并重新安装APK失败:', error);
        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }
        window.AppNotifications?.error(`操作失败: ${error.message}`);
    }
}

// 卸载并重新安装APK（已知包名版本）
async function uninstallAndReinstallWithKnownPackage(deviceId, apkPath, packageName) {
    const { ipcRenderer } = getGlobals();

    try {
        // 显示加载状态
        if (window.ApkUI && window.ApkUI.showApkInstallLoading) {
            window.ApkUI.showApkInstallLoading('正在卸载旧版本...', '', packageName);
        }

        // 第一步：卸载现有应用
        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);
        window.rLog('卸载结果:', uninstallResult);

        // 第二步：安装新APK
        if (window.ApkUI && window.ApkUI.updateApkInstallLoading) {
            window.ApkUI.updateApkInstallLoading('正在安装APK...', '', packageName);
        }

        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, false);
        window.rLog('安装结果:', installResult);

        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }

        if (installResult.success) {
            window.AppNotifications?.success('APK卸载并重新安装成功！');

            // 启动应用
            if (window.ApkLauncher && window.ApkLauncher.autoLaunchAppAfterInstall) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }

            // 刷新设备列表
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
        } else {
            window.AppNotifications?.error(`安装失败: ${installResult.error || '未知错误'}`);
        }

    } catch (error) {
        window.rError('卸载并重新安装APK失败:', error);
        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }
        window.AppNotifications?.error(`操作失败: ${error.message}`);
    }
}

// 导出模块
window.ApkInstaller = {
    setupDragAndDropForDevice,
    installApkToDevice,
    uninstallAndReinstallApk,
    uninstallAndReinstallWithKnownPackage
};

// 注册全局函数
window.setupDragAndDropForDevice = setupDragAndDropForDevice;
window.installApkToDevice = installApkToDevice;
window.uninstallAndReinstallApk = uninstallAndReinstallApk;
window.uninstallAndReinstallWithKnownPackage = uninstallAndReinstallWithKnownPackage;
