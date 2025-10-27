// 模式滑块控制器模块
// 负责四种模式切换的UI交互(滑块动画和按钮状态)

const ModeSlider = {
    // 设置模式切换滑块
    setupModeButtons(switchModeCallback) {
        const modeOptions = document.querySelectorAll('.mode-option');
        const modeSlider = document.getElementById('modeSlider');

        modeOptions.forEach(option => {
            option.addEventListener('click', () => {
                // 检查是否被禁用(测试运行期间)
                if (option.classList.contains('disabled')) {
                    return;
                }

                const mode = option.dataset.mode;
                // 处理不同的模式名称映射
                let actualMode = mode;
                if (mode === 'crop') {
                    actualMode = 'screenshot';
                }

                // 调用回调函数切换模式
                if (switchModeCallback) {
                    switchModeCallback(actualMode);
                }
            });
        });

        // 初始化滑块位置
        this.updateSliderPosition('normal');
    },

    // 更新滑块位置
    updateSliderPosition(mode) {
        const modeSlider = document.getElementById('modeSlider');
        const modeOptions = document.querySelectorAll('.mode-option');

        if (!modeSlider) return;

        // 设置滑块的data-active属性来控制指示器位置
        modeSlider.setAttribute('data-active', mode);

        // 更新选项的激活状态
        modeOptions.forEach(option => {
            if (option.dataset.mode === mode) {
                option.classList.add('active');
            } else {
                option.classList.remove('active');
            }
        });

        // 为不同模式设置不同颜色
        const sliderIndicator = document.getElementById('sliderIndicator');
        if (sliderIndicator) {
            sliderIndicator.className = 'slider-indicator';
            sliderIndicator.classList.add(`mode-${mode}`);
        }
    },

    // 设置测试运行状态 - 禁用/启用模式切换
    setTestRunning(isRunning) {
        const modeOptions = document.querySelectorAll('.mode-option');

        modeOptions.forEach(option => {
            if (isRunning) {
                option.classList.add('disabled');
            } else {
                option.classList.remove('disabled');
            }
        });
    }
};

// 导出模块
window.ModeSlider = ModeSlider;
