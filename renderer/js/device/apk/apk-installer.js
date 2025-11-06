// APK安装器模块
// 负责处理APK文件的拖拽和安装逻辑
// 依赖：
// - ./apk-ui.js - APK安装UI
// - ./apk-launcher.js - APK启动功能

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 设置设备卡片的拖拽功能
function setupDragAndDropForDevice(deviceCard, deviceId) {
    // 防止默认拖拽行为
    deviceCard.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.add('drag-over');
    });

    deviceCard.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');
    });

    deviceCard.addEventListener('drop', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        deviceCard.classList.remove('drag-over');

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
        // 第一步：先用 AAPT 获取包名和主Activity
        if (window.ApkUI && window.ApkUI.showApkInstallLoading) {
            window.ApkUI.showApkInstallLoading('正在解析APK信息...', '');
        }

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

        // 第二步：开始安装
        if (window.ApkUI && window.ApkUI.updateApkInstallLoading) {
            window.ApkUI.updateApkInstallLoading('正在安装APK...', '', packageName);
        }

        const result = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, false);
        window.rLog('收到安装结果:', result);

        if (result.success) {
            // 安装成功，使用已获取的包名启动应用
            if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
                window.ApkUI.hideApkInstallLoading();
            }
            if (window.ApkLauncher && window.ApkLauncher.autoLaunchAppAfterInstall) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }
            window.AppNotifications?.success('APK安装成功！');

            // 刷新设备列表
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
            return;
        }

        // 第三步：安装失败，显示确认对话框询问是否卸载重装
        window.rLog('安装失败，显示卸载重装确认对话框');
        if (window.ApkUI && window.ApkUI.hideApkInstallLoading) {
            window.ApkUI.hideApkInstallLoading();
        }
        if (window.ApkUI && window.ApkUI.showApkInstallModalWithPackage) {
            window.ApkUI.showApkInstallModalWithPackage(deviceId, apkPath, packageName);
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
