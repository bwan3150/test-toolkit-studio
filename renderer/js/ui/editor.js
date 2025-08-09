// 新编辑器模块 - 基于ContentEditable的单层实现

class SimpleCodeEditor {
    constructor(container) {
        this.container = container;
        this.value = '';
        this.listeners = [];
        this.saveTimeout = null;
        this.isComposing = false; // 是否在输入法组合输入中
        this.isTestRunning = false; // 测试运行状态
        this.suppressCursorRestore = false; // 抑制光标恢复，用于防止异常跳转
        this.forceTextMode = false; // 强制文本模式标志
        this.isOtherInputFocused = false; // 其他输入元素是否有焦点
        
        this.createEditor();
        this.setupEventListeners();
        this.setupFocusTracking(); // 设置焦点跟踪
    }
    
    createEditor() {
        this.container.innerHTML = `
            <div class="editor-wrapper-new">
                <div class="editor-gutter">
                    <div class="line-numbers-new" id="lineNumbersNew"></div>
                </div>
                <div class="editor-main">
                    <div class="editor-content" id="editorContent" contenteditable="true"></div>
                    <div class="editor-status-indicator" id="editorStatusIndicator"></div>
                </div>
            </div>
        `;
        
        this.lineNumbersEl = document.getElementById('lineNumbersNew');
        this.contentEl = document.getElementById('editorContent');
        this.statusIndicatorEl = document.getElementById('editorStatusIndicator');
        
        
        // 设置ContentEditable样式 - 使用应用统一的等宽字体
        const fontSettings = {
            fontFamily: 'var(--font-mono)', // 使用应用统一的等宽字体变量
            fontSize: '14px',
            lineHeight: '21px',
            letterSpacing: 'normal',
            wordSpacing: 'normal',
            fontVariantNumeric: 'normal',
            fontFeatureSettings: 'normal',
            fontKerning: 'none'
        };
        
        Object.assign(this.contentEl.style, {
            ...fontSettings,
            padding: '16px',
            margin: '0',
            border: 'none',
            outline: 'none',
            whiteSpace: 'pre-wrap',
            overflowWrap: 'break-word',
            wordBreak: 'break-word',
            color: '#d4d4d4',
            background: 'transparent',
            caretColor: '#ffffff',
            width: '100%',
            height: '100%',
            minHeight: '100%',
            textRendering: 'optimizeSpeed',
            fontSmooth: 'never',
            webkitFontSmoothing: 'none',
            overflow: 'auto'
        });
        
        this.setPlaceholder('在Project页面选择测试项并创建Case后, 在左侧文件树点击对应Case下的.tks自动化脚本开始编辑');
        
        // 初始化状态指示器
        this.updateStatusIndicator();
    }
    
    setupEventListeners() {
        // 处理输入法组合输入
        this.contentEl.addEventListener('compositionstart', () => {
            this.isComposing = true;
        });
        
        this.contentEl.addEventListener('compositionend', () => {
            this.isComposing = false;
            // 组合输入结束后延迟更新，确保输入内容已经正确插入DOM
            setTimeout(() => {
                this.updateValue();
                this.updateLineNumbers();
                this.applySyntaxHighlighting();
                this.triggerChange();
            }, 10);
        });
        
        // 处理输入事件
        this.contentEl.addEventListener('input', (e) => {
            // 如果测试正在运行，阻止编辑
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            // 输入时隐藏预览
            this.hideImagePreview();
            
            // 如果在组合输入中，跳过更新
            if (this.isComposing) return;
            
            this.updateValue();
            this.updateLineNumbers();
            this.applySyntaxHighlighting();
            this.triggerChange();
        });
        
        // 处理按键事件
        this.contentEl.addEventListener('keydown', (e) => {
            // 如果测试正在运行，阻止所有按键操作
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            if (e.key === 'Tab') {
                e.preventDefault();
                this.insertText('  '); // 插入2个空格
            } else if (e.key === 'Enter') {
                e.preventDefault();
                this.insertText('\n'); // 插入换行符
            } else if (e.key === 'Backspace' || e.key === 'Delete') {
                // 特殊处理删除键，防止光标跳转
                this.handleDeleteKey(e);
            } else if (e.key === '/' && (e.metaKey || e.ctrlKey)) {
                // Cmd/Ctrl + / : 切换所有图片为文本格式
                e.preventDefault();
                this.toggleAllImagesToText();
            }
        });
        
        // 添加beforeinput事件处理，提供更好的删除控制
        this.contentEl.addEventListener('beforeinput', (e) => {
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            // 对于删除类型的输入，进行特殊处理
            if (e.inputType === 'deleteContentBackward' || 
                e.inputType === 'deleteContentForward') {
                this.handleDeleteInput(e);
            }
        });
        
        // 处理滚动同步
        this.contentEl.addEventListener('scroll', () => {
            if (this.lineNumbersEl) {
                this.lineNumbersEl.scrollTop = this.contentEl.scrollTop;
            }
        });
        
        // 处理图片定位器的点击
        this.contentEl.addEventListener('click', (e) => {
            // 如果测试正在运行，阻止图片点击
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            if (e.target.classList.contains('inline-image-locator')) {
                e.preventDefault();
                // 点击时立即隐藏预览
                this.hideImagePreview();
                this.handleImageLocatorClick(e.target);
            }
        });
        
        // 处理图片定位器的hover预览
        this.contentEl.addEventListener('mouseover', (e) => {
            if (e.target.classList.contains('inline-image-locator')) {
                // 立即显示预览，提高效率
                this.showImagePreview(e.target, e);
            }
        });
        
        this.contentEl.addEventListener('mouseout', (e) => {
            if (e.target.classList.contains('inline-image-locator')) {
                this.hideImagePreview();
            }
        });
        
        // 监听光标位置变化，用于图片/文本模式切换
        this.contentEl.addEventListener('keyup', (e) => {
            // 如果测试正在运行，不处理光标变化
            if (this.isTestRunning) return;
            
            // 键盘操作时隐藏预览
            this.hideImagePreview();
            
            // 只在可能影响定位器显示的按键后检查光标位置
            if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || 
                e.key === 'Home' || e.key === 'End' || 
                e.key.startsWith('Arrow')) {
                setTimeout(() => this.checkImageLocatorCursor(), 100);
            }
        });
        
        this.contentEl.addEventListener('mouseup', () => {
            // 如果测试正在运行，不处理光标变化
            if (this.isTestRunning) return;
            
            // 鼠标点击后稍微延迟检查，给用户时间完成操作
            // 添加额外检查确保不在测试即将开始的状态下触发
            setTimeout(() => {
                if (!this.isTestRunning) {
                    this.checkImageLocatorCursor();
                }
            }, 150);
        });
        
