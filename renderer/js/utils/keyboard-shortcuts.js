// 键盘快捷键模块
function initializeKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Ctrl/Cmd + S 保存当前文件
        if ((e.ctrlKey || e.metaKey) && e.key === 's') {
            e.preventDefault();
            window.EditorModule.saveCurrentFileWithNotification();
        }
        
        // Ctrl/Cmd + W 关闭当前tab
        if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
            e.preventDefault();
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                window.EditorModule.closeTab(activeTab.id);
            }
        }
        
        // Ctrl + Tab 切换到下一个tab
        if (e.ctrlKey && e.key === 'Tab' && !e.shiftKey) {
            e.preventDefault();
            window.EditorModule.switchToNextTab();
        }
        
        // Ctrl + Shift + Tab 切换到上一个tab
        if (e.ctrlKey && e.key === 'Tab' && e.shiftKey) {
            e.preventDefault();
            window.EditorModule.switchToPreviousTab();
        }
    });
}

// 导出函数
window.KeyboardShortcutsModule = {
    initializeKeyboardShortcuts
};