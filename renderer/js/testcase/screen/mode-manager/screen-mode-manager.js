// 设备屏幕模式管理器
// 支持四种模式:普通屏幕、XML overlay、截图取坐标、坐标点取值
// 本文件作为总控制器,协调各个子模块

const ScreenModeManager = {
    initialized: false,

    // 初始化模式管理器
    init() {
        // 防止重复初始化
        if (this.initialized) {
            window.rLog('ScreenModeManager 已经初始化过了,跳过重复初始化');
            return;
        }

        // 确保所有子模块已加载
        if (!window.CoordinateConverter || !window.ModeSlider ||
            !window.ModeSwitcher || !window.ScreenshotSelector ||
            !window.CoordinateMode) {
            window.rError('子模块未完全加载,无法初始化 ScreenModeManager');
            return;
        }

        // 设置模式滑块,并传入切换模式的回调
        window.ModeSlider.setupModeButtons((mode) => {
            this.switchMode(mode);
        });

        // 设置截图模式
        window.ScreenshotSelector.setup(window.CoordinateConverter);

        // 设置坐标模式
        window.CoordinateMode.setup(window.CoordinateConverter);

        this.initialized = true;
        window.rLog('ScreenModeManager 初始化完成');
    },

    // 切换模式(委托给ModeSwitcher)
    switchMode(mode) {
        window.ModeSwitcher.switchMode(mode, window.ModeSlider);
    },

    // 获取当前模式
    getCurrentMode() {
        return window.ModeSwitcher.getCurrentMode();
    },

    // 设置测试运行状态 - 禁用/启用模式切换
    setTestRunning(isRunning) {
        window.ModeSlider.setTestRunning(isRunning);

        // 如果测试开始,强制切换到纯屏幕模式
        if (isRunning && this.getCurrentMode() !== 'normal') {
            this.switchMode('normal');
        }
    },

    // 更新缩放控制的可见性(委托给ModeSwitcher)
    updateZoomControlsVisibility() {
        window.ModeSwitcher.updateZoomControlsVisibility();
    },

    // 坐标转换相关方法(委托给CoordinateConverter)
    getImageDisplayInfo() {
        return window.CoordinateConverter.getImageDisplayInfo();
    },

    screenToImageCoords(screenX, screenY) {
        return window.CoordinateConverter.screenToImageCoords(screenX, screenY);
    },

    imageToDeviceCoords(imageX, imageY) {
        return window.CoordinateConverter.imageToDeviceCoords(imageX, imageY);
    },

    isPointInImage(screenX, screenY) {
        return window.CoordinateConverter.isPointInImage(screenX, screenY);
    }
};

// 延迟初始化模式管理器,确保在 testcase 页面显示后再初始化
function initializeScreenModeManager() {
    window.rLog('开始初始化 ScreenModeManager');

    // 检查必要的 DOM 元素是否存在
    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');

    if (screenContent && screenshotSelector) {
        window.rLog('DOM 元素已准备好,初始化 ScreenModeManager');
        ScreenModeManager.init();
    } else {
        window.rLog('DOM 元素未准备好,延迟初始化', {
            screenContent: !!screenContent,
            screenshotSelector: !!screenshotSelector
        });
        // 延迟重试
        setTimeout(initializeScreenModeManager, 500);
    }
}

// 导出模块
window.ScreenModeManagerModule = {
    ScreenModeManager,
    initializeScreenModeManager
};
