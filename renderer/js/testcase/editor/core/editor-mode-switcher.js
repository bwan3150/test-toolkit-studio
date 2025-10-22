// 编辑器模式切换器 - 负责文本模式和块模式之间的切换

const EditorModeSwitcher = {
    /**
     * 切换编辑模式（文本 <-> 块）
     */
    toggleMode() {
        // 这个方法保留用于向后兼容，但优先使用全局切换
        if (this.currentMode === 'text') {
            this.switchToBlockMode();
        } else {
            this.switchToTextMode();
        }
    },

    /**
     * 切换到文本模式
     */
    async switchToTextMode() {
        window.rLog('切换到文本模式');

        // 如果当前是块模式，不需要特别保存，因为块模式的修改已经通过TKE自动同步到buffer

        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;

        this.currentMode = 'text';
        this.render();

        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            window.rLog('恢复文本模式高亮:', savedHighlightLine);
            // 延迟一点确保DOM已完全渲染
            setTimeout(() => {
                this.highlightExecutingLine(savedHighlightLine);
            }, 50);
        }
    },

    /**
     * 切换到块模式
     */
    async switchToBlockMode() {
        window.rLog('切换到块编程模式');

        // 如果当前是文本模式，需要先保存文本编辑器的内容到buffer
        if (this.currentMode === 'text' && this.textContentEl) {
            const textContent = this.textContentEl.innerText || '';
            window.rLog('保存文本模式内容到buffer:', textContent.length, '字符', '包含换行:', textContent.includes('\n'));

            // 更新buffer内容，这会触发TKE重新解析
            try {
                await this.buffer.updateFromText(textContent);
                window.rLog('✅ 文本内容已同步到buffer');
            } catch (error) {
                window.rError('❌ 同步文本内容到buffer失败:', error.message);
            }
        }

        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;

        this.currentMode = 'block';
        this.render();

        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            window.rLog('恢复块模式高亮:', savedHighlightLine);
            // 延迟一点确保DOM已完全渲染
            setTimeout(() => {
                this.highlightExecutingLine(savedHighlightLine);
            }, 50);
        }
    }
};

// 导出到全局
window.EditorModeSwitcher = EditorModeSwitcher;

if (window.rLog) {
    window.rLog('✅ EditorModeSwitcher 模块已加载');
}
