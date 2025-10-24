// XML Overlay 管理模块
// 负责XML overlay的开启、关闭和切换逻辑

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 切换 XML overlay 状态
async function toggleXmlOverlay() {
    window.rLog('🔘 toggleXmlOverlay 被调用');
    const deviceSelect = document.getElementById('deviceSelect');

    if (!deviceSelect?.value) {
        window.AppNotifications?.deviceRequired();
        return;
    }

    // 切换状态
    const newState = !window.ScreenState.xmlOverlayEnabled;

    if (newState) {
        // 先尝试启用,成功后再设置状态
        await enableXmlOverlay(deviceSelect.value);
        // enableXmlOverlay 内部会设置状态
    } else {
        // 禁用时直接设置状态
        window.ScreenState.setXmlOverlayEnabled(false);
        disableXmlOverlay();
    }
}

// 启用 XML overlay
async function enableXmlOverlay(deviceId) {
    window.rLog(`🎯 启用 XML Overlay, deviceId = ${deviceId}`);

    try {
        // 不需要提示 - 用户已通过UI操作知道

        const { ipcRenderer } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;

        // 1. 先确保有截图
        const deviceImage = document.getElementById('deviceScreenshot');
        if (!deviceImage || !deviceImage.complete || deviceImage.naturalWidth === 0) {
            window.rLog('设备截图未加载,先刷新屏幕');
            if (window.ScreenCapture && window.ScreenCapture.refreshDeviceScreen) {
                await window.ScreenCapture.refreshDeviceScreen();
            }

            // 等待图片加载完成
            await new Promise((resolve, reject) => {
                const img = document.getElementById('deviceScreenshot');
                if (img.complete) {
                    resolve();
                } else {
                    img.onload = resolve;
                    img.onerror = () => reject(new Error('截图加载失败'));
                }
            });
        }

        // 2. 使用 TKE fetcher infer-screen-size 推断屏幕尺寸
        const sizeResult = await ipcRenderer.invoke('tke-fetcher-infer-screen-size', projectPath);

        let screenSize = { width: 1080, height: 1920 }; // 默认值
        if (sizeResult.success) {
            try {
                const sizeData = JSON.parse(sizeResult.output);
                screenSize = {
                    width: sizeData.width,
                    height: sizeData.height
                };
                window.rLog('推断屏幕尺寸:', screenSize);
            } catch (e) {
                window.rWarn('解析屏幕尺寸失败,使用默认值:', e);
            }
        }

        // 如果有保存的屏幕尺寸且合理,也可以使用
        if (window.AppGlobals.currentScreenSize &&
            window.AppGlobals.currentScreenSize.width > 0 &&
            window.AppGlobals.currentScreenSize.height > 0) {
            // 比较两个尺寸,如果差异太大,使用推断的
            const savedSize = window.AppGlobals.currentScreenSize;
            const widthDiff = Math.abs(savedSize.width - screenSize.width);
            const heightDiff = Math.abs(savedSize.height - screenSize.height);

            if (widthDiff < 100 && heightDiff < 100) {
                // 差异不大,使用保存的尺寸(可能更准确)
                screenSize = savedSize;
                window.rLog(`使用保存的屏幕尺寸: ${screenSize.width}x${screenSize.height}`);
            }
        }

        // 3. 使用 TKE fetcher extract-ui-elements 提取 UI 元素
        const extractResult = await ipcRenderer.invoke('tke-fetcher-extract-ui-elements', {
            projectPath: projectPath,
            screenWidth: screenSize.width,
            screenHeight: screenSize.height
        });

        if (!extractResult.success) {
            throw new Error('TKE提取UI元素失败: ' + (extractResult.error || '未知错误'));
        }

        // 解析 TKE 返回的 JSON
        let elements;
        try {
            elements = JSON.parse(extractResult.output);
        } catch (e) {
            throw new Error('解析TKE输出失败: ' + e.message);
        }

        if (!Array.isArray(elements)) {
            throw new Error('TKE返回的数据格式不正确');
        }

        // 存储元素和屏幕尺寸
        window.ScreenState.currentUIElements = elements;
        window.ScreenState.currentScreenSize = screenSize;

        // 4. 创建 overlay
        if (window.OverlayRenderer && window.OverlayRenderer.createUIOverlay) {
            await window.OverlayRenderer.createUIOverlay(window.ScreenState.currentUIElements, window.ScreenState.currentScreenSize);
        }

        // 5. 显示元素列表
        if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
            window.UIExtractor.displayUIElementList(window.ScreenState.currentUIElements);
        }

        // 6. 更新按钮状态
        const toggleBtn = document.getElementById('toggleXmlBtn');
        if (toggleBtn) {
            toggleBtn.style.background = '#4CAF50';
            toggleBtn.setAttribute('title', '关闭XML Overlay');
        }

        // 成功后设置状态为 true
        window.ScreenState.setXmlOverlayEnabled(true);

        // 不需要Toast - 用户通过滑块UI已知道切换成功
        window.rLog(`✅ XML Overlay 启用成功! 元素数量 = ${window.ScreenState.currentUIElements.length}`);

    } catch (error) {
        const errorMsg = error?.message || error?.toString() || JSON.stringify(error) || '未知错误';
        window.rError('❌ 启用XML Overlay失败:', errorMsg, error);
        window.AppNotifications?.error(`启用XML Overlay失败: ${errorMsg}`);
        window.ScreenState.setXmlOverlayEnabled(false);
    }
}

// 禁用 XML overlay
function disableXmlOverlay() {
    window.rLog('📊 禁用 XML Overlay');

    // 移除UI叠层
    const screenContent = document.getElementById('screenContent');
    if (screenContent) {
        const overlay = screenContent.querySelector('.ui-overlay');
        if (overlay) {
            overlay.remove();
        }
    }

    // 清空UI元素列表
    if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
        window.UIExtractor.displayUIElementList([]);
    }

    // 重置状态
    window.ScreenState.currentUIElements = [];
    window.ScreenState.selectedElement = null;
    window.ScreenState.setXmlOverlayEnabled(false);

    // 更新按钮状态
    const toggleBtn = document.getElementById('toggleXmlBtn');
    if (toggleBtn) {
        toggleBtn.style.background = '';
        toggleBtn.setAttribute('title', '启用XML Overlay');
    }

    // 断开 ResizeObserver
    if (window.ScreenState.resizeObserver) {
        window.ScreenState.resizeObserver.disconnect();
        window.ScreenState.resizeObserver = null;
    }

    // 不需要Toast - 用户通过滑块UI已知道
}

// 更新 XML overlay(当屏幕刷新时)
async function updateXmlOverlay(elements, screenSize) {
    window.rLog('🔄 更新 XML Overlay');

    // 更新状态
    window.ScreenState.currentUIElements = elements;
    window.ScreenState.currentScreenSize = screenSize;

    // 先移除旧的overlay
    const screenContent = document.getElementById('screenContent');
    const existingOverlay = screenContent?.querySelector('.ui-overlay');
    if (existingOverlay) {
        existingOverlay.remove();
    }

    // 等待一帧,确保DOM更新完成
    await new Promise(resolve => requestAnimationFrame(resolve));

    // 重新创建 overlay
    if (window.OverlayRenderer && window.OverlayRenderer.createUIOverlay) {
        await window.OverlayRenderer.createUIOverlay(elements, screenSize);
    }
}

// 导出模块
window.XmlOverlayManager = {
    toggleXmlOverlay,
    enableXmlOverlay,
    disableXmlOverlay,
    updateXmlOverlay
};
