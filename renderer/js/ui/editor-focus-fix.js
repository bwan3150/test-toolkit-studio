// 编辑器焦点修复补丁
// 解决编辑器与其他输入元素的焦点冲突问题

(function() {
    console.log('应用编辑器焦点修复补丁...');
    
    // 等待编辑器模块加载
    const waitForEditor = setInterval(() => {
        if (window.SimpleCodeEditor && window.editorInstance) {
            clearInterval(waitForEditor);
            applyFocusFix();
        }
    }, 100);
    
    function applyFocusFix() {
        // 给 SimpleCodeEditor 原型添加焦点控制方法
        if (!SimpleCodeEditor.prototype._originalRestoreCursorPosition) {
            // 保存原始方法
            SimpleCodeEditor.prototype._originalRestoreCursorPosition = SimpleCodeEditor.prototype.restoreCursorPosition;
            SimpleCodeEditor.prototype._originalApplySyntaxHighlighting = SimpleCodeEditor.prototype.applySyntaxHighlighting;
        }
        
        // 添加焦点控制标志
        SimpleCodeEditor.prototype.isOtherInputFocused = false;
        SimpleCodeEditor.prototype.focusControlEnabled = true;
        
        // 重写 restoreCursorPosition 方法
        SimpleCodeEditor.prototype.restoreCursorPosition = function(offset) {
            // 如果其他输入元素有焦点，不恢复编辑器光标
            if (this.isOtherInputFocused || !this.focusControlEnabled) {
                return;
            }
            
            // 调用原始方法
            this._originalRestoreCursorPosition.call(this, offset);
        };
        
        // 重写 applySyntaxHighlighting 方法
        SimpleCodeEditor.prototype.applySyntaxHighlighting = function() {
            // 如果正在组合输入，不进行语法高亮
            if (this.isComposing) {
                return;
            }
            
            if (!this.value) {
                this.showPlaceholder();
                return;
            }
            
            this.hidePlaceholder();
            
            // 应用语法高亮到ContentEditable
            const highlightedHtml = this.highlightSyntax(this.value);
            
            // 保存当前光标位置（仅在非测试运行时且没有其他输入焦点时）
            let cursorOffset = 0;
            let shouldRestoreCursor = !this.isTestRunning && 
                                    !this.suppressCursorRestore && 
                                    !this.isOtherInputFocused;
            
            if (shouldRestoreCursor) {
                const selection = window.getSelection();
                const range = selection.rangeCount > 0 ? selection.getRangeAt(0) : null;
                cursorOffset = range ? this.getTextOffset(range.startContainer, range.startOffset) : 0;
            }
            
            // 设置高亮内容
            this.contentEl.innerHTML = highlightedHtml;
            
            // 恢复光标位置（仅在应该恢复时）
            if (shouldRestoreCursor && cursorOffset > 0) {
                // 使用setTimeout确保DOM更新完成
                setTimeout(() => {
                    if (!this.isOtherInputFocused) {
                        this.restoreCursorPosition(cursorOffset);
                    }
                }, 0);
            }
        };
        
        // 监听全局焦点事件
        let lastFocusedElement = null;
        
        document.addEventListener('focusin', (e) => {
            const target = e.target;
            
            // 检查是否是编辑器外的输入元素
            const isExternalInput = (
                // 输入框和文本域
                (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') &&
                !target.closest('#editorContent') ||
                // 可编辑的元素（用于内联编辑）
                (target.contentEditable === 'true' && !target.id?.includes('editorContent')) ||
                // 特定的输入元素
                target.id === 'inputDialogInput' ||
                target.id === 'imageAliasInput' ||
                target.closest('.context-menu-item') ||
                target.closest('.modal-dialog') ||
                target.closest('.search-input') ||
                target.closest('.editing') // 内联编辑状态
            );
            
            if (isExternalInput) {
                // 标记其他输入元素获得焦点
                if (window.editorInstance) {
                    window.editorInstance.isOtherInputFocused = true;
                    console.log('其他输入元素获得焦点:', target);
                }
                lastFocusedElement = target;
            } else if (target.id === 'editorContent' || target.closest('#editorContent')) {
                // 编辑器获得焦点
                if (window.editorInstance) {
                    window.editorInstance.isOtherInputFocused = false;
                }
            }
        }, true);
        
        // 监听失焦事件
        document.addEventListener('focusout', (e) => {
            const target = e.target;
            
            // 延迟检查，因为焦点可能正在转移
            setTimeout(() => {
                const activeElement = document.activeElement;
                
                // 如果焦点转移到了编辑器，且之前是外部输入元素
                if ((activeElement.id === 'editorContent' || activeElement.closest('#editorContent')) &&
                    lastFocusedElement && lastFocusedElement === target) {
                    // 检查是否应该保持外部输入的焦点
                    if (target.contentEditable === 'true' || 
                        target.classList.contains('editing') ||
                        target.closest('.modal-dialog') ||
                        target.closest('.context-menu')) {
                        // 阻止编辑器获取焦点，返回焦点给原元素
                        e.preventDefault();
                        e.stopPropagation();
                        target.focus();
                        
                        if (window.editorInstance) {
                            window.editorInstance.isOtherInputFocused = true;
                        }
                    }
                }
            }, 10);
        }, true);
        
        // 添加全局方法以便调试
        window.editorFocusControl = {
            disable: () => {
                if (window.editorInstance) {
                    window.editorInstance.focusControlEnabled = false;
                    window.editorInstance.isOtherInputFocused = true;
                }
            },
            enable: () => {
                if (window.editorInstance) {
                    window.editorInstance.focusControlEnabled = true;
                    window.editorInstance.isOtherInputFocused = false;
                }
            },
            status: () => {
                if (window.editorInstance) {
                    return {
                        focusControlEnabled: window.editorInstance.focusControlEnabled,
                        isOtherInputFocused: window.editorInstance.isOtherInputFocused
                    };
                }
                return null;
            }
        };
        
        console.log('编辑器焦点修复补丁已应用');
    }
})();
