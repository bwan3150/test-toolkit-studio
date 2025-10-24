// 屏幕状态管理模块
// 负责管理XML Overlay状态、UI元素列表、屏幕尺寸等全局状态

const ScreenState = {
    // XML Overlay 状态
    xmlOverlayEnabled: false,
    currentUIElements: [],
    currentScreenSize: null,
    selectedElement: null,

    // 观察器
    resizeObserver: null,

    // 设置状态并同步到全局
    setXmlOverlayEnabled(value) {
        this.xmlOverlayEnabled = value;
        window.xmlOverlayEnabled = value; // 向后兼容
        window.rLog(`📊 XML Overlay 状态更新: ${value}`);
    },

    reset() {
        this.xmlOverlayEnabled = false;
        this.currentUIElements = [];
        this.currentScreenSize = null;
        this.selectedElement = null;
        window.xmlOverlayEnabled = false;
    }
};

// 导出模块
window.ScreenState = ScreenState;
