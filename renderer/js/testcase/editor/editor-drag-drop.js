// 拖拽功能模块 - 作为EditorTab的扩展方法
const EditorDragDrop = {
    // 为locator类型的输入框添加拖放支持
    setupLocatorInputDragDrop() {
        window.rLog('设置拖拽监听器...');
        
        // 移除之前的拖拽监听器（如果存在）
        if (this.dragOverHandler) {
            this.blocksContainer.removeEventListener('dragover', this.dragOverHandler);
        }
        if (this.dragLeaveHandler) {
            this.blocksContainer.removeEventListener('dragleave', this.dragLeaveHandler);
        }
        if (this.dropHandler) {
            this.blocksContainer.removeEventListener('drop', this.dropHandler);
        }
        
        // 使用事件委托，在块容器上监听拖拽事件
        this.dragOverHandler = (e) => {
            // 查找拖拽目标
            const dropTarget = this.findDropTarget(e.target);
            if (!dropTarget) return;
            
            e.preventDefault();
            e.stopPropagation();
            e.dataTransfer.dropEffect = 'copy';
            
            // 清除之前的高亮
            this.blocksContainer.querySelectorAll('.drag-over').forEach(el => {
                el.classList.remove('drag-over');
            });
            
            // 添加当前目标的高亮
            dropTarget.classList.add('drag-over');
            window.rLog('拖拽悬停在:', dropTarget, '数据:', dropTarget.dataset);
        };
        
        this.dragLeaveHandler = (e) => {
            const dropTarget = this.findDropTarget(e.target);
            if (!dropTarget) return;
            
            // 检查鼠标是否真的离开了目标区域
            const rect = dropTarget.getBoundingClientRect();
            const isInside = e.clientX >= rect.left && e.clientX <= rect.right && 
                           e.clientY >= rect.top && e.clientY <= rect.bottom;
            
            if (!isInside) {
                dropTarget.classList.remove('drag-over');
            }
        };
        
        this.dropHandler = async (e) => {
            const dropTarget = this.findDropTarget(e.target);
            if (!dropTarget) return;
            
            e.preventDefault();
            e.stopPropagation();
            
            window.rLog('拖拽放置在:', dropTarget);
            
            // 清除高亮
            dropTarget.classList.remove('drag-over');
            
            // 获取参数信息
            const commandIndex = parseInt(dropTarget.dataset.commandIndex);
            const paramName = dropTarget.dataset.param;
            
            if (isNaN(commandIndex) || !paramName) {
                window.rLog('无法获取命令索引或参数名:', { commandIndex, paramName, dataset: dropTarget.dataset });
                return;
            }
            
            // 获取拖拽数据
            const locatorDataStr = e.dataTransfer.getData('application/json');
            const textData = e.dataTransfer.getData('text/plain');
            
            window.rLog(`🔄 块编辑器接收拖拽数据: textData="${textData}", locatorDataStr="${locatorDataStr}"`);
            
            if (textData && this.buffer) {
                // 通过TKE缓冲区更新块编辑器内容
                window.rLog(`🔧 块编辑器拖拽更新: 命令${commandIndex}, 参数${paramName}, 值${textData}`);
                
                // 构造更新后的命令行（临时实现，理想情况下TKE应提供接口）
                const updatedLine = this.constructUpdatedCommandLine(commandIndex, paramName, textData);
                
                if (updatedLine) {
                    // 通过缓冲区更新内容
                    await this.buffer.updateFromBlocks({
                        commandIndex: commandIndex,
                        updatedLine: updatedLine
                    });
                    
                    window.rLog('✅ 块编辑器参数更新完成');
                } else {
                    window.rError('❌ 构造更新命令行失败');
                }
            } else {
                window.rError(`❌ 块编辑器未获取到拖拽数据或缓冲区未初始化，textData: ${textData}, buffer: ${!!this.buffer}`);
            }
        };
        
        // 添加事件监听器
        this.blocksContainer.addEventListener('dragover', this.dragOverHandler);
        this.blocksContainer.addEventListener('dragleave', this.dragLeaveHandler);
        this.blocksContainer.addEventListener('drop', this.dropHandler);
        
        // 统计当前可拖拽目标数量
        const dropTargets = this.getAllDropTargets();
        window.rLog(`已设置拖拽监听器，找到 ${dropTargets.length} 个可拖拽目标:`, dropTargets);
    },

    // 构造更新后的命令行（临时实现）
    constructUpdatedCommandLine(commandIndex, paramName, newValue) {
        if (!this.buffer) return null;
        
        const content = this.buffer.getRawContent();
        const lines = content.split('\n');
        
        let currentCommandIndex = -1;
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i].trim();
            if (this.isCommandLine(line)) {
                currentCommandIndex++;
                if (currentCommandIndex === commandIndex) {
                    // 找到目标命令行，构造更新后的行
                    return this.updateCommandLineParameter(line, paramName, newValue);
                }
            }
        }
        
        return null;
    },
    
    // 更新命令行中的参数（临时实现，应该由TKE提供）
    updateCommandLineParameter(commandLine, paramName, newValue) {
        // 根据TKS语法规范更新命令行
        // 例如: 点击 [{200,400}] -> 点击 [{设置}]
        
        if (paramName === 'target') {
            // 替换目标参数
            if (commandLine.includes('[') && commandLine.includes(']')) {
                // 替换方括号中的内容
                return commandLine.replace(/\[([^\]]*)\]/, `[${newValue}]`);
            } else {
                // 添加参数
                return `${commandLine} [${newValue}]`;
            }
        }
        
        // 其他参数类型的处理...
        return commandLine;
    },
    
    // 判断是否是命令行（与TKE缓冲区中的实现保持一致）
    isCommandLine(line) {
        if (!line || line.startsWith('#') || line.startsWith('用例:') || 
            line.startsWith('脚本名:') || line === '详情:' || line === '步骤:' ||
            line.includes('appPackage:') || line.includes('appActivity:')) {
            return false;
        }
        return true;
    },
    
    // 查找有效的拖拽目标
    findDropTarget(element) {
        let current = element;
        while (current && current !== this.blocksContainer) {
            // 检查是否是element类型的输入框或容器
            if ((current.classList.contains('param-hole') && current.dataset.paramType === 'element') ||
                (current.classList.contains('param-hole-container') && current.dataset.type === 'element') ||
                current.matches('input[data-param-type="element"]')) {
                
                window.rLog('找到拖拽目标:', current, {
                    classList: current.classList.toString(),
                    dataset: current.dataset,
                    tagName: current.tagName
                });
                return current;
            }
            current = current.parentElement;
        }
        window.rLog('未找到拖拽目标，检查的元素:', element);
        return null;
    },
    
    // 获取所有可拖拽目标（用于调试）
    getAllDropTargets() {
        return this.blocksContainer.querySelectorAll(
            'input[data-param-type="element"], .param-hole[data-param-type="element"], .param-hole-container[data-type="element"]'
        );
    }
};

// 导出到全局
window.EditorDragDrop = EditorDragDrop;