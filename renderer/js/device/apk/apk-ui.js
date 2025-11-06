// APK安装UI相关功能
// 负责APK安装过程中的loading提示和确认对话框
// 依赖：renderer-logger.js, window.AppNotifications, apk-launcher.js

// APK安装Loading状态管理
function showApkInstallLoading(title, message) {
    const modal = document.getElementById('apkInstallLoadingModal');
    const titleElement = document.getElementById('apkLoadingTitle');
    const messageElement = document.getElementById('apkLoadingMessage');
    const detailsElement = document.getElementById('apkLoadingDetails');

    if (modal && titleElement && messageElement) {
        titleElement.textContent = title;
        messageElement.textContent = message;
        detailsElement.style.display = 'none'; // 初始时隐藏详情
        modal.style.display = 'block';
    }
}

function updateApkInstallLoading(title, message, packageName = null) {
    const titleElement = document.getElementById('apkLoadingTitle');
    const messageElement = document.getElementById('apkLoadingMessage');
    const detailsElement = document.getElementById('apkLoadingDetails');
    const packageNameElement = document.getElementById('apkPackageName');

    if (titleElement && messageElement) {
        titleElement.textContent = title;
        messageElement.textContent = message;

        if (packageName && detailsElement && packageNameElement) {
            packageNameElement.textContent = packageName;
            detailsElement.style.display = 'block';
        }
    }
}

function hideApkInstallLoading() {
    const modal = document.getElementById('apkInstallLoadingModal');
    if (modal) {
        modal.style.display = 'none';
    }
}

// 显示APK安装确认模态框（带已知包名）
function showApkInstallModalWithPackage(deviceId, apkPath, packageName) {
    const modal = document.getElementById('apkInstallModal');
    if (!modal) {
        window.rError('找不到APK安装模态框');
        return;
    }

    // 更新模态框内容，显示具体的包名信息
    const messageElement = modal.querySelector('.modal-message');
    if (messageElement) {
        messageElement.innerHTML = `
            <p><strong>签名不匹配</strong></p>
            <p>应用：<code>${packageName}</code></p>
            <p>需要先卸载现有版本才能安装。</p>
        `;
    }

    // 设置确认按钮的事件处理器
    const confirmBtn = modal.querySelector('#confirmUninstallBtn');
    const cancelBtn = modal.querySelector('#cancelUninstallBtn');

    if (confirmBtn) {
        // 移除之前的事件监听器
        confirmBtn.replaceWith(confirmBtn.cloneNode(true));
        const newConfirmBtn = modal.querySelector('#confirmUninstallBtn');

        newConfirmBtn.addEventListener('click', async () => {
            modal.style.display = 'none';
            // 使用已知的包名直接进行卸载重装
            await uninstallAndReinstallWithKnownPackage(deviceId, apkPath, packageName);
        });
    }

    if (cancelBtn) {
        cancelBtn.replaceWith(cancelBtn.cloneNode(true));
        const newCancelBtn = modal.querySelector('#cancelUninstallBtn');

        newCancelBtn.addEventListener('click', () => {
            modal.style.display = 'none';
            // 确保loading modal也被隐藏
            hideApkInstallLoading();
        });
    }

    modal.style.display = 'block';
}

// 显示APK安装确认模态框（原版，保留兼容性）
function showApkInstallModal(deviceId, apkPath) {
    const modal = document.getElementById('apkInstallModal');

    if (modal) {
        modal.style.display = 'flex';

        // 保存当前的设备ID和APK路径
        window.pendingApkInstall = {
            deviceId: deviceId,
            apkPath: apkPath
        };
    } else {
        // 如果模态框不存在，使用confirm作为备用
        if (confirm('APK签名冲突，是否先卸载原应用再安装？\n\n注意：卸载会清除应用数据！')) {
            uninstallAndReinstallApk(deviceId, apkPath);
        }
    }
}

// 隐藏APK安装确认模态框
function hideApkInstallModal() {
    const modal = document.getElementById('apkInstallModal');
    if (modal) {
        modal.style.display = 'none';
    }
    window.pendingApkInstall = null;
}

// 取消APK安装
window.cancelApkInstall = function() {
    hideApkInstallModal();
    window.AppNotifications?.info('已取消安装');
};

// 确认卸载并安装
window.confirmApkUninstall = async function() {
    if (!window.pendingApkInstall) {
        hideApkInstallModal();
        return;
    }

    const { deviceId, apkPath } = window.pendingApkInstall;
    hideApkInstallModal();

    // 执行卸载并重新安装
    await uninstallAndReinstallApk(deviceId, apkPath);
};

