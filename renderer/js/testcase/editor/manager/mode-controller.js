// 模式控制模块
const ModeController = {
    /**
     * 设置全局快捷键
     */
    setupGlobalKeyboardShortcuts() {
        // 全局监听 Cmd/Ctrl + / 用于切换模式
        this.globalModeToggleHandler = (e) => {
            // 检查是否在 Testcase 页面
            const testcasePage = document.getElementById('testcasePage');
            const isTestcasePageActive = testcasePage && testcasePage.classList.contains('active');

            if (!isTestcasePageActive) {
                return;
            }

            if (e.key === '/' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                e.stopPropagation();
                e.stopImmediatePropagation();

                window.rLog('全局快捷键触发模式切换');
                this.toggleGlobalEditMode();
                return false;
            }
        };

        // 使用最高优先级监听
        document.addEventListener('keydown', this.globalModeToggleHandler, true);
    },

    /**
     * 切换编辑模式（全局）
     */
    toggleGlobalEditMode() {
        // 切换全局模式
        this.globalEditMode = this.globalEditMode === 'block' ? 'text' : 'block';
        window.rLog('切换全局编辑模式为:', this.globalEditMode);

        // 同步到所有打开的编辑器
        this.editors.forEach((editor, tabId) => {
            if (editor.currentMode !== this.globalEditMode) {
                if (this.globalEditMode === 'text') {
                    editor.switchToTextMode();
                } else {
                    editor.switchToBlockMode();
                }
                window.rLog('同步 tab', tabId, '到模式:', this.globalEditMode);
            }
        });
    },

    /**
     * 获取当前全局编辑模式
     * @returns {string} 编辑模式 ('text' 或 'block')
     */
    getGlobalEditMode() {
        return this.globalEditMode;
    }
};

// 导出到全局
window.ModeController = ModeController;

if (window.rLog) {
    window.rLog('✅ ModeController 模块已加载');
}
