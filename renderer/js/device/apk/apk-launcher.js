// APK启动相关功能
// 负责安装APK后自动启动应用
// 依赖：renderer-logger.js, window.AppNotifications

// 自动启动刚安装的应用
async function autoLaunchAppAfterInstall(deviceId, packageName) {
    if (!deviceId || !packageName) {
        window.rLog('缺少设备ID或包名，跳过自动启动');
        return;
    }

    try {
        window.AppNotifications?.info('正在启动应用...');

        // 首先尝试获取应用的主Activity
        const mainActivity = await getMainActivity(deviceId, packageName);

        if (mainActivity) {
            // 使用TKE ADB启动应用
            await launchAppWithTKE(deviceId, packageName, mainActivity);
        } else {
            // 如果无法获取主Activity，使用monkey方式启动
            await launchAppWithMonkey(deviceId, packageName);
        }

    } catch (error) {
        window.rError('自动启动应用失败:', error);
        // 静默失败，不显示错误通知，因为安装已经成功了
    }
}

// 执行TKE ADB命令的辅助函数
async function executeTkeAdbCommand(deviceId, adbArgs) {
    const { ipcRenderer } = window.AppGlobals;

    try {
        const result = await ipcRenderer.invoke('execute-tke-adb-command', deviceId, adbArgs);
        return result;
    } catch (error) {
        window.rError('执行TKE ADB命令失败:', error);
        return { success: false, error: error.message };
    }
}

// 获取应用的主Activity
async function getMainActivity(deviceId, packageName) {
    try {
        // 使用TKE ADB直通命令获取包信息
        const dumpsysResult = await executeTkeAdbCommand(deviceId, ['shell', 'dumpsys', 'package', packageName]);

        if (dumpsysResult.success && dumpsysResult.output) {
            // 从输出中解析主Activity
            const lines = dumpsysResult.output.split('\n');

            for (const line of lines) {
                // 查找intent-filter中的MAIN和LAUNCHER
                if (line.includes('android.intent.action.MAIN') || line.includes('MAIN')) {
                    // 往上查找对应的Activity
                    const activityMatch = dumpsysResult.output.match(new RegExp(`${packageName}/([^\\s]+).*?android\\.intent\\.action\\.MAIN`, 's'));
                    if (activityMatch) {
                        const activityName = activityMatch[1];
                        window.rLog('找到主Activity:', activityName);
                        return activityName.startsWith('.') ? packageName + activityName : activityName;
                    }
                }
            }

            // 尝试另一种解析方式
            const activityPattern = new RegExp(`Activity #\\d+.*?${packageName}/([^\\s}]+)`, 'g');
            const activities = [...dumpsysResult.output.matchAll(activityPattern)];

            if (activities.length > 0) {
                const activityName = activities[0][1];
                window.rLog('找到Activity:', activityName);
                return activityName.startsWith('.') ? packageName + activityName : activityName;
            }
        }

        // 尝试使用pm dump获取
        const pmResult = await executeTkeAdbCommand(deviceId, ['shell', 'pm', 'dump', packageName]);

        if (pmResult.success && pmResult.output) {
            // 查找主Activity
            const mainActivityMatch = pmResult.output.match(/android\.intent\.action\.MAIN.*?\n.*?([A-Za-z0-9_.]+)/);
            if (mainActivityMatch) {
                const activityName = mainActivityMatch[1];
                window.rLog('通过pm dump找到主Activity:', activityName);
                return activityName.startsWith('.') ? packageName + activityName : activityName;
            }
        }

        window.rLog('未找到主Activity');
        return null;

    } catch (error) {
        window.rError('获取主Activity失败:', error);
        return null;
    }
}

// 使用TKE ADB启动应用
async function launchAppWithTKE(deviceId, packageName, activityName) {
    try {
        const componentName = `${packageName}/${activityName}`;
        window.rLog('使用TKE ADB启动应用:', componentName);

        const result = await executeTkeAdbCommand(deviceId, [
            'shell', 'am', 'start', '-n', componentName
        ]);

        if (result.success) {
            window.rLog('应用启动成功');
            window.AppNotifications?.success(`应用 ${packageName} 已启动`);
        } else {
            window.rError('TKE ADB启动失败:', result.error);
            // 尝试备用方案
            await launchAppWithMonkey(deviceId, packageName);
        }

    } catch (error) {
        window.rError('TKE ADB启动失败:', error);
        // 尝试备用方案
        await launchAppWithMonkey(deviceId, packageName);
    }
}

// 使用monkey方式启动应用（备用方案）
async function launchAppWithMonkey(deviceId, packageName) {
    try {
        window.rLog('使用monkey方式启动应用:', packageName);

        const result = await executeTkeAdbCommand(deviceId, [
            'shell', 'monkey', '-p', packageName, '-c', 'android.intent.category.LAUNCHER', '1'
        ]);

        if (result.success) {
            window.rLog('应用启动成功（monkey方式）');
            window.AppNotifications?.success(`应用 ${packageName} 已启动`);
        } else {
            window.rError('Monkey启动失败:', result.error);
        }

    } catch (error) {
        window.rError('Monkey启动失败:', error);
    }
}

// 导出函数
window.ApkLauncher = {
    autoLaunchAppAfterInstall,
    executeTkeAdbCommand,
    getMainActivity,
    launchAppWithTKE,
    launchAppWithMonkey
};
