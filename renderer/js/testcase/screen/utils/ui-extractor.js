// UI元素提取模块
// 负责从TKE获取UI结构并更新到状态

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 更新设备信息并获取UI结构
async function updateDeviceInfoAndGetUIStructure(xmlPath) {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;

    if (!deviceSelect?.value || !projectPath) return;

    window.rLog(`🔄 获取设备UI结构, 当前 overlay 状态: ${window.ScreenState.xmlOverlayEnabled}`);

    try {
        // 1. 首先使用 TKE fetcher infer-screen-size 推断屏幕尺寸
        const sizeResult = await ipcRenderer.invoke('tke-fetcher-infer-screen-size', projectPath);

        let screenSize = { width: 1080, height: 1920 }; // 默认值
        if (sizeResult.success) {
            try {
                const sizeData = JSON.parse(sizeResult.output);
                screenSize = {
                    width: sizeData.width,
                    height: sizeData.height
                };
                window.rLog('屏幕尺寸:', screenSize);
            } catch (e) {
                window.rWarn('解析屏幕尺寸失败,使用默认值:', e);
            }
        }

        // 显示图片尺寸信息
        const deviceImage = document.getElementById('deviceScreenshot');
        if (deviceImage && deviceImage.complete) {
            const rect = deviceImage.getBoundingClientRect();
            window.rLog('图片显示信息:', {
                left: rect.left,
                top: rect.top,
                width: rect.width,
                height: rect.height
            });
        }

        // 2. 使用 TKE fetcher extract-ui-elements 提取 UI 元素
        window.rLog('开始通过 TKE 提取 UI 元素...');

        const extractResult = await ipcRenderer.invoke('tke-fetcher-extract-ui-elements', {
            projectPath: projectPath,
            screenWidth: screenSize.width,
            screenHeight: screenSize.height
        });

        if (!extractResult.success) {
            window.rError('TKE 提取UI元素失败:', extractResult.error);
            window.AppNotifications?.error('提取UI元素失败: ' + (extractResult.error || '未知错误'));
            return;
        }

        // 解析 TKE 返回的 JSON
        let elements;
        try {
            elements = JSON.parse(extractResult.output);
        } catch (e) {
            window.rError('解析TKE输出失败:', e);
            window.AppNotifications?.error('解析UI元素失败');
            return;
        }

        if (Array.isArray(elements)) {
            window.rLog(`TKE 提取到 ${elements.length} 个UI元素`);

            // 显示UI元素列表
            displayUIElementList(elements);

            // 如果XML overlay 已启用,重新创建overlay UI
            if (window.ScreenState.xmlOverlayEnabled) {
                window.rLog('📊 重新创建XML overlay UI');

                // 更新状态
                window.ScreenState.currentUIElements = elements;
                window.ScreenState.currentScreenSize = screenSize;

                // 创建新的overlay
                if (window.OverlayRenderer && window.OverlayRenderer.createUIOverlay) {
                    await window.OverlayRenderer.createUIOverlay(elements, screenSize);
                }
            }

            // 存储当前屏幕尺寸
            window.AppGlobals.currentScreenSize = screenSize;

            // 如果有TKE适配器,更新屏幕信息
            if (window.TkeAdapterModule && window.TkeAdapterModule.updateScreenInfo) {
                window.TkeAdapterModule.updateScreenInfo(screenSize);
            }
        }

    } catch (error) {
        window.rError('Error updating device info:', error);
        window.AppNotifications?.error('更新设备信息失败: ' + error.message);
    }
}

// 显示UI元素列表
function displayUIElementList(elements) {
    // 触发自定义事件,通知UI元素列表面板更新
    const event = new CustomEvent('uiElementsUpdated', {
        detail: { elements }
    });
    document.dispatchEvent(event);

    window.rLog(`触发UI元素更新事件,元素数量: ${elements.length}`);
}

// 导出模块
window.UIExtractor = {
    updateDeviceInfoAndGetUIStructure,
    displayUIElementList
};
