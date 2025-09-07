// 字体设置功能模块 - 作为EditorTab的扩展方法
const EditorFontSettings = {
    // 更新字体设置
    updateFontSettings(fontFamily, fontSize) {
        if (this.textContentEl) {
            this.textContentEl.style.fontFamily = fontFamily;
            this.textContentEl.style.fontSize = fontSize + 'px';
        }
        if (this.lineNumbersEl) {
            this.lineNumbersEl.style.fontFamily = fontFamily;
            this.lineNumbersEl.style.fontSize = fontSize + 'px';
        }
        // 更新CSS变量
        const root = document.documentElement;
        if (fontFamily !== 'var(--font-mono)') {
            root.style.setProperty('--font-mono', fontFamily);
        }
        root.style.setProperty('--font-size-editor', fontSize + 'px');
    }
};

// 导出到全局
window.EditorFontSettings = EditorFontSettings;