        // 监听失去焦点，恢复所有图片显示
        this.contentEl.addEventListener('blur', () => {
            this.hideImagePreview(); // 失焦时隐藏预览
            setTimeout(() => this.restoreAllImages(), 100);
        });
    }
    
    // 设置焦点跟踪
    setupFocusTracking() {
        // 监听全局焦点事件
        document.addEventListener('focusin', (e) => {
            const target = e.target;
            
            // 检查是否是编辑器外的输入元素
            const isExternalInput = (
                // 输入框和文本域
                (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') &&
                !target.closest('#editorContent') ||
                // 可编辑的元素（用于内联编辑）
                (target.contentEditable === 'true' && target.id !== 'editorContent' && !target.closest('#editorContent')) ||
                // 特定的输入元素
                target.id === 'inputDialogInput' ||
                target.id === 'imageAliasInput' ||
                target.id === 'locatorSearchInput' || // Locator库搜索框
                target.closest('.context-menu-item') ||
                target.closest('.modal-dialog') ||
                target.closest('.search-input') ||
                target.closest('.editing') || // 内联编辑状态
                target.classList.contains('editing') || // 直接检查editing类
                target.closest('.locator-search-bar') // Locator搜索栏
            );
            
            if (isExternalInput) {
                // 标记其他输入元素获得焦点
                this.isOtherInputFocused = true;
                console.log('其他输入元素获得焦点:', target);
            } else if (target === this.contentEl || target.closest('#editorContent')) {
                // 编辑器获得焦点
                this.isOtherInputFocused = false;
            }
        }, true);
        
        // 监听失焦事件
        document.addEventListener('focusout', (e) => {
            const target = e.target;
            const relatedTarget = e.relatedTarget;
            
            // 如果焦点从外部输入移到编辑器，保持标志
            if (target && (target.contentEditable === 'true' || target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
                if (relatedTarget === this.contentEl || (relatedTarget && relatedTarget.closest('#editorContent'))) {
                    // 延迟检查，确保不是正常的焦点转移
                    setTimeout(() => {
                        const activeElement = document.activeElement;
                        if (activeElement && (
                            activeElement.classList.contains('editing') ||
                            activeElement.contentEditable === 'true' ||
                            activeElement.tagName === 'INPUT' ||
                            activeElement.tagName === 'TEXTAREA'
                        ) && activeElement !== this.contentEl) {
                            this.isOtherInputFocused = true;
                        }
                    }, 10);
                }
            }
        }, true);
    }
    
    insertText(text) {
        const selection = window.getSelection();
        const range = selection.getRangeAt(0);
        
        // 保存插入前的光标位置
        const startOffset = this.getTextOffset(range.startContainer, range.startOffset);
        
        range.deleteContents();
        const textNode = document.createTextNode(text);
        range.insertNode(textNode);
        
        // 计算插入后光标应该在的位置
        const targetOffset = startOffset + text.length;
        
        // 更新内容
        this.updateValue();
        this.updateLineNumbers();
        this.applySyntaxHighlighting();
        
        // 在语法高亮后恢复光标到正确位置
        this.restoreCursorPosition(targetOffset);
        
        this.triggerChange();
    }
    
    updateValue() {
        // 从ContentEditable获取纯文本值
        this.value = this.getPlainText();
    }
    
    getPlainText() {
        // 从ContentEditable提取纯文本，需要将图片元素转换回原始文本
        let text = '';
        const walker = document.createTreeWalker(
            this.contentEl,
            NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT,
            {
                acceptNode: (node) => {
                    // 接受文本节点和img元素
                    if (node.nodeType === Node.TEXT_NODE) {
                        return NodeFilter.FILTER_ACCEPT;
                    }
                    if (node.nodeType === Node.ELEMENT_NODE && 
                        node.tagName.toLowerCase() === 'img' && 
                        node.classList.contains('inline-image-locator')) {
                        return NodeFilter.FILTER_ACCEPT;
                    }
                    return NodeFilter.FILTER_SKIP;
                }
            },
            false
        );
        
        let currentNode;
        while (currentNode = walker.nextNode()) {
            if (currentNode.nodeType === Node.TEXT_NODE) {
                text += currentNode.textContent;
            } else if (currentNode.nodeType === Node.ELEMENT_NODE && 
                       currentNode.tagName.toLowerCase() === 'img' && 
                       currentNode.classList.contains('inline-image-locator')) {
                // 将图片元素转换回原始的 @{name} 格式
                const imageName = currentNode.dataset.name || currentNode.alt;
                if (imageName) {
                    text += `@{${imageName}}`;
                }
            }
        }
        
        return text;
    }
    
    setValue(text) {
        this.value = text || '';
        
        if (!this.value) {
            this.showPlaceholder();
        } else {
            this.hidePlaceholder();
            // 确保LocatorManager数据已加载后再渲染
            this.ensureLocatorsLoadedThenHighlight();
        }
        
        this.updateLineNumbers();
    }
    
    // 确保定位器数据加载完成后再高亮
    async ensureLocatorsLoadedThenHighlight() {
        try {
            // 如果LocatorManager存在但数据为空，先加载数据
            const locatorManager = window.LocatorManagerModule?.instance;
            if (locatorManager && (!locatorManager.locators || Object.keys(locatorManager.locators).length === 0)) {
                await locatorManager.loadLocators();
            }
        } catch (error) {
            console.warn('Failed to ensure locators loaded:', error);
        }
        
        // 然后应用语法高亮
        this.applySyntaxHighlighting();
    }
    
    getValue() {
        return this.value;
    }
    
    updateLineNumbers() {
        const lines = this.value.split('\n');
        const lineNumbersHtml = lines.map((_, index) => 
            `<div class="line-number">${index + 1}</div>`
        ).join('');
        this.lineNumbersEl.innerHTML = lineNumbersHtml;
    }
    
    applySyntaxHighlighting() {
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
        if (!this.isTestRunning && !this.suppressCursorRestore && !this.isOtherInputFocused) {
            const selection = window.getSelection();
            const range = selection.rangeCount > 0 ? selection.getRangeAt(0) : null;
            cursorOffset = range ? this.getTextOffset(range.startContainer, range.startOffset) : 0;
        }
        
        // 设置高亮内容
        this.contentEl.innerHTML = highlightedHtml;
        
        // 恢复光标位置（仅在非测试运行时、未被抑制时且没有其他输入焦点时）
        if (!this.isTestRunning && !this.suppressCursorRestore && !this.isOtherInputFocused && cursorOffset > 0) {
            // 使用setTimeout确保DOM更新完成
            setTimeout(() => {
                if (!this.isOtherInputFocused) {
                    this.restoreCursorPosition(cursorOffset);
                }
            }, 0);
        }
    }
    
    // 获取文本偏移位置
    getTextOffset(node, offset) {
        let textOffset = 0;
        const walker = document.createTreeWalker(
            this.contentEl,
            NodeFilter.SHOW_TEXT,
            null,
            false
        );
        
        let currentNode;
        while (currentNode = walker.nextNode()) {
            if (currentNode === node) {
                return textOffset + offset;
            }
            textOffset += currentNode.textContent.length;
        }
        
        return textOffset;
    }
    
    // 恢复光标位置
    restoreCursorPosition(offset) {
        if (offset <= 0 || this.suppressCursorRestore || this.isOtherInputFocused) return;
        
        try {
            const textLength = this.getPlainText().length;
            
            // 如果偏移超出文本长度，将光标设置到文本末尾
            if (offset > textLength) {
                offset = textLength;
            }
            
            const walker = document.createTreeWalker(
                this.contentEl,
                NodeFilter.SHOW_TEXT,
                null,
                false
            );
            
            let currentOffset = 0;
            let currentNode;
            let lastValidNode = null;
            
            while (currentNode = walker.nextNode()) {
                lastValidNode = currentNode; // 记录最后一个有效节点
                const nodeLength = currentNode.textContent.length;
                
                if (currentOffset + nodeLength >= offset) {
                    // 找到目标节点
                    const range = document.createRange();
                    const selection = window.getSelection();
                    
                    const targetOffset = Math.min(offset - currentOffset, nodeLength);
                    
                    // 确保偏移值在有效范围内
                    const safeOffset = Math.max(0, Math.min(targetOffset, nodeLength));
                    
                    range.setStart(currentNode, safeOffset);
                    range.collapse(true);
                    
                    selection.removeAllRanges();
                    selection.addRange(range);
                    
                    return;
                }
                currentOffset += nodeLength;
            }
            
            // 如果没有找到合适的位置，将光标设置到最后一个文本节点的末尾
            if (lastValidNode) {
                const range = document.createRange();
                const selection = window.getSelection();
                
                range.setStart(lastValidNode, lastValidNode.textContent.length);
                range.collapse(true);
                
                selection.removeAllRanges();
                selection.addRange(range);
            }
            
        } catch (error) {
            console.warn('光标位置恢复失败:', error);
            // 如果恢复失败，尝试一个更安全的方法
            this.setCursorToSafePosition(offset);
        }
    }
    
    // 安全地设置光标位置
    setCursorToSafePosition(preferredOffset = 0) {
        try {
            const selection = window.getSelection();
            const range = document.createRange();
            
            // 首先尝试设置到内容开始
            if (this.contentEl.firstChild) {
                if (this.contentEl.firstChild.nodeType === Node.TEXT_NODE) {
                    const textLength = this.contentEl.firstChild.textContent.length;
                    const safeOffset = Math.min(preferredOffset, textLength);
                    range.setStart(this.contentEl.firstChild, safeOffset);
                } else {
                    range.setStartBefore(this.contentEl.firstChild);
                }
                range.collapse(true);
                
                selection.removeAllRanges();
                selection.addRange(range);
            }
        } catch (error) {
            console.warn('设置安全光标位置失败:', error);
        }
    }
    
    // 处理删除键操作
    handleDeleteKey(e) {
        try {
            const selection = window.getSelection();
            if (selection.rangeCount === 0) return;
            
            const range = selection.getRangeAt(0);
            
            // 如果有选中内容，让默认删除操作处理
            if (!range.collapsed) {
                return; // 不阻止默认行为
            }
            
            // 检查是否在图片定位器附近
            const cursorOffset = this.getTextOffset(range.startContainer, range.startOffset);
            const text = this.getPlainText();
            
            // 查找附近的图片定位器
            const nearbyLocator = this.findNearbyImageLocator(text, cursorOffset, e.key === 'Backspace');
            
            if (nearbyLocator) {
                // 如果删除操作会影响图片定位器，进行特殊处理
                e.preventDefault();
                this.handleLocatorDeletion(nearbyLocator, cursorOffset, e.key === 'Backspace');
            }
            
        } catch (error) {
            console.warn('删除键处理失败:', error);
        }
    }
    
    // 处理删除输入
    handleDeleteInput(e) {
        // 抑制光标恢复，防止删除操作后光标跳转
        this.suppressCursorRestore = true;
        
        setTimeout(() => {
            this.suppressCursorRestore = false;
        }, 100);
    }
    
    // 查找附近的图片定位器
    findNearbyImageLocator(text, cursorOffset, isBackspace) {
        const imageLocatorRegex = /@\{([^}]+)\}/g;
        let match;
        
        while ((match = imageLocatorRegex.exec(text)) !== null) {
            const start = match.index;
            const end = match.index + match[0].length;
            
            if (isBackspace) {
                // Backspace: 检查光标是否在定位器内部或紧接着定位器后面
                if (cursorOffset > start && cursorOffset <= end) {
                    return { start, end, name: match[1], fullMatch: match[0] };
                }
            } else {
                // Delete: 检查光标是否在定位器内部或紧接着定位器前面
                if (cursorOffset >= start && cursorOffset < end) {
                    return { start, end, name: match[1], fullMatch: match[0] };
                }
            }
        }
        
        return null;
    }
    
    // 处理定位器删除
    handleLocatorDeletion(locator, cursorOffset, isBackspace) {
        try {
            // 抑制光标恢复，避免在删除过程中的异常跳转
            this.suppressCursorRestore = true;
            
            // 如果光标在定位器内部，删除整个定位器
            const text = this.getPlainText();
            const beforeText = text.substring(0, locator.start);
            const afterText = text.substring(locator.end);
            const newText = beforeText + afterText;
            
            // 更新值
            this.value = newText;
            
            // 先更新行号
            this.updateLineNumbers();
            
            // 应用语法高亮
            this.applySyntaxHighlighting();
            
            // 设置光标到删除位置（定位器的开始位置）
            setTimeout(() => {
                // 确保偏移位置有效
                const targetOffset = Math.min(locator.start, newText.length);
                this.setCursorToSafePosition(targetOffset);
                this.suppressCursorRestore = false;
            }, 50);
            
            this.triggerChange();
            
        } catch (error) {
            console.warn('定位器删除处理失败:', error);
            this.suppressCursorRestore = false;
        }
    }
    
    // 更新编辑器状态提示
    updateStatusIndicator() {
        if (!this.statusIndicatorEl) return;
        
        if (this.isTestRunning) {
            // 测试运行状态 - 黄色
            this.statusIndicatorEl.className = 'editor-status-indicator running';
            this.statusIndicatorEl.textContent = '运行中';
            this.statusIndicatorEl.style.display = 'block';
        } else if (this.forceTextMode) {
            // 文本编辑模式 - 蓝色
            this.statusIndicatorEl.className = 'editor-status-indicator text-mode';
            this.statusIndicatorEl.textContent = '文本模式';
            this.statusIndicatorEl.style.display = 'block';
        } else {
            // 普通状态 - 隐藏
            this.statusIndicatorEl.style.display = 'none';
        }
    }
    
    // 切换所有图片定位器为文本格式
    toggleAllImagesToText() {
        try {
            // 保存当前光标位置
            const selection = window.getSelection();
            let cursorOffset = 0;
            if (selection.rangeCount > 0) {
                const range = selection.getRangeAt(0);
                cursorOffset = this.getTextOffset(range.startContainer, range.startOffset);
            }
            
            // 如果已经是强制文本模式，则切换回图片模式
            if (this.forceTextMode) {
                this.forceTextMode = false;
                console.log('切换回图片模式');
            } else {
                // 切换到强制文本模式
                this.forceTextMode = true;
                console.log('切换到文本模式');
            }
            
            // 更新状态提示
            this.updateStatusIndicator();
            
            // 抑制光标恢复
            this.suppressCursorRestore = true;
            
            // 重新应用语法高亮
            this.applySyntaxHighlighting();
            
            // 恢复光标到原始位置
            setTimeout(() => {
                this.restoreCursorPosition(cursorOffset);
                this.suppressCursorRestore = false;
            }, 50);
            
        } catch (error) {
            console.warn('切换图片为文本模式失败:', error);
            this.suppressCursorRestore = false;
            this.forceTextMode = false;
        }
    }
    
    
    highlightSyntax(text) {
        if (!text) return '';
        
        // 按行处理，为TKS格式应用语法高亮
        const lines = text.split('\n');
        const highlightedLines = lines.map(line => this.highlightTksLine(line));
        
        return highlightedLines.join('\n');
    }
    
    highlightTksLine(line) {
        if (!line.trim()) return line;
        
        // HTML转义
        let escaped = line
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
        
        // TKS语法高亮规则
        let result = escaped;
        
        // 1. 头部字段 (用例:, 脚本名:, 详情:, 步骤:)
        if (/^(用例|脚本名|详情|步骤)\s*:/.test(result)) {
            result = result.replace(/^(用例|脚本名|详情|步骤)(\s*)(:)/, 
                '<span class="syntax-key">$1</span>$2<span class="syntax-punctuation">$3</span>');
        }
        
        // 2. 缩进的配置项 (appPackage:, appActivity:)
        else if (/^\s+(appPackage|appActivity)\s*:/.test(result)) {
            result = result.replace(/^(\s+)(appPackage|appActivity)(\s*)(:)/, 
                '$1<span class="syntax-key">$2</span>$3<span class="syntax-punctuation">$4</span>');
        }
        
        // 3. 动作关键词 (支持缩进)
        else if (/^\s*(启动|关闭|点击|按压|滑动|定向滑动|输入|清理|隐藏键盘|返回|等待|断言)\s/.test(result)) {
            result = result.replace(/^(\s*)(启动|关闭|点击|按压|滑动|定向滑动|输入|清理|隐藏键盘|返回|等待|断言)(\s)/, 
                '$1<span class="syntax-action">$2</span>$3');
        }
        
        // 4. 图片定位器 @{图片名称} - 渲染为实际图片
        result = result.replace(/@\{([^}]+)\}/g, (match, imageName) => {
            return this.renderImageLocator(imageName, match);
        });
        
        // 5. 方括号内容 [坐标], [元素], [时间] 等
        result = result.replace(/\[([^\]]+)\]/g, function(match, content) {
            // 坐标格式 [数字,数字]
            if (/^\d+,\d+$/.test(content)) {
                const coords = content.split(',');
                return '[<span class="syntax-coordinate">' + coords[0] + '</span>,<span class="syntax-coordinate">' + coords[1] + '</span>]';
            }
            // 时间格式 [数字s] 或 [数字ms]
            else if (/^\d+(s|ms|秒|毫秒)$/.test(content)) {
                return '[<span class="syntax-number">' + content.replace(/(\d+)(s|ms|秒|毫秒)/, '$1</span><span class="syntax-unit">$2') + '</span>]';
            }
            // 其他内容（元素、图片等）
            else {
                return '[<span class="syntax-string">' + content + '</span>]';
            }
        });
        
        // 6. 存在、不存在等断言关键词
        const assertionKeywords = ['存在', '不存在', '显示', '隐藏', '包含', '不包含', '等于', '不等于'];
        assertionKeywords.forEach(keyword => {
            const regex = new RegExp('\\b(' + keyword + ')\\b', 'g');
            result = result.replace(regex, '<span class="syntax-assertion">$1</span>');
        });
        
        // 7. 时间格式 (单独出现的)
        result = result.replace(/\b(\d+)(s|ms|秒|毫秒)\b/g, '<span class="syntax-number">$1</span><span class="syntax-unit">$2</span>');
        
        // 8. 包名格式
        result = result.replace(/\b(com\.[a-zA-Z0-9_.]+)\b/g, '<span class="syntax-package">$1</span>');
        
        // 9. 方向关键词
        const directions = ['上', '下', '左', '右', '向上', '向下', '向左', '向右'];
        directions.forEach(direction => {
            const regex = new RegExp('\\b(' + direction + ')\\b', 'g');
            result = result.replace(regex, '<span class="syntax-direction">$1</span>');
        });
        
        return result;
    }
    
    // 渲染图片定位器为实际图片
    renderImageLocator(imageName, originalText) {
        // 如果强制文本模式，所有图片都显示为文本
        if (this.forceTextMode) {
            return `@{<span class="syntax-string">${imageName}</span>}`;
        }
        
        // 测试运行期间总是显示为图片，不检查光标位置
        if (this.isTestRunning) {
            return this.renderAsImage(imageName, originalText);
        }
        
        // 使用ContentEditable的光标检测来判断是否正在编辑这个定位器
        try {
            const selection = window.getSelection();
            if (selection.rangeCount > 0) {
                const range = selection.getRangeAt(0);
                const cursorOffset = this.getTextOffset(range.startContainer, range.startOffset);
                const text = this.getPlainText();
                
                // 查找所有与此图片名匹配的定位器位置
                const searchText = `@{${imageName}}`;
                let index = text.indexOf(searchText);
                
                while (index !== -1) {
                    const start = index;
                    const end = index + searchText.length;
                    
                    // 只有光标在定位器内部时才显示为文本（不包括边界位置）
                    // 这样光标可以停留在图片前后而不会触发切换
                    if (cursorOffset > start && cursorOffset < end) {
                        return `@{<span class="syntax-string">${imageName}</span>}`;
                    }
                    
                    // 查找下一个同名的定位器
                    index = text.indexOf(searchText, end);
                }
            }
        } catch (error) {
            console.warn('光标位置检测失败，默认渲染为图片:', error);
            return this.renderAsImage(imageName, originalText);
        }
        
        return this.renderAsImage(imageName, originalText);
    }
    
    // 将定位器渲染为图片的通用方法
    renderAsImage(imageName, originalText) {
        // 获取当前项目路径
        const projectPath = window.AppGlobals.currentProject;
        if (!projectPath) {
            return `@{<span class="syntax-string">${imageName}</span>}`;
        }
        
        // 尝试从元素库中查找对应的图片路径
        try {
            // 从LocatorManager获取定位器信息
            const locatorManager = window.LocatorManagerModule?.instance;
            
            if (locatorManager && locatorManager.locators && locatorManager.locators[imageName]) {
                const locatorData = locatorManager.locators[imageName];
                
                if (locatorData.type === 'image' && locatorData.path) {
                    const { path: PathModule } = window.AppGlobals;
                    const imagePath = PathModule.join(projectPath, locatorData.path);
                    
                    // 直接渲染为内联图片，失败时显示原始文本
                    const fallbackText = originalText || `@{${imageName}}`;
                    return `<img class="inline-image-locator" src="file://${imagePath}" alt="${imageName}" data-name="${imageName}" draggable="false" onerror="this.outerHTML='<span class=&quot;syntax-string&quot;>${fallbackText.replace(/"/g, '&quot;')}</span>';">`;
                }
            }
        } catch (error) {
            console.warn('Failed to resolve image locator:', imageName, error);
        }
        
        // 如果找不到对应的图片，显示原始文本
        return `@{<span class="syntax-string">${imageName}</span>}`;
    }
    
    // 设置光标到图片定位器位置
    setCursorToImageLocator(imageElement) {
        const imageName = imageElement.dataset.name;
        if (!imageName) return;
        
        // 通过计算位置来精确定位对应的文本
        const textToFind = `@{${imageName}}`;
        this.setCursorByPosition(imageElement, textToFind);
    }
    
    // 设置光标到指定位置
    setCursorByPosition(wrapperElement, textToFind) {
        // 获取wrapper在highlight层中的位置
        const highlightRect = this.highlightEl.getBoundingClientRect();
        const wrapperRect = wrapperElement.getBoundingClientRect();
        
        // 计算相对位置
        const relativeTop = wrapperRect.top - highlightRect.top + this.highlightEl.scrollTop;
        
        // 根据行高计算行号
        const lineHeight = parseFloat(getComputedStyle(this.textareaEl).lineHeight);
        const lineNumber = Math.floor(relativeTop / lineHeight);
        
        // 查找对应行中的文本
        const lines = this.textareaEl.value.split('\n');
        if (lineNumber >= 0 && lineNumber < lines.length) {
            const lineText = lines[lineNumber];
            const textIndex = lineText.indexOf(textToFind);
            
            if (textIndex !== -1) {
                // 计算在整个文本中的位置
                const linesBeforeCurrent = lines.slice(0, lineNumber);
                const totalCharsBeforeLine = linesBeforeCurrent.reduce((sum, line) => sum + line.length + 1, 0); // +1 for \n
                const absoluteIndex = totalCharsBeforeLine + textIndex;
                
                // 聚焦textarea并设置光标到文本开始位置
                this.textareaEl.focus();
                this.textareaEl.setSelectionRange(absoluteIndex, absoluteIndex);
                
                // 立即重新渲染以显示文本模式
                this.handleCursorChange();
            }
        }
    }
    
    // 处理光标位置变化
    handleCursorChange() {
        if (!this.textareaEl.value) return;
        
        const cursorPosition = this.textareaEl.selectionStart;
        const text = this.textareaEl.value;
        
        // 查找所有图片定位器
        const imageLocatorRegex = /@\{([^}]+)\}/g;
        const matches = [];
        let match;
        
        while ((match = imageLocatorRegex.exec(text)) !== null) {
            matches.push({
                name: match[1],
                start: match.index,
                end: match.index + match[0].length,
                fullMatch: match[0]
            });
        }
        
        // 检查光标是否在某个图片定位器内
        this.currentEditingLocator = null;
        for (const locatorMatch of matches) {
            if (cursorPosition >= locatorMatch.start && cursorPosition <= locatorMatch.end) {
                this.currentEditingLocator = locatorMatch;
                break;
            }
        }
        
        // 重新渲染高亮
        this.applySyntaxHighlightingWithCursor();
    }
    
    // 通过位置精确选择文本
    selectTextByPosition(wrapperElement, textToFind) {
        // 获取wrapper在highlight层中的位置
        const highlightRect = this.highlightEl.getBoundingClientRect();
        const wrapperRect = wrapperElement.getBoundingClientRect();
        
        // 计算相对位置
        const relativeTop = wrapperRect.top - highlightRect.top + this.highlightEl.scrollTop;
        const relativeLeft = wrapperRect.left - highlightRect.left + this.highlightEl.scrollLeft;
        
        // 根据行高和字符宽度估算文本位置
        const lineHeight = parseFloat(getComputedStyle(this.textareaEl).lineHeight);
        const lineNumber = Math.floor(relativeTop / lineHeight);
        
        // 查找对应行中的文本
        const lines = this.textareaEl.value.split('\n');
        if (lineNumber >= 0 && lineNumber < lines.length) {
            const lineText = lines[lineNumber];
            const textIndex = lineText.indexOf(textToFind);
            
            if (textIndex !== -1) {
                // 计算在整个文本中的位置
                const linesBeforeCurrent = lines.slice(0, lineNumber);
                const totalCharsBeforeLine = linesBeforeCurrent.reduce((sum, line) => sum + line.length + 1, 0); // +1 for \n
                const absoluteIndex = totalCharsBeforeLine + textIndex;
                
                // 聚焦textarea并选择对应文本
                this.textareaEl.focus();
                this.textareaEl.setSelectionRange(absoluteIndex, absoluteIndex + textToFind.length);
            }
        }
    }
    
    // 切换图片定位器的显示模式
    toggleImageLocator(imageElement) {
        const wrapper = imageElement.closest('.image-locator-wrapper');
        if (!wrapper) return;
        
        const fallback = wrapper.querySelector('.image-locator-fallback');
        if (!fallback) return;
        
        // 切换显示模式
        if (imageElement.style.display === 'none') {
            // 显示图片，隐藏文本
            imageElement.style.display = 'inline';
            fallback.style.display = 'none';
        } else {
            // 显示文本，隐藏图片
            imageElement.style.display = 'none';
            fallback.style.display = 'inline';
        }
    }
    
    highlightLine(line) {
        if (!line.trim()) return line;
        
        // HTML转义
        let escaped = line
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
        
        // 按优先级应用高亮规则，避免重复匹配
        escaped = this.applyHighlightRules(escaped);
        
        return escaped;
    }
    
    applyHighlightRules(line) {
        let result = line;
        
        // 跳过已经有span标签的行，避免重复处理
        if (result.includes('<span class=')) {
            return result;
        }
        
        // 1. 注释 - 优先级最高，如果是注释行就直接返回
        if (result.match(/#/)) {
            return result.replace(/(#.*$)/, '<span class="syntax-comment">$1</span>');
        }
        
        // 2. YAML字段 (key:) - 检测行首的key: 模式
        const yamlFieldMatch = result.match(/^(\s*)(\w+)(:)/);
        if (yamlFieldMatch) {
            const [, spaces, field, colon] = yamlFieldMatch;
            const yamlFields = ['name', 'description', 'discription', 'action', 'params', 'steps', 'appPackage', 'appActivity'];
            if (yamlFields.includes(field)) {
                result = result.replace(
                    new RegExp(`^(\\s*)(${field})(:)`), 
                    `$1<span class="syntax-key">$2</span><span class="syntax-punctuation">$3</span>`
                );
            }
        }
        
        // 3. 数组标识符
        result = result.replace(/^(\s*)(-)(\s+)/, '$1<span class="syntax-dash">$2</span>$3');
        
        // 4. 中文动作关键词（行首）
        const lowCodeActions = ['启动', '关闭', '点击', '按压', '滑动', '定向滑动', '输入', '清理', '隐藏键盘', '返回', '等待', '断言'];
        for (const action of lowCodeActions) {
            const regex = new RegExp(`^(\\s*)(${action})(\\s|$)`);
            if (regex.test(result)) {
                result = result.replace(regex, `$1<span class="syntax-action">$2</span>$3`);
                break; // 只匹配一个动作
            }
        }
        
        // 5. 英文动作关键词
        const englishActions = ['launch_app', 'click', 'tap', 'press', 'swipe', 'scroll', 'input', 'type', 'clear', 'wait', 'assert', 'back', 'hide_keyboard', 'take_screenshot'];
        for (const action of englishActions) {
            const regex = new RegExp(`\\b(${action})\\b`);
            if (regex.test(result)) {
                result = result.replace(regex, `<span class="syntax-action-en">$1</span>`);
                break; // 只匹配一个动作
            }
        }
        
        // 6. 坐标格式 [x,y]
        result = result.replace(/\[(\d+),(\d+)\]/g, '[<span class="syntax-coordinate">$1</span>,<span class="syntax-coordinate">$2</span>]');
        
        // 7. 时间格式
        result = result.replace(/(\d+)(s|ms|秒|毫秒)\b/g, '<span class="syntax-number">$1</span><span class="syntax-unit">$2</span>');
        
        // 8. 断言关键词
        const assertionKeywords = ['存在', '不存在', '显示', '隐藏', '包含', '不包含', '等于', '不等于'];
        for (const keyword of assertionKeywords) {
            const regex = new RegExp(`\\b(${keyword})\\b`);
            if (regex.test(result)) {
                result = result.replace(regex, `<span class="syntax-assertion">$1</span>`);
                break;
            }
        }
        
        // 9. 包名 (com.xxx.xxx)
        result = result.replace(/\b(com\.[\w\.]+)\b/g, '<span class="syntax-package">$1</span>');
        
        // 10. 字符串（引号包围）
        result = result.replace(/(['"])((?:\\.|(?!\1)[^\\])*?)\1/g, '<span class="syntax-string">$1$2$1</span>');
        
        // 11. 布尔值
        result = result.replace(/\b(true|false|True|False|TRUE|FALSE)\b/g, '<span class="syntax-boolean">$1</span>');
        
        // 12. 纯数字（最后处理，避免与其他规则冲突，并确保不在span标签内）
        result = result.replace(/\b(\d+)\b(?![^<]*>)/g, '<span class="syntax-number">$1</span>');
        
        return result;
    }
    
    
    setPlaceholder(text) {
        this.placeholderText = text;
        // 只有当内容确实为空，并且编辑器没有任何文本内容时才显示占位符
        if (!this.value && (!this.contentEl.textContent || this.contentEl.textContent.trim() === '')) {
            this.showPlaceholder();
        }
    }
    
    showPlaceholder() {
        // 只有当确实没有内容时才显示占位符
        if (!this.value && (!this.contentEl.textContent || this.contentEl.textContent.trim() === '')) {
            this.contentEl.innerHTML = `<span class="editor-placeholder">${this.placeholderText}</span>`;
        }
    }
    
    hidePlaceholder() {
        const placeholder = this.contentEl.querySelector('.editor-placeholder');
        if (placeholder) {
            this.contentEl.innerHTML = '';
        }
    }
    
    triggerChange() {
        // 触发change事件
        this.listeners.forEach(listener => {
            if (listener.type === 'change') {
                listener.callback(this.value);
            }
        });
        
        // 触发自动保存
        const activeTab = document.querySelector('.tab.active');
        if (activeTab) {
            const tabId = activeTab.id;
            const tab = window.AppGlobals.openTabs.find(t => t.id === tabId);
            if (tab) {
                tab.content = this.value;
                
                // 延迟自动保存
                clearTimeout(this.saveTimeout);
                this.saveTimeout = setTimeout(() => {
                    if (window.EditorModule && window.EditorModule.saveCurrentFile) {
                        window.EditorModule.saveCurrentFile();
                    }
                }, 1000);
            }
        }
    }
    
    on(event, callback) {
        this.listeners.push({ type: event, callback });
    }
    
    focus() {
        if (!this.isTestRunning) {
            this.contentEl.focus();
        }
    }
    
    // 设置光标到内容末尾
    setCursorToEnd() {
        try {
            const selection = window.getSelection();
            const range = document.createRange();
            range.selectNodeContents(this.contentEl);
            range.collapse(false); // false表示折叠到末尾
            selection.removeAllRanges();
            selection.addRange(range);
        } catch (error) {
            console.warn('设置光标到末尾失败:', error);
        }
    }
    
    // 处理图片定位器点击
    handleImageLocatorClick(imageElement) {
        const imageName = imageElement.dataset.name;
        if (!imageName) return;
        
        // 抑制光标恢复，防止异常跳转
        this.suppressCursorRestore = true;
        
        try {
            // 计算图片在文本中的精确位置
            const imagePosition = this.getImagePositionInText(imageElement, imageName);
            
            // 将图片替换为文本形式
            const textToInsert = `@{${imageName}}`;
            const textNode = document.createTextNode(textToInsert);
            imageElement.parentNode.replaceChild(textNode, imageElement);
            
            // 更新内容值
            this.updateValue();
            
            // 设置光标到文本开始位置（这样用户可以选择整个定位器）
            const range = document.createRange();
            const selection = window.getSelection();
            
            range.setStart(textNode, 0);
            range.setEnd(textNode, textToInsert.length);
            
            selection.removeAllRanges();
            selection.addRange(range);
            
            
        } catch (error) {
            console.warn('图片定位器点击处理失败:', error);
        }
        
        // 延迟恢复光标恢复功能
        setTimeout(() => {
            this.suppressCursorRestore = false;
        }, 200);
    }
    
    // 计算图片在文本中的位置
    getImagePositionInText(imageElement, imageName) {
        try {
            const textContent = this.getPlainText();
            const searchText = `@{${imageName}}`;
            
            // 通过DOM位置大概估算在文本中的位置
            let walker = document.createTreeWalker(
                this.contentEl,
                NodeFilter.SHOW_ALL,
                null,
                false
            );
            
            let position = 0;
            let currentNode;
            
            while (currentNode = walker.nextNode()) {
                if (currentNode === imageElement) {
                    return position;
                }
                if (currentNode.nodeType === Node.TEXT_NODE) {
                    position += currentNode.textContent.length;
                } else if (currentNode.nodeType === Node.ELEMENT_NODE && 
                          currentNode.classList && 
                          currentNode.classList.contains('inline-image-locator')) {
                    const imgName = currentNode.dataset.name;
                    if (imgName) {
                        position += `@{${imgName}}`.length;
                    }
                }
            }
            
            return position;
        } catch (error) {
            console.warn('计算图片位置失败:', error);
            return 0;
        }
    }
    
    // 显示图片预览
    showImagePreview(imageElement, event) {
        // 如果预览已存在，先移除
        this.hideImagePreview();
        
        const imageSrc = imageElement.src;
        const imageName = imageElement.dataset.name;
        
        if (!imageSrc || !imageName) return;
        
        // 创建预览容器
        const preview = document.createElement('div');
        preview.className = 'image-preview-tooltip';
        preview.innerHTML = `
            <div class="preview-header">${imageName}</div>
            <img src="${imageSrc}" alt="${imageName}" class="preview-image">
        `;
        
        // 添加到body
        document.body.appendChild(preview);
        this.currentPreview = preview;
        
        // 计算位置
        this.positionPreview(preview, imageElement, event);
        
        // 立即显示
        requestAnimationFrame(() => {
            if (this.currentPreview === preview) {
                preview.classList.add('show');
            }
        });
        
        // 监听鼠标移动以更新位置
        this.previewMouseMoveHandler = (e) => {
            this.positionPreview(preview, imageElement, e);
        };
        document.addEventListener('mousemove', this.previewMouseMoveHandler);
    }
    
    // 隐藏图片预览
    hideImagePreview() {
        if (this.currentPreview) {
            document.body.removeChild(this.currentPreview);
            this.currentPreview = null;
        }
        
        if (this.previewMouseMoveHandler) {
            document.removeEventListener('mousemove', this.previewMouseMoveHandler);
            this.previewMouseMoveHandler = null;
        }
    }
    
    // 定位预览窗口
    positionPreview(preview, imageElement, event) {
        const mouseX = event.clientX;
        const mouseY = event.clientY;
        const windowWidth = window.innerWidth;
        const windowHeight = window.innerHeight;
        
        // 预览窗口尺寸
        const previewWidth = 300; // 预估宽度
        const previewHeight = 200; // 预估高度
        const offset = 15; // 鼠标偏移
        
        let left = mouseX + offset;
        let top = mouseY + offset;
        
        // 防止超出右边界
        if (left + previewWidth > windowWidth) {
            left = mouseX - previewWidth - offset;
        }
        
        // 防止超出下边界
        if (top + previewHeight > windowHeight) {
            top = mouseY - previewHeight - offset;
        }
        
        // 防止超出左边界和上边界
        left = Math.max(10, left);
        top = Math.max(10, top);
        
        preview.style.left = left + 'px';
        preview.style.top = top + 'px';
    }
    
    // 检查光标是否在图片定位器文本范围内
    checkImageLocatorCursor() {
        const selection = window.getSelection();
        if (selection.rangeCount === 0) return;
        
        const range = selection.getRangeAt(0);
        
        // 如果有选择文本（不是单纯的光标位置），不进行处理，保持用户选择
        if (!range.collapsed) {
            return;
        }
        
        const cursorOffset = this.getTextOffset(range.startContainer, range.startOffset);
        const text = this.getPlainText();
        
        // 查找所有图片定位器位置
        const imageLocatorRegex = /@\{([^}]+)\}/g;
        let match;
        let isInsideAnyLocator = false;
        
        while ((match = imageLocatorRegex.exec(text)) !== null) {
            const start = match.index;
            const end = match.index + match[0].length;
            
            // 检查光标是否在某个定位器内部（不包括边界）
            if (cursorOffset > start && cursorOffset < end) {
                isInsideAnyLocator = true;
                break;
            }
        }
        
        // 只有当光标确实在定位器内部时，才需要特殊处理
        // 如果光标在定位器外部（包括紧靠着的位置），让用户正常编辑
        if (!isInsideAnyLocator) {
            // 不执行恢复图片操作，允许光标停留在任何位置
            return;
        }
        
        // 光标在定位器内部时，可以选择切换为文本模式便于编辑
        // 但这里我们保持图片模式，用户可以通过点击图片来切换
    }
    
    // 恢复所有图片显示
    restoreAllImages() {
        // 重新应用语法高亮，这会将所有@{name}转换回图片
        this.applySyntaxHighlighting();
    }
    
    // 刷新图片定位器渲染
    refreshImageLocators() {
        this.applySyntaxHighlighting();
    }
    
    // 更新字体设置
    updateFontSettings(fontFamily, fontSize) {
        const lineHeight = fontSize + 7; // 保持行高与字体大小的比例
        
        const fontSettings = {
            fontFamily: fontFamily,
            fontSize: fontSize + 'px',
            lineHeight: lineHeight + 'px'
        };
        
        // 更新ContentEditable样式
        Object.assign(this.contentEl.style, fontSettings);
        
        // 更新行号的字体设置以保持对齐
        if (this.lineNumbersEl) {
            Object.assign(this.lineNumbersEl.style, {
                fontFamily: fontFamily,
                fontSize: fontSize + 'px',
                lineHeight: lineHeight + 'px'
            });
        }
        
        // 重新计算行号和重新渲染
        this.updateLineNumbers();
        this.applySyntaxHighlighting();
    }
    
    // 清除焦点和光标状态
    clearFocusAndCursor() {
        try {
            console.log('编辑器: 清除焦点和光标状态');
            
            // 移除编辑器焦点
            if (this.contentEl) {
                this.contentEl.blur();
            }
            
            // 清除所有选择区域
            if (window.getSelection) {
                const selection = window.getSelection();
                if (selection.rangeCount > 0) {
                    selection.removeAllRanges();
                }
            }
            
            // 将焦点移到body上，确保没有任何元素获得焦点
            if (document.body) {
                document.body.focus();
                // 立即再次失焦，确保没有任何可见的焦点状态
                setTimeout(() => document.body.blur(), 0);
            }
            
            console.log('编辑器: 焦点和光标状态已清除');
        } catch (error) {
            console.warn('清除焦点和光标状态失败:', error);
        }
    }
    
    destroy() {
        clearTimeout(this.saveTimeout);
        this.hideImagePreview(); // 清理图片预览
        this.listeners = [];
    }
    
    // 设置测试运行状态
    setTestRunning(isRunning) {
        // 如果状态没有改变，直接返回，避免不必要的DOM操作
        if (this.isTestRunning === isRunning) {
            return;
        }
        
        this.isTestRunning = isRunning;
        
        if (isRunning) {
            // 测试开始时立即清除焦点和光标状态，防止干扰高亮
            this.clearFocusAndCursor();
            
            // 禁用编辑器交互
            this.contentEl.setAttribute('contenteditable', 'false');
            this.contentEl.style.pointerEvents = 'none';
            this.contentEl.style.userSelect = 'none';
        } else {
            // 测试结束时恢复编辑器交互
            this.contentEl.setAttribute('contenteditable', 'true');
            this.contentEl.style.pointerEvents = 'auto';
            this.contentEl.style.userSelect = 'text';
        }
        
        // 更新状态提示 - 使用异步调用避免干扰当前的渲染流程
        setTimeout(() => {
            this.updateStatusIndicator();
        }, 0);
    }
    
    // 高亮当前执行的行
    highlightExecutingLine(lineNumber) {
        console.log(`编辑器: 高亮执行行 ${lineNumber}`);
        
        // 确保焦点和光标状态已清除，防止干扰高亮显示
        this.clearFocusAndCursor();
        
        // 先设置执行行信息
        this.currentExecutingLine = lineNumber;
        this.currentErrorLine = null; // 清除错误行高亮
        
        // 只在第一次高亮时设置测试运行状态，避免重复调用影响高亮
        if (!this.isTestRunning) {
            this.setTestRunning(true);
        }
        
        // 应用执行行高亮（这应该在状态设置之后）
        this.applySyntaxHighlightingWithExecution();
    }
    
    // 高亮错误行
    highlightErrorLine(lineNumber) {
        console.log(`编辑器: 高亮错误行 ${lineNumber}`);
        this.currentErrorLine = lineNumber;
        this.currentExecutingLine = null; // 清除执行行高亮
        this.applySyntaxHighlightingWithExecution();
        // 错误时保持测试运行状态，防止用户干预
    }
    
    // 清除执行行高亮
    clearExecutionHighlight() {
        console.log('编辑器: 清除执行行高亮');
        this.setTestRunning(false); // 测试结束时恢复编辑状态
        this.currentExecutingLine = null;
        this.currentErrorLine = null;
        this.applySyntaxHighlighting();
    }
    
    // 修改语法高亮应用方法以支持执行行高亮
    applySyntaxHighlightingWithExecution() {
        // 如果正在组合输入，不进行语法高亮
        if (this.isComposing) {
            return;
        }
        
        if (!this.value) {
            this.showPlaceholder();
            return;
        }
        
        this.hidePlaceholder();
        
        // 按行处理，同时应用语法高亮和执行行高亮
        const lines = this.value.split('\n');
        const highlightedLines = lines.map((line, index) => {
            const lineNumber = index + 1;
            let highlightedLine = this.highlightTksLine(line);
            
            // 如果是当前执行的行，添加执行高亮
            if (this.currentExecutingLine === lineNumber) {
                highlightedLine = `<span class="executing-line">${highlightedLine}</span>`;
            }
            // 如果是错误行，添加错误高亮
            else if (this.currentErrorLine === lineNumber) {
                highlightedLine = `<span class="error-line">${highlightedLine}</span>`;
            }
            
            return highlightedLine;
        });
        
        this.contentEl.innerHTML = highlightedLines.join('\n');
    }
    
    // 应用带光标状态的语法高亮
    applySyntaxHighlightingWithCursor() {
        if (!this.value) {
            this.showPlaceholder();
            return;
        }
        
        this.hidePlaceholder();
        
        // 使用正常的语法高亮
        const highlightedHtml = this.highlightSyntax(this.value);
        this.highlightEl.innerHTML = highlightedHtml;
    }
}

let editorInstance = null;

// 初始化编辑器
function initializeSimpleEditor() {
    const container = document.getElementById('simpleEditor');
    if (!container) {
        console.error('Editor container not found');
        return;
    }
    
    // 销毁旧的编辑器实例
    if (editorInstance) {
        editorInstance.destroy();
    }
    
    // 创建新的编辑器实例
    editorInstance = new SimpleCodeEditor(container);
    
    // 将编辑器实例存储到容器中，以便后续获取
    container._editor = editorInstance;
    
    // 更新全局变量引用
    window.AppGlobals.setCodeEditor({
        get value() { return editorInstance.getValue(); },
        set value(val) { editorInstance.setValue(val); },
        set placeholder(text) { editorInstance.setPlaceholder(text); },
        focus() { editorInstance.focus(); },
        refreshImageLocators() { editorInstance.refreshImageLocators(); },
        updateFontSettings(fontFamily, fontSize) { editorInstance.updateFontSettings(fontFamily, fontSize); },
        highlightExecutingLine(lineNumber) { editorInstance.highlightExecutingLine(lineNumber); },
        highlightErrorLine(lineNumber) { editorInstance.highlightErrorLine(lineNumber); },
        clearExecutionHighlight() { editorInstance.clearExecutionHighlight(); },
        setTestRunning(isRunning) { editorInstance.setTestRunning(isRunning); }
    });
    
    console.log('New Simple Editor initialized successfully');
    
    // 初始化后立即加载字体设置
    setTimeout(() => {
        if (window.SettingsModule && window.SettingsModule.loadEditorFontSettings) {
            window.SettingsModule.loadEditorFontSettings();
        }
    }, 100);
}


// 标签管理函数
function createTab(tab) {
    const tabsContainer = document.getElementById('editorTabs');
    
    const tabElement = document.createElement('div');
    tabElement.className = 'tab';
    tabElement.id = tab.id;
    tabElement.innerHTML = `
        <span class="tab-name" title="${tab.path}">${tab.name}</span>
        <span class="tab-close">×</span>
    `;
    
    tabElement.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close')) {
            selectTab(tab.id);
        }
    });
    
    tabElement.querySelector('.tab-close').addEventListener('click', (e) => {
        e.stopPropagation();
        closeTab(tab.id);
    });
    
    tabsContainer.appendChild(tabElement);
}

function selectTab(tabId) {
    console.log('Selecting tab:', tabId);
    
    // 首先，保存当前活动标签的内容
    const currentActiveTab = document.querySelector('.tab.active');
    if (currentActiveTab && window.AppGlobals.codeEditor) {
        const currentTabId = currentActiveTab.id;
        const currentTab = window.AppGlobals.openTabs.find(tab => tab.id === currentTabId);
        if (currentTab) {
            currentTab.content = window.AppGlobals.codeEditor.value;
        }
    }
    
    // 更新活动标签样式
    const tabs = document.querySelectorAll('.tab');
    tabs.forEach(tab => tab.classList.remove('active'));
    
    const selectedTab = document.getElementById(tabId);
    if (selectedTab) {
        selectedTab.classList.add('active');
        
        // 为选中的标签加载内容
        const tabData = window.AppGlobals.openTabs.find(tab => tab.id === tabId);
        if (tabData && window.AppGlobals.codeEditor) {
            console.log('Setting content for tab:', tabData.name, 'Content length:', tabData.content.length);
            window.AppGlobals.codeEditor.value = tabData.content;
            window.AppGlobals.codeEditor.placeholder = `正在编辑: ${tabData.name}`;
            
            // 更新当前标签页全局变量
            window.AppGlobals.setCurrentTab(tabData);
        }
    }
}

function closeTab(tabId) {
    const openTabs = window.AppGlobals.openTabs;
    const index = openTabs.findIndex(tab => tab.id === tabId);
    if (index > -1) {
        openTabs.splice(index, 1);
        window.AppGlobals.setOpenTabs(openTabs);
        
        const tabElement = document.getElementById(tabId);
        if (tabElement) {
            tabElement.remove();
        }
        
        // 如果有可用标签则选择另一个标签
        if (openTabs.length > 0) {
            selectTab(openTabs[openTabs.length - 1].id);
        } else {
            // 没有更多标签，清空编辑器
            if (window.AppGlobals.codeEditor) {
                window.AppGlobals.codeEditor.value = '';
                window.AppGlobals.codeEditor.placeholder = '在Project页面选择测试项并创建Case后, 在左侧文件树点击对应Case下的.tks自动化脚本开始编辑';
            }
            // 清空当前标签页
            window.AppGlobals.setCurrentTab(null);
        }
    }
}

// 保存相关函数
async function saveCurrentFile() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) return;
    
    const tabId = activeTab.id;
    const tab = window.AppGlobals.openTabs.find(t => t.id === tabId);
    
    if (tab) {
        const result = await window.AppGlobals.ipcRenderer.invoke('write-file', tab.path, tab.content);
        if (!result.success) {
            console.error('Failed to save file:', result.error);
        }
    }
}

