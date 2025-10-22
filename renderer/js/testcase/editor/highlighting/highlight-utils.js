// 高亮工具函数模块
const HighlightUtils = {
    /**
     * 添加行高亮
     * @param {number} lineNumber - 行号
     * @param {string} type - 高亮类型 ('executing' 或 'error')
     */
    addLineHighlight(lineNumber, type) {
        window.rLog('添加行高亮:', lineNumber, type);

        // 高亮行号
        if (this.lineNumbersEl) {
            const lineNumbers = this.lineNumbersEl.querySelectorAll('.line-number');
            window.rLog('行号元素数量:', lineNumbers.length, '目标行号:', lineNumber - 1);
            if (lineNumbers[lineNumber - 1]) {
                lineNumbers[lineNumber - 1].classList.add('highlighted', type);
                window.rLog('行号高亮已添加');
            } else {
                window.rLog('找不到行号元素:', lineNumber - 1);
            }
        } else {
            window.rLog('lineNumbersEl不存在');
        }

        // 在文本内容上添加高亮背景
        if (!this.textContentEl) {
            window.rLog('textContentEl不存在，无法添加高亮');
            return;
        }

        // 确保容器有正确的定位
        const textWrapper = this.textContentEl.parentElement;
        if (textWrapper && !textWrapper.style.position) {
            textWrapper.style.position = 'relative';
        }

        const lineHeight = 21; // 与CSS中的line-height保持一致
        const topOffset = 16; // 与padding-top保持一致

        const highlightDiv = document.createElement('div');
        highlightDiv.className = `line-highlight ${type}`;
        highlightDiv.style.cssText = `
            position: absolute !important;
            left: 0 !important;
            right: 0 !important;
            height: ${lineHeight}px !important;
            top: ${topOffset + (lineNumber - 1) * lineHeight}px !important;
            background: ${type === 'executing' ? 'rgba(255, 193, 7, 0.3)' : 'rgba(220, 53, 69, 0.3)'} !important;
            border-left: 3px solid ${type === 'executing' ? '#ffc107' : '#dc3545'} !important;
            pointer-events: none !important;
            z-index: 10 !important;
            margin: 0 !important;
            padding: 0 !important;
        `;

        // 添加到文本编辑器的包装器而不是内容元素
        const targetContainer = textWrapper || this.textContentEl;
        targetContainer.appendChild(highlightDiv);

        // 滚动到高亮行
        setTimeout(() => {
            const lineNumberEl = this.lineNumbersEl?.querySelectorAll('.line-number')[lineNumber - 1];
            if (lineNumberEl) {
                lineNumberEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        }, 100);

        window.rLog('文本高亮已添加到:', targetContainer.className);
        window.rLog('高亮div样式:', highlightDiv.style.cssText);
        window.rLog('高亮div实际父元素:', highlightDiv.parentElement);
    },

    /**
     * 获取脚本数据 - 兼容新旧系统
     * @returns {Object|null} 脚本数据对象 {originalLines, lineToCommandMap}
     */
    getScriptData() {
        // 新系统：使用TKE buffer + 从内容生成行映射
        if (this.buffer) {
            const rawContent = this.buffer.getRawContent();
            if (!rawContent) {
                window.rLog('Buffer中无原始内容');
                return null;
            }

            // 从原始内容生成行映射数据
            const originalLines = rawContent.split('\n');
            const lineToCommandMap = [];
            let commandIndex = 0;
            let inStepsSection = false; // 标记是否进入步骤部分

            originalLines.forEach((line, lineIndex) => {
                const trimmed = line.trim();

                // 检测步骤部分
                if (trimmed === '步骤:') {
                    inStepsSection = true;
                    lineToCommandMap.push(null);
                    return;
                }

                // 在步骤部分之前,全部跳过
                if (!inStepsSection) {
                    lineToCommandMap.push(null);
                    return;
                }

                // 跳过空行和注释
                if (!trimmed || trimmed.startsWith('#')) {
                    lineToCommandMap.push(null);
                    return;
                }

                // 这是一个命令行
                lineToCommandMap.push(commandIndex);
                commandIndex++;
            });

            return {
                originalLines,
                lineToCommandMap
            };
        }

        // 旧系统：使用script对象
        if (this.script && this.script.originalLines && this.script.lineToCommandMap) {
            return {
                originalLines: this.script.originalLines,
                lineToCommandMap: this.script.lineToCommandMap
            };
        }

        window.rError('无法从buffer或script获取脚本数据');
        return null;
    }
};

// 导出到全局
window.HighlightUtils = HighlightUtils;

if (window.rLog) {
    window.rLog('✅ HighlightUtils 模块已加载');
}
