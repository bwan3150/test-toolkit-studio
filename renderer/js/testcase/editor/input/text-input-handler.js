// 文本输入处理器 - 负责文本模式下的用户交互事件

const TextInputHandler = {
    /**
     * 设置文本模式的事件监听器
     */
    setupTextModeListeners() {
        // 获取文本内容元素
        this.textContentEl = this.editorContainer.querySelector('.text-content');
        if (!this.textContentEl) {
            window.rLog('textContent元素未找到');
            return;
        }

        // 输入法组合状态标志
        let isComposing = false;

        // 输入法开始组合
        this.textContentEl.addEventListener('compositionstart', () => {
            isComposing = true;
        });

        // 输入法结束组合
        this.textContentEl.addEventListener('compositionend', () => {
            isComposing = false;
            // 组合结束后立即触发一次更新
            this.updateEditorHighlight();
        });

        this.textContentEl.addEventListener('input', () => {
            if (this.isTestRunning) return;

            // 如果正在输入法组合中，不更新高亮
            if (isComposing) {
                return;
            }

            this.updateEditorHighlight();
        });
    },

    /**
     * 更新编辑器高亮（提取为独立方法）
     */
    updateEditorHighlight() {
        if (!this.textContentEl) return;

        // 1. 保存光标位置
        const cursorPosition = window.EditorCursor.saveCursorPosition(this.textContentEl);

        // 2. 获取纯文本内容
        const tksCode = window.EditorCursor.getPlainText(this.textContentEl);
        window.rLog(`文本编辑器输入事件，内容长度: ${tksCode.length}，包含换行: ${tksCode.includes('\n')}`);

        // 3. 重新渲染高亮 HTML
        const highlightedHTML = this.highlightTKSSyntax(tksCode);
        this.textContentEl.innerHTML = highlightedHTML;

        // 4. 恢复光标位置
        window.EditorCursor.restoreCursorPosition(this.textContentEl, cursorPosition);

        // 5. 更新行号和触发变化
        this.updateLineNumbers();
        this.triggerChange();
    }
};

// 导出到全局
window.TextInputHandler = TextInputHandler;

if (window.rLog) {
    window.rLog('✅ TextInputHandler 模块已加载');
}