async function saveCurrentFileWithNotification() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) {
        window.NotificationModule.showNotification('没有可保存的文件', 'warning');
        return;
    }
    
    const tabId = activeTab.id;
    const tab = window.AppGlobals.openTabs.find(t => t.id === tabId);
    
    if (!tab) {
        window.NotificationModule.showNotification('没有可保存的文件', 'warning');
        return;
    }
    
    try {
        const result = await window.AppGlobals.ipcRenderer.invoke('write-file', tab.path, tab.content);
        if (result.success) {
            window.NotificationModule.showNotification(`已保存 ${tab.name}`, 'success');
        } else {
            window.NotificationModule.showNotification(`保存失败: ${result.error}`, 'error');
        }
    } catch (error) {
        window.NotificationModule.showNotification(`保存失败: ${error.message}`, 'error');
    }
}

// 标签切换函数
function switchToNextTab() {
    const allTabs = document.querySelectorAll('.tab');
    if (allTabs.length <= 1) return;
    
    const currentActiveTab = document.querySelector('.tab.active');
    if (!currentActiveTab) return;
    
    // 找到当前标签的索引
    let currentIndex = -1;
    allTabs.forEach((tab, index) => {
        if (tab === currentActiveTab) {
            currentIndex = index;
        }
    });
    
    // 切换到下一个标签（循环）
    const nextIndex = (currentIndex + 1) % allTabs.length;
    const nextTab = allTabs[nextIndex];
    
    if (nextTab && nextTab.id) {
        selectTab(nextTab.id);
    }
}

