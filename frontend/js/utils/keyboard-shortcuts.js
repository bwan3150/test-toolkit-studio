// 键盘快捷键模块
function initializeKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Ctrl/Cmd + S 保存当前文件
        if ((e.ctrlKey || e.metaKey) && e.key === 's') {
            e.preventDefault();
            window.EditorManager.saveCurrentFileWithNotification();
        }
        
        // Ctrl/Cmd + W 关闭当前tab
        if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
            e.preventDefault();
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                window.EditorManager.closeTab(activeTab.id);
            }
        }
        
        // Ctrl + Tab 切换到下一个tab
        if (e.ctrlKey && e.key === 'Tab' && !e.shiftKey) {
            e.preventDefault();
            window.EditorManager.switchToNextTab();
        }
        
        // Ctrl + Shift + Tab 切换到上一个tab
        if (e.ctrlKey && e.key === 'Tab' && e.shiftKey) {
            e.preventDefault();
            window.EditorManager.switchToPreviousTab();
        }

        // Ctrl/Cmd + / 切换编辑模式(块模式/文本模式)
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
            e.preventDefault();
            const editor = window.EditorManager.getCurrentEditor();
            if (editor && editor.toggleMode) {
                editor.toggleMode();
            }
        }
    });
}

// 导出函数
window.KeyboardShortcutsModule = {
    initializeKeyboardShortcuts
};