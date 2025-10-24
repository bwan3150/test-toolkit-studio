// 模式切换管理器模块
// 负责在四种模式之间切换的核心逻辑(normal, xml, screenshot, coordinate)

const ModeSwitcher = {
    currentMode: 'normal', // 'normal', 'xml', 'screenshot', 'coordinate'

    // 切换模式
    switchMode(mode, modeSlider) {
        this.currentMode = mode;
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');
        const coordinateMarker = document.getElementById('coordinateMarker');

        // 重置所有模式选项状态
        document.querySelectorAll('.mode-option').forEach(option => option.classList.remove('active'));

        // 清理之前的模式状态
        screenContent.classList.remove('screenshot-mode', 'coordinate-mode');
        if (screenshotSelector) screenshotSelector.style.display = 'none';
        if (coordinateMarker) coordinateMarker.style.display = 'none';

        // 先禁用任何活动的 XML overlay
        const existingOverlay = document.querySelector('.ui-overlay');
        if (existingOverlay) {
            existingOverlay.remove();
        }

        // 更新滑块位置和激活状态
        let uiMode = mode;
        if (mode === 'screenshot') {
            uiMode = 'crop'; // UI中显示为crop模式
        }

        if (modeSlider) {
            modeSlider.updateSliderPosition(uiMode);
        }

        switch(mode) {
            case 'normal':
                // 纯屏幕模式,不显示任何覆盖层
                window.xmlOverlayEnabled = false;
                break;

            case 'xml':
                // 启用XML overlay
                window.xmlOverlayEnabled = true;
                const deviceSelect = document.getElementById('deviceSelect');
                if (deviceSelect?.value && window.TestcaseController?.enableXmlOverlay) {
                    window.TestcaseController.enableXmlOverlay(deviceSelect.value);
                }
                break;

            case 'screenshot':
                screenContent.classList.add('screenshot-mode');
                window.xmlOverlayEnabled = false;
                break;

            case 'coordinate':
                screenContent.classList.add('coordinate-mode');
                window.xmlOverlayEnabled = false;
                break;
        }

        // 显示或隐藏缩放控制
        this.updateZoomControlsVisibility();
    },

    // 更新缩放控制的可见性
    updateZoomControlsVisibility() {
        const screenZoomControls = document.getElementById('screenZoomControls');
        const deviceImage = document.getElementById('deviceScreenshot');

        if (screenZoomControls && deviceImage && deviceImage.style.display !== 'none') {
            screenZoomControls.style.display = 'flex';
        } else if (screenZoomControls) {
            screenZoomControls.style.display = 'none';
        }
    },

    // 获取当前模式
    getCurrentMode() {
        return this.currentMode;
    }
};

// 导出模块
window.ModeSwitcher = ModeSwitcher;
