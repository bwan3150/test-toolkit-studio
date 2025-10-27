// 设备屏幕管理器
// 负责设备屏幕截图刷新、XML overlay 显示和 UI 元素管理
// 本文件作为总控制器,协调各个子模块

// 导出模块 - 整合所有子模块的功能
window.DeviceScreenManagerModule = {
    // 从 ScreenCapture 模块导出
    refreshDeviceScreen: () => {
        if (window.ScreenCapture && window.ScreenCapture.refreshDeviceScreen) {
            return window.ScreenCapture.refreshDeviceScreen();
        }
    },

    // 从 UIExtractor 模块导出
    updateDeviceInfoAndGetUIStructure: (xmlPath) => {
        if (window.UIExtractor && window.UIExtractor.updateDeviceInfoAndGetUIStructure) {
            return window.UIExtractor.updateDeviceInfoAndGetUIStructure(xmlPath);
        }
    },

    // 从 XmlOverlayManager 模块导出
    toggleXmlOverlay: () => {
        if (window.XmlOverlayManager && window.XmlOverlayManager.toggleXmlOverlay) {
            return window.XmlOverlayManager.toggleXmlOverlay();
        }
    },

    enableXmlOverlay: (deviceId) => {
        if (window.XmlOverlayManager && window.XmlOverlayManager.enableXmlOverlay) {
            return window.XmlOverlayManager.enableXmlOverlay(deviceId);
        }
    },

    disableXmlOverlay: () => {
        if (window.XmlOverlayManager && window.XmlOverlayManager.disableXmlOverlay) {
            return window.XmlOverlayManager.disableXmlOverlay();
        }
    },

    // 从 UIExtractor 模块导出
    displayUIElementList: (elements) => {
        if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
            return window.UIExtractor.displayUIElementList(elements);
        }
    },

    // 从 OverlayRenderer 模块导出
    selectElement: (index) => {
        if (window.OverlayRenderer && window.OverlayRenderer.selectElement) {
            return window.OverlayRenderer.selectElement(index);
        }
    },

    // 导出状态(只读)
    getState: () => {
        if (window.ScreenState) {
            return {
                xmlOverlayEnabled: window.ScreenState.xmlOverlayEnabled,
                currentUIElements: window.ScreenState.currentUIElements,
                currentScreenSize: window.ScreenState.currentScreenSize,
                selectedElement: window.ScreenState.selectedElement
            };
        }
        return {};
    }
};

// 向后兼容: 确保全局变量存在
if (!window.xmlOverlayEnabled && window.ScreenState) {
    window.xmlOverlayEnabled = window.ScreenState.xmlOverlayEnabled;
}