// 卸载并重新安装APK
async function uninstallAndReinstallApk(deviceId, apkPath) {
    const { ipcRenderer } = window.AppGlobals;

    try {
        // 先尝试获取APK的包名
        window.AppNotifications?.info('正在获取APK信息...');

        // 尝试通过aapt获取包名
        let packageName = null;
        const packageInfo = await ipcRenderer.invoke('get-apk-package-name', apkPath);

        if (packageInfo.success && packageInfo.packageName) {
            packageName = packageInfo.packageName;
        } else {
            // 如果无法自动获取包名，尝试通过安装错误信息获取
            window.AppNotifications?.warn('无法自动获取包名，尝试直接安装...');

            // 直接尝试强制安装，不卸载
            const directInstallResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);
            if (directInstallResult.success) {
                window.AppNotifications?.success('APK安装成功！');
                await refreshConnectedDevices();
                return; // 安装成功，直接返回
            } else if (directInstallResult.packageName) {
                // 从安装错误中获取到了包名，使用该包名继续卸载重装流程
                packageName = directInstallResult.packageName;
                window.AppNotifications?.info(`从错误信息中获取到包名: ${packageName}，继续卸载重装...`);
                // 不return，继续执行后面的卸载重装逻辑
            } else {
                window.AppNotifications?.error('无法获取包名，请手动卸载原应用后重试');
                return;
            }
        }

        // 卸载应用（卸载的是与APK相同包名的已安装版本）
        window.AppNotifications?.info(`正在卸载已安装的 ${packageName}...`);
        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);

        if (!uninstallResult.success) {
            // 如果卸载失败，可能是应用不存在，直接尝试安装
            window.AppNotifications?.info('原应用可能不存在，尝试直接安装...');
        }

        // 安装新的APK
        window.AppNotifications?.info('正在安装APK...');
        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);

        if (installResult.success) {
            // 尝试自动启动应用
            if (packageName && window.ApkLauncher) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }

            window.AppNotifications?.success('APK安装成功！');
            // 需要调用外部的refreshConnectedDevices
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
        } else {
            window.AppNotifications?.error(`安装失败: ${installResult.error}`);
        }

    } catch (error) {
        window.rError('卸载并重装失败:', error);
        window.AppNotifications?.error(`操作失败: ${error.message}`);
    }
}

// 使用已知包名进行卸载重装（简化版本，无需提取包名）
async function uninstallAndReinstallWithKnownPackage(deviceId, apkPath, packageName) {
    const { ipcRenderer } = window.AppGlobals;

    try {
        window.rLog(`开始卸载重装流程，包名: ${packageName}`);

        // 显示卸载loading
        showApkInstallLoading('正在卸载...', '');
        // 然后更新显示包名
        updateApkInstallLoading('正在卸载...', '', packageName);

        // 第一步：卸载已安装的应用
        const uninstallResult = await ipcRenderer.invoke('adb-uninstall-app', deviceId, packageName);

        if (!uninstallResult.success) {
            // 如果卸载失败，可能是应用不存在，继续尝试安装
            updateApkInstallLoading('准备安装...', '', packageName);
        } else {
            updateApkInstallLoading('正在安装...', '', packageName);
        }

        // 第二步：安装新的APK
        const installResult = await ipcRenderer.invoke('adb-install-apk', deviceId, apkPath, true);

        // 隐藏loading
        hideApkInstallLoading();

        if (installResult.success) {
            // 尝试自动启动应用
            if (packageName && window.ApkLauncher) {
                await window.ApkLauncher.autoLaunchAppAfterInstall(deviceId, packageName);
            }

            window.AppNotifications?.success('APK安装成功！');
            // 需要调用外部的refreshConnectedDevices
            if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
                await window.DeviceManagerModule.refreshConnectedDevices();
            }
        } else {
            window.AppNotifications?.error(`安装失败: ${installResult.error}`);
        }

    } catch (error) {
        window.rError('卸载并重装失败:', error);
        hideApkInstallLoading();
        window.AppNotifications?.error(`操作失败: ${error.message}`);
    }
}

// 导出函数
window.ApkUI = {
    showApkInstallLoading,
    updateApkInstallLoading,
    hideApkInstallLoading,
    showApkInstallModal,
    showApkInstallModalWithPackage,
    hideApkInstallModal,
    uninstallAndReinstallApk,
    uninstallAndReinstallWithKnownPackage
};
