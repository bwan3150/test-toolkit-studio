// 执行高亮功能模块
const ExecutionHighlighter = {
    /**
     * 高亮正在执行的行
     * @param {number} tksOriginalLineNumber - TKS 原始行号
     */
    highlightExecutingLine(tksOriginalLineNumber) {
        window.rLog('=== 高亮请求开始 ===');
        window.rLog('TKS原始行号:', tksOriginalLineNumber);
        window.rLog('当前模式:', this.currentMode);
        window.rLog('上一次高亮行号:', this.currentHighlightedLine);
        window.rLog('测试运行状态:', this.isTestRunning);

        // 获取脚本数据 - 兼容新旧系统
        const scriptData = this.getScriptData();
        if (!scriptData) {
            window.rError('无法获取脚本数据，跳过高亮');
            return;
        }

        // 先检查这是否是一个有效的命令行
        let isValidCommandLine = false;
        if (scriptData.originalLines && scriptData.lineToCommandMap) {
            const originalLineIndex = tksOriginalLineNumber - 1;
            if (originalLineIndex >= 0 && originalLineIndex < scriptData.lineToCommandMap.length) {
                isValidCommandLine = scriptData.lineToCommandMap[originalLineIndex] !== null;
            }
        }

        if (!isValidCommandLine) {
            window.rLog('✓ 非命令行请求，保持当前高亮连续性');
            const originalLineIndex = tksOriginalLineNumber - 1;
            window.rLog('行内容:', scriptData.originalLines ? `"${scriptData.originalLines[originalLineIndex]}"` : '无法获取');
            window.rLog('=== 高亮请求结束（非命令行，保持当前高亮）===');
            return;
        }

        // 只有在切换到不同的有效命令行时才清除之前的高亮
        if (this.currentHighlightedLine !== tksOriginalLineNumber) {
            window.rLog('✓ 切换到新的命令行，清除之前的高亮');
            window.rLog('从行', this.currentHighlightedLine, '切换到行', tksOriginalLineNumber);
            this.clearExecutionHighlight();
            this.currentHighlightedLine = tksOriginalLineNumber;
        } else {
            window.rLog('✓ 同一命令行重复高亮请求，检查高亮是否仍然存在');

            // 检查当前高亮是否仍然存在
            let highlightExists = false;
            if (this.currentMode === 'text') {
                const existingHighlights = this.container.querySelectorAll('.line-highlight.executing');
                highlightExists = existingHighlights.length > 0;
                window.rLog('文本模式 - 现有执行高亮数量:', existingHighlights.length);
            } else if (this.currentMode === 'block') {
                const existingBlockHighlights = this.container.querySelectorAll('.workspace-block.highlighted.executing');
                highlightExists = existingBlockHighlights.length > 0;
                window.rLog('块模式 - 现有执行高亮数量:', existingBlockHighlights.length);
            }

            if (highlightExists) {
                window.rLog('✓ 高亮仍然存在，跳过重新创建');
                window.rLog('=== 高亮请求结束（跳过）===');
                return;
            } else {
                window.rLog('✗ 高亮丢失，重新创建');
            }
        }

        if (this.currentMode === 'text' && this.textContentEl) {
            // 文本模式：将TKS原始行号转换为显示行号
            const displayLineNumber = this.calculateDisplayLineNumber(tksOriginalLineNumber);
            window.rLog('文本模式 - 计算显示行号:', displayLineNumber);
            if (displayLineNumber > 0) {
                this.addLineHighlight(displayLineNumber, 'executing');
                window.rLog('✓ 文本模式高亮已创建 - 显示行号:', displayLineNumber, '(TKS原始行号:', tksOriginalLineNumber, ')');
                window.rLog('=== 高亮请求结束（文本模式完成）===');
            } else {
                window.rError('✗ 有效命令行但显示行号计算失败:', displayLineNumber);
                window.rLog('=== 高亮请求结束（文本模式失败）===');
            }
        } else if (this.currentMode === 'block') {
            // 块模式：将TKS原始行号转换为命令索引（已确认是有效命令行）
            const originalLineIndex = tksOriginalLineNumber - 1;
            const commandIndex = scriptData.lineToCommandMap[originalLineIndex];
            window.rLog('块模式 - TKS行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);

            // 命令索引转换为1基索引进行高亮
            const blockIndex = commandIndex + 1;
            this.highlightExecutingBlock(blockIndex, 'executing');
            window.rLog('✓ 块模式高亮已创建 - 块索引:', blockIndex, '(命令索引:', commandIndex, ')');
            window.rLog('=== 高亮请求结束（块模式完成）===');
        } else {
            window.rLog('高亮条件不满足:', {
                currentMode: this.currentMode,
                hasTextContentEl: !!this.textContentEl,
                hasBlocksContainer: !!this.blocksContainer
            });
        }
    },

    /**
     * 清除所有执行高亮
     */
    clearExecutionHighlight() {
        window.rLog('清除执行高亮');
        this.currentHighlightedLine = null; // 重置当前高亮行号

        // 文本模式：清除行高亮
        if (this.currentMode === 'text') {
            // 从文本编辑器容器和内容元素中移除所有行高亮
            const containers = [this.textContentEl, this.textContentEl?.parentElement, this.editorContainer];
            containers.forEach(container => {
                if (container) {
                    const existingHighlights = container.querySelectorAll('.line-highlight');
                    window.rLog(`从 ${container.className} 中移除 ${existingHighlights.length} 个高亮元素`);
                    existingHighlights.forEach(highlight => highlight.remove());
                }
            });

            // 移除行号高亮
            if (this.lineNumbersEl) {
                const highlightedLineNumbers = this.lineNumbersEl.querySelectorAll('.line-number.highlighted');
                highlightedLineNumbers.forEach(lineNum => {
                    lineNum.classList.remove('highlighted', 'executing', 'error');
                });
                window.rLog(`移除了 ${highlightedLineNumbers.length} 个行号高亮`);
            }
        }

        // 块模式：清除块高亮
        if (this.currentMode === 'block' && this.blocksContainer) {
            const highlightedBlocks = this.blocksContainer.querySelectorAll('.workspace-block.highlighted');
            highlightedBlocks.forEach(block => {
                block.classList.remove('highlighted', 'executing', 'error');
                block.style.boxShadow = '';
                block.style.transform = '';
                block.style.animation = '';

                // 移除高亮覆盖层
                const overlays = block.querySelectorAll('.block-highlight-overlay');
                overlays.forEach(overlay => overlay.remove());
            });
            window.rLog(`移除了 ${highlightedBlocks.length} 个块高亮`);
        }

        window.rLog('已清除所有高亮');
    },

    /**
     * 高亮执行中的块
     * @param {number} commandIndex - 命令索引（1-based）
     * @param {string} type - 高亮类型 ('executing' 或 'error')
     */
    highlightExecutingBlock(commandIndex, type) {
        window.rLog('块模式高亮请求:', commandIndex, type, '容器存在:', !!this.blocksContainer);

        if (!this.blocksContainer) {
            window.rLog('blocksContainer不存在');
            return;
        }

        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        window.rLog('找到块数量:', blocks.length, '目标索引:', commandIndex - 1);

        if (commandIndex >= 1 && commandIndex <= blocks.length) {
            const targetBlock = blocks[commandIndex - 1]; // 转换为0基索引
            if (targetBlock) {
                // 添加高亮类
                targetBlock.classList.add('highlighted', type);

                // 创建高亮覆盖层，而不是直接修改块的样式
                const highlightOverlay = document.createElement('div');
                highlightOverlay.className = `block-highlight-overlay ${type}`;
                highlightOverlay.style.cssText = `
                    position: absolute !important;
                    top: -2px !important;
                    left: -2px !important;
                    right: -2px !important;
                    bottom: -2px !important;
                    border-radius: 10px !important;
                    pointer-events: none !important;
                    z-index: 5 !important;
                    transition: all 0.2s ease !important;
                `;

                if (type === 'executing') {
                    highlightOverlay.style.cssText += `
                        border: 3px solid #ffc107 !important;
                        background: rgba(255, 193, 7, 0.1) !important;
                        box-shadow: 0 0 12px rgba(255, 193, 7, 0.4), inset 0 0 12px rgba(255, 193, 7, 0.1) !important;
                    `;
                } else if (type === 'error') {
                    highlightOverlay.style.cssText += `
                        border: 3px solid #dc3545 !important;
                        background: rgba(220, 53, 69, 0.1) !important;
                        box-shadow: 0 0 12px rgba(220, 53, 69, 0.4), inset 0 0 12px rgba(220, 53, 69, 0.1) !important;
                    `;
                }

                // 确保目标块有相对定位
                if (!targetBlock.style.position) {
                    targetBlock.style.position = 'relative';
                }

                // 添加覆盖层到块中
                targetBlock.appendChild(highlightOverlay);

                // 添加脉搏效果
                if (type === 'executing') {
                    targetBlock.style.animation = 'pulse-executing 2s infinite';
                }

                // 滚动到可见区域
                setTimeout(() => {
                    targetBlock.scrollIntoView({ behavior: 'smooth', block: 'center' });
                }, 100);

                window.rLog('块模式高亮已添加:', commandIndex, type, '覆盖层已添加到块中');
            } else {
                window.rLog('找不到目标块:', commandIndex - 1);
            }
        } else {
            window.rLog('无效的块索引:', commandIndex, '有效范围: 1-' + blocks.length);
        }
    },

    /**
     * 设置测试运行状态
     * @param {boolean} isRunning - 是否正在运行
     * @param {boolean} clearHighlight - 是否清除高亮
     */
    setTestRunning(isRunning, clearHighlight = false) {
        window.rLog('设置测试运行状态:', isRunning, '清除高亮:', clearHighlight, '当前模式:', this.currentMode);
        this.isTestRunning = isRunning;

        // 只有在明确要求清除高亮时才清除（成功完成时）
        if (!isRunning && clearHighlight) {
            window.rLog('测试成功结束，清除所有高亮');
            this.clearExecutionHighlight();
        } else if (!isRunning) {
            window.rLog('测试结束但保持错误高亮（如果有）');
        }

        // 更新状态指示器
        this.updateStatusIndicator();

        // 在文本模式下禁用/启用编辑
        if (this.currentMode === 'text' && this.textContentEl) {
            this.textContentEl.contentEditable = !isRunning;
            this.textContentEl.style.opacity = isRunning ? '0.7' : '1';
            this.textContentEl.style.cursor = isRunning ? 'not-allowed' : 'text';
        }
    }
};

// 导出到全局
window.ExecutionHighlighter = ExecutionHighlighter;

if (window.rLog) {
    window.rLog('✅ ExecutionHighlighter 模块已加载');
}
