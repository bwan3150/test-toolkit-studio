// 编辑器管理器核心类定义
class EditorManagerCore {
    constructor() {
        this.editors = new Map(); // tabId -> EditorTab实例
        this.activeTabId = null;
        this.tabsContainer = null;
        this.editorContainer = null;
        this.globalEditMode = 'block'; // 全局编辑模式，所有 tab 共享
        this.init();
    }

    /**
     * 初始化编辑器管理器
     */
    init() {
        this.tabsContainer = document.getElementById('editorTabs');
        this.editorContainer = document.getElementById('editorWorkspace');

        if (!this.tabsContainer || !this.editorContainer) {
            window.rError('编辑器容器未找到');
            return;
        }

        // 设置全局快捷键监听器
        this.setupGlobalKeyboardShortcuts();

        window.rLog('编辑器管理器初始化完成');
    }

    /**
     * 销毁管理器
     */
    destroy() {
        // 移除全局快捷键监听器
        if (this.globalModeToggleHandler) {
            document.removeEventListener('keydown', this.globalModeToggleHandler, true);
        }

        this.editors.forEach(editor => editor.destroy());
        this.editors.clear();
        this.activeTabId = null;
    }
}

// 导出到全局
window.EditorManagerCore = EditorManagerCore;

if (window.rLog) {
    window.rLog('✅ EditorManagerCore 模块已加载');
}
