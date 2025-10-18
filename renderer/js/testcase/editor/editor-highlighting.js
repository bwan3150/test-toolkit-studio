// 行高亮功能模块 - 作为EditorTab的扩展方法
const EditorHighlighting = {
    // 行高亮功能
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
    },
    
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
    
    // 块模式高亮功能
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
    },
    
    // 获取脚本数据 - 兼容新旧系统
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
            
            originalLines.forEach((line, lineIndex) => {
                const trimmed = line.trim();
                
                // 跳过头部信息
                if (!trimmed || trimmed.startsWith('#') || 
                    trimmed.startsWith('用例:') || trimmed.startsWith('脚本名:') ||
                    trimmed === '详情:' || trimmed === '步骤:' ||
                    trimmed.includes('appPackage:') || trimmed.includes('appActivity:')) {
                    lineToCommandMap.push(null); // 非命令行
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
window.EditorHighlighting = EditorHighlighting;

// 记录模块加载（延迟到 rLog 可用时）
if (window.rLog) {
    window.rLog('✅ EditorHighlighting 模块已加载，包含方法:', Object.keys(EditorHighlighting));
    window.rLog('检查 setTestRunning 方法:', {
        exists: 'setTestRunning' in EditorHighlighting,
        type: typeof EditorHighlighting.setTestRunning
    });
} else {
    // 如果 rLog 还不可用，使用 console.log
    console.log('✅ EditorHighlighting 模块已加载，包含方法:', Object.keys(EditorHighlighting));
    console.log('检查 setTestRunning 方法:', {
        exists: 'setTestRunning' in EditorHighlighting,
        type: typeof EditorHighlighting.setTestRunning
    });
}