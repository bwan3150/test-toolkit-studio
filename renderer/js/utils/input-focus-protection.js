// 输入框焦点保护补丁
// 防止某些输入框在点击时立即失去焦点

(function() {
    console.log('应用输入框焦点保护补丁...');
    
    // 需要保护的输入框选择器
    const protectedInputSelectors = [
        '#inputDialogInput',
        '#imageAliasInput', 
        '#locatorSearchInput',
        '#newNameInput',
        '.editing',
        '[contenteditable="true"]:not(#editorContent)',
        'input[type="text"]',
        'input[type="search"]',
        'textarea:not(#editorTextarea)'
    ];
    
    // 阻止编辑器在这些输入框活动时抢夺焦点
    document.addEventListener('mousedown', (e) => {
        const target = e.target;
        
        // 检查是否点击了受保护的输入框
        const isProtectedInput = protectedInputSelectors.some(selector => {
            return target.matches(selector) || target.closest(selector);
        });
        
        if (isProtectedInput) {
            // 标记正在与受保护的输入交互
            window._protectedInputActive = true;
            
            // 确保编辑器知道有其他输入活动
            if (window.editorInstance) {
                window.editorInstance.isOtherInputFocused = true;
                window.editorInstance.suppressCursorRestore = true;
            }
            
            // 阻止事件冒泡到编辑器
            e.stopPropagation();
            
            console.log('保护输入框焦点:', target);
        }
    }, true);
    
    // 在输入框获得焦点时维持保护状态
    document.addEventListener('focus', (e) => {
        const target = e.target;
        
        const isProtectedInput = protectedInputSelectors.some(selector => {
            return target.matches(selector) || target.closest(selector);
        });
        
        if (isProtectedInput) {
            window._protectedInputActive = true;
            
            if (window.editorInstance) {
                window.editorInstance.isOtherInputFocused = true;
            }
        }
    }, true);
    
    // 在输入框失去焦点时检查是否应该解除保护
    document.addEventListener('blur', (e) => {
        const target = e.target;
        const relatedTarget = e.relatedTarget;
        
        // 延迟检查，确保焦点转移完成
        setTimeout(() => {
            const activeElement = document.activeElement;
            
            // 如果焦点没有转移到另一个受保护的输入框，则解除保护
            const isStillProtected = protectedInputSelectors.some(selector => {
                return activeElement && (activeElement.matches(selector) || activeElement.closest(selector));
            });
            
            if (!isStillProtected) {
                window._protectedInputActive = false;
                
                // 延迟一点再恢复编辑器的焦点控制
                setTimeout(() => {
                    if (!window._protectedInputActive && window.editorInstance) {
                        window.editorInstance.isOtherInputFocused = false;
                        window.editorInstance.suppressCursorRestore = false;
                    }
                }, 100);
            }
        }, 10);
    }, true);
    
    // 特别处理内联编辑（重命名）
    const originalStartInlineEdit = window.startInlineEdit;
    if (originalStartInlineEdit) {
        window.startInlineEdit = function(...args) {
            // 设置保护状态
            window._protectedInputActive = true;
            if (window.editorInstance) {
                window.editorInstance.isOtherInputFocused = true;
                window.editorInstance.suppressCursorRestore = true;
            }
            
            // 调用原始函数
            return originalStartInlineEdit.apply(this, args);
        };
    }
    
    // 监听内联编辑完成
    const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
            if (mutation.type === 'attributes' && mutation.attributeName === 'contenteditable') {
                const target = mutation.target;
                if (target.contentEditable === 'false' && target.classList.contains('editing')) {
                    // 内联编辑结束
                    setTimeout(() => {
                        window._protectedInputActive = false;
                        if (window.editorInstance) {
                            window.editorInstance.isOtherInputFocused = false;
                            window.editorInstance.suppressCursorRestore = false;
                        }
                    }, 100);
                }
            }
        });
    });
    
    // 开始观察DOM变化
    observer.observe(document.body, {
        attributes: true,
        subtree: true,
        attributeFilter: ['contenteditable']
    });
    
    console.log('输入框焦点保护补丁已应用');
})();
