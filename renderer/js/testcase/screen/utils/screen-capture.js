// 屏幕截图功能模块
// 负责调用TKE获取设备截图并显示

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 刷新设备屏幕截图
async function refreshDeviceScreen() {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;

    if (!deviceSelect?.value) {
        window.AppNotifications?.deviceRequired();
        return;
    }

    if (!projectPath) {
        window.AppNotifications?.projectRequired();
        return;
    }

    // 如果存在提示按钮，将其改为 loading 状态
    if (window.ScreenPrompt && window.ScreenPrompt.setButtonLoading) {
        window.ScreenPrompt.setButtonLoading();
    }

    // 如果XML overlay已启用,先移除overlay UI(但保持状态)
    const wasXmlOverlayEnabled = window.ScreenState.xmlOverlayEnabled;
    if (wasXmlOverlayEnabled) {
        window.rLog('🔄 刷新前先移除XML overlay UI');
        const screenContent = document.getElementById('screenContent');
        const existingOverlay = screenContent?.querySelector('.ui-overlay');
        if (existingOverlay) {
            existingOverlay.remove();
        }
    }

    window.rLog('开始截图,设备:', deviceSelect.value, '项目路径:', projectPath);

    // 使用新的 TKE controller capture 命令
    const captureResult = await ipcRenderer.invoke('tke-controller-capture', deviceSelect.value, projectPath);

    if (!captureResult.success) {
        const error = captureResult.error || '未知错误';
        window.rError('截图失败:', error);
        window.AppNotifications?.error(`截图失败: ${error}`);

        // 隐藏截图
        const img = document.getElementById('deviceScreenshot');
        if (img) {
            img.style.display = 'none';
        }

        // 恢复按钮状态（如果存在）
        if (window.ScreenPrompt && window.ScreenPrompt.resetButton) {
            window.ScreenPrompt.resetButton();
        }
        return;
    }

    // 解析 TKE 返回的 JSON
    let result;
    try {
        result = JSON.parse(captureResult.output);
    } catch (e) {
        window.rError('解析TKE输出失败:', e);
        window.AppNotifications?.error('解析截图结果失败');

        // 恢复按钮状态（如果存在）
        if (window.ScreenPrompt && window.ScreenPrompt.resetButton) {
            window.ScreenPrompt.resetButton();
        }
        return;
    }

    window.rLog('截图结果:', { success: result.success, hasScreenshot: !!result.screenshot });

    if (result.success && result.screenshot) {
        const imagePath = result.screenshot;
        const img = document.getElementById('deviceScreenshot');
        if (!img) {
            window.rError('未找到 deviceScreenshot 元素');
            return;
        }

        // 等待图片加载完成
        await new Promise((resolve) => {
            img.onload = () => {
                img.style.display = 'block';
                window.rLog('截图显示成功');

                // 移除提示按钮（如果存在）
                if (window.ScreenPrompt && window.ScreenPrompt.removePrompt) {
                    window.ScreenPrompt.removePrompt();
                }

                // 给浏览器一点时间完成布局
                setTimeout(resolve, 50);
            };
            img.src = `file://${imagePath}?t=${Date.now()}`;
        });

        // 更新设备信息并获取UI结构(传入xml路径)
        if (window.UIExtractor && window.UIExtractor.updateDeviceInfoAndGetUIStructure) {
            await window.UIExtractor.updateDeviceInfoAndGetUIStructure(result.xml);
        }

        // 检查设备状态并更新滑块（截图成功后应该解锁滑块）
        if (window.ScreenCoordinator && window.ScreenCoordinator.checkDeviceStatusAndPrompt) {
            window.ScreenCoordinator.checkDeviceStatusAndPrompt();
        }
    }
}

// 导出模块
window.ScreenCapture = {
    refreshDeviceScreen
};
