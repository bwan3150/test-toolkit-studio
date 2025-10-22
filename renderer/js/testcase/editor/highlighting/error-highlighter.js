// 错误高亮功能模块
const ErrorHighlighter = {
    /**
     * 高亮错误行
     * @param {number} tksOriginalLineNumber - TKS 原始行号
     */
    highlightErrorLine(tksOriginalLineNumber) {
        window.rLog('收到错误高亮请求 - TKS原始行号:', tksOriginalLineNumber, '当前模式:', this.currentMode);

        // 获取脚本数据 - 兼容新旧系统
        const scriptData = this.getScriptData();
        if (!scriptData) {
            window.rError('无法获取脚本数据，跳过错误高亮');
            return;
        }

        // 不清除之前的高亮，直接转换为错误高亮

        if (this.currentMode === 'text' && this.textContentEl) {
            // 文本模式：将TKS原始行号转换为显示行号
            const displayLineNumber = this.calculateDisplayLineNumber(tksOriginalLineNumber);
            window.rLog('文本模式 - 计算错误显示行号:', displayLineNumber);
            if (displayLineNumber > 0) {
                // 先清除执行高亮，然后添加错误高亮
                this.clearExecutionHighlight();
                this.addLineHighlight(displayLineNumber, 'error');
                window.rLog('文本模式高亮错误行:', displayLineNumber, '(TKS原始行号:', tksOriginalLineNumber, ')');
            } else {
                window.rLog('无效的错误显示行号:', displayLineNumber);
            }
        } else if (this.currentMode === 'block') {
            // 块模式：将TKS原始行号转换为命令索引
            if (!scriptData.originalLines || !scriptData.lineToCommandMap) {
                window.rLog('缺少行号映射数据');
                return;
            }

            const originalLineIndex = tksOriginalLineNumber - 1;
            if (originalLineIndex >= 0 && originalLineIndex < scriptData.lineToCommandMap.length) {
                const commandIndex = scriptData.lineToCommandMap[originalLineIndex];
                window.rLog('块模式 - TKS错误行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);

                if (commandIndex !== null) {
                    // 先清除执行高亮，然后添加错误高亮
                    this.clearExecutionHighlight();
                    const blockIndex = commandIndex + 1;
                    this.highlightExecutingBlock(blockIndex, 'error');
                    window.rLog('块模式高亮错误块:', blockIndex, '(命令索引:', commandIndex, ')');
                } else {
                    window.rLog('TKS错误行号不是命令行:', tksOriginalLineNumber);
                }
            } else {
                window.rLog('TKS错误行号超出范围:', tksOriginalLineNumber);
            }
        } else {
            window.rLog('错误高亮条件不满足:', {
                currentMode: this.currentMode,
                hasTextContentEl: !!this.textContentEl,
                hasBlocksContainer: !!this.blocksContainer
            });
        }
    }
};

// 导出到全局
window.ErrorHighlighter = ErrorHighlighter;

if (window.rLog) {
    window.rLog('✅ ErrorHighlighter 模块已加载');
}