function switchToPreviousTab() {
    const allTabs = document.querySelectorAll('.tab');
    if (allTabs.length <= 1) return;
    
    const currentActiveTab = document.querySelector('.tab.active');
    if (!currentActiveTab) return;
    
    // 找到当前标签的索引
    let currentIndex = -1;
    allTabs.forEach((tab, index) => {
        if (tab === currentActiveTab) {
            currentIndex = index;
        }
    });
    
    // 切换到上一个标签（循环）
    const prevIndex = currentIndex === 0 ? allTabs.length - 1 : currentIndex - 1;
    const prevTab = allTabs[prevIndex];
    
    if (prevTab && prevTab.id) {
        selectTab(prevTab.id);
    }
}

// 获取指定标签的编辑器实例
function getEditor(tabId) {
    const tabContent = document.getElementById(`content-${tabId}`);
    if (!tabContent) return null;
    
    const editorContainer = tabContent.querySelector('.simple-editor-container');
    if (!editorContainer) return null;
    
    return editorContainer._editor || null;
}

// 获取当前活动编辑器实例
function getActiveEditor() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) return null;
    
    return getEditor(activeTab.id);
}

// 导出函数
window.EditorModule = {
    initializeSimpleEditor,
    createTab,
    selectTab,
    closeTab,
    saveCurrentFile,
    saveCurrentFileWithNotification,
    switchToNextTab,
    switchToPreviousTab,
    getEditor,
    getActiveEditor
};