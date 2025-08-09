// 全局输入焦点管理器
// 确保编辑器不会干扰其他输入元素的正常使用

(function() {
    console.log('初始化全局输入焦点管理器...');
    
    // 创建焦点管理器
    window.InputFocusManager = {
        isExternalInputFocused: false,
        currentFocusedInput: null,
        
        // 检查元素是否是外部输入元素
        isExternalInput(element) {
            if (!element) return false;
            
            // 排除编辑器本身
            if (element.id === 'editorContent' || element.closest('#editorContent')) {
                return false;
            }
            
            return (
                // 标准输入元素
                (element.tagName === 'INPUT' || 
                 element.tagName === 'TEXTAREA' || 
                 element.tagName === 'SELECT') ||
                // 可编辑元素
                (element.contentEditable === 'true') ||
                // 特定的输入框
                element.id === 'inputDialogInput' ||
                element.id === 'imageAliasInput' ||
                element.id === 'locatorSearchInput' ||
                element.id === 'newNameInput' ||
                element.id === 'confirmScreenshotBtn' ||
                // 通过类名或容器判断
                element.closest('.modal-dialog') ||
                element.closest('.context-menu') ||
                element.closest('.locator-context-menu') ||
                element.closest('.editing') ||
                element.classList.contains('editing') ||
                element.closest('.form-control') ||
                element.closest('.search-input') ||
                element.closest('.locator-search-bar')
            );
        },
        
        // 设置外部输入焦点状态
        setExternalInputFocus(element) {
            this.isExternalInputFocused = true;
            this.currentFocusedInput = element;
            
            // 通知编辑器
            if (window.editorInstance) {
                window.editorInstance.isOtherInputFocused = true;
            }
            
            console.log('外部输入元素获得焦点:', element);
        },
        
        // 清除外部输入焦点状态
        clearExternalInputFocus() {
            this.isExternalInputFocused = false;
            this.currentFocusedInput = null;
            
            // 通知编辑器
            if (window.editorInstance) {
                window.editorInstance.isOtherInputFocused = false;
            }
        },
        
        // 初始化监听器
        init() {
            // 全局focusin事件
            document.addEventListener('focusin', (e) => {
                const target = e.target;
                
                if (this.isExternalInput(target)) {
                    this.setExternalInputFocus(target);
                } else if (target.id === 'editorContent' || target.closest('#editorContent')) {
                    // 只有当没有外部输入正在编辑时才清除状态
                    if (!this.isActivelyEditing()) {
                        this.clearExternalInputFocus();
                    }
                }
            }, true);
            
            // 全局focusout事件
            document.addEventListener('focusout', (e) => {
                const target = e.target;
                const relatedTarget = e.relatedTarget;
                
                // 延迟检查，确保焦点转移完成
                setTimeout(() => {
                    const activeElement = document.activeElement;
                    
                    // 如果焦点转移到了编辑器，但之前的元素是外部输入
                    if (this.currentFocusedInput === target && 
                        (activeElement === document.getElementById('editorContent') || 
                         (activeElement && activeElement.closest('#editorContent')))) {
                        
                        // 检查是否应该保持外部输入的焦点
                        if (this.shouldKeepFocus(target)) {
                            e.preventDefault();
                            target.focus();
                            this.setExternalInputFocus(target);
                        }
                    }
                }, 50);
            }, true);
            
            // 监听点击事件，防止编辑器抢夺焦点
            document.addEventListener('mousedown', (e) => {
                const target = e.target;
                
                if (this.isExternalInput(target)) {
                    // 暂时禁用编辑器的焦点恢复
                    if (window.editorInstance) {
                        window.editorInstance.suppressCursorRestore = true;
                        setTimeout(() => {
                            window.editorInstance.suppressCursorRestore = false;
                        }, 200);
                    }
                }
            }, true);
        },
        
        // 检查是否正在活动编辑
        isActivelyEditing() {
            return this.currentFocusedInput && 
                   (this.currentFocusedInput.classList.contains('editing') ||
                    this.currentFocusedInput.closest('.editing') ||
                    this.currentFocusedInput.closest('.modal-dialog'));
        },
        
        // 检查是否应该保持焦点
        shouldKeepFocus(element) {
            return element && (
                element.classList.contains('editing') ||
                element.closest('.editing') ||
                element.closest('.modal-dialog') ||
                element.closest('.context-menu') ||
                element.id === 'locatorSearchInput'
            );
        }
    };
    
    // 初始化
    InputFocusManager.init();
    
    console.log('全局输入焦点管理器初始化完成');
})();
