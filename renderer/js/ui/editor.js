// 新编辑器模块 - 基于ContentEditable的单层实现

class SimpleCodeEditor {
    constructor(container) {
        this.container = container;
        this.value = '';
        this.listeners = [];
        this.saveTimeout = null;
        
        this.createEditor();
        this.setupEventListeners();
    }
    
    createEditor() {
        this.container.innerHTML = `
            <div class="editor-wrapper-new">
                <div class="editor-gutter">
                    <div class="line-numbers-new" id="lineNumbersNew"></div>
                </div>
                <div class="editor-main">
                    <div class="editor-content" id="editorContent" contenteditable="true"></div>
                </div>
            </div>
        `;
        
        this.lineNumbersEl = document.getElementById('lineNumbersNew');
        this.contentEl = document.getElementById('editorContent');
        
        
        // 设置ContentEditable样式
        const fontSettings = {
            fontFamily: 'Consolas, "Courier New", monospace',
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
        
        this.setPlaceholder('请在Project页面选择测试项创建Case后, 在左侧文件树选择对应YAML文件开始编辑自动化脚本');
    }
    
    setupEventListeners() {
        // 处理输入事件
        this.contentEl.addEventListener('input', (e) => {
            this.updateValue();
            this.updateLineNumbers();
            this.applySyntaxHighlighting();
            this.triggerChange();
        });
        
        // 处理按键事件
        this.contentEl.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                this.insertText('  '); // 插入2个空格
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
            if (e.target.classList.contains('inline-image-locator')) {
                e.preventDefault();
                this.handleImageLocatorClick(e.target);
            }
        });
    }
    
    insertText(text) {
        const selection = window.getSelection();
        const range = selection.getRangeAt(0);
        
        range.deleteContents();
        range.insertNode(document.createTextNode(text));
        
        // 移动光标到插入文本后
        range.setStartAfter(range.endContainer);
        range.collapse(true);
        selection.removeAllRanges();
        selection.addRange(range);
        
        this.updateValue();
        this.updateLineNumbers();
        this.applySyntaxHighlighting();
        this.triggerChange();
    }
    
    updateValue() {
        // 从ContentEditable获取纯文本值
        this.value = this.getPlainText();
    }
    
    getPlainText() {
        // 从ContentEditable提取纯文本
        return this.contentEl.textContent || '';
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
        if (!this.value) {
            this.showPlaceholder();
            return;
        }
        
        this.hidePlaceholder();
        
        // 应用语法高亮到ContentEditable
        const highlightedHtml = this.highlightSyntax(this.value);
        
        // 保存当前光标位置
        const selection = window.getSelection();
        const range = selection.rangeCount > 0 ? selection.getRangeAt(0) : null;
        const cursorOffset = range ? this.getTextOffset(range.startContainer, range.startOffset) : 0;
        
        // 设置高亮内容
        this.contentEl.innerHTML = highlightedHtml;
        
        // 恢复光标位置
        if (cursorOffset > 0) {
            this.restoreCursorPosition(cursorOffset);
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
        if (offset <= 0) return;
        
        const walker = document.createTreeWalker(
            this.contentEl,
            NodeFilter.SHOW_TEXT,
            null,
            false
        );
        
        let currentOffset = 0;
        let currentNode;
        
        while (currentNode = walker.nextNode()) {
            const nodeLength = currentNode.textContent.length;
            if (currentOffset + nodeLength >= offset) {
                // 找到目标节点
                const range = document.createRange();
                const selection = window.getSelection();
                
                range.setStart(currentNode, offset - currentOffset);
                range.collapse(true);
                
                selection.removeAllRanges();
                selection.addRange(range);
                return;
            }
            currentOffset += nodeLength;
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
        // 检查是否正在编辑这个定位器
        if (this.currentEditingLocator && this.currentEditingLocator.name === imageName) {
            // 如果正在编辑，显示文本形式
            return `@{<span class="syntax-string">${imageName}</span>}`;
        }
        
        // 获取当前项目路径
        const projectPath = window.AppGlobals.currentProject;
        if (!projectPath) {
            // 如果没有项目，显示原始文本
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
                    return `<img class="inline-image-locator" src="${imagePath}" alt="${imageName}" data-name="${imageName}" title="点击编辑: ${fallbackText}" onerror="this.outerHTML='<span class=&quot;syntax-string&quot;>${fallbackText}</span>';">`;
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
        if (!this.value) {
            this.showPlaceholder();
        }
    }
    
    showPlaceholder() {
        if (!this.value) {
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
        this.contentEl.focus();
    }
    
    // 处理图片定位器点击
    handleImageLocatorClick(imageElement) {
        const imageName = imageElement.dataset.name;
        if (!imageName) return;
        
        // 将图片替换为文本形式，并设置光标
        const textToInsert = `@{${imageName}}`;
        
        // 创建文本节点替换图片
        const textNode = document.createTextNode(textToInsert);
        imageElement.parentNode.replaceChild(textNode, imageElement);
        
        // 设置光标到文本末尾
        const range = document.createRange();
        const selection = window.getSelection();
        
        range.setStart(textNode, textToInsert.length);
        range.collapse(true);
        
        selection.removeAllRanges();
        selection.addRange(range);
        
        this.updateValue();
    }
    
    // 刷新图片定位器渲染
    refreshImageLocators() {
        this.applySyntaxHighlighting();
    }
    
    destroy() {
        clearTimeout(this.saveTimeout);
        this.listeners = [];
    }
    
    // 高亮当前执行的行
    highlightExecutingLine(lineNumber) {
        console.log(`编辑器: 高亮执行行 ${lineNumber}`);
        this.currentExecutingLine = lineNumber;
        this.currentErrorLine = null; // 清除错误行高亮
        this.applySyntaxHighlightingWithExecution();
    }
    
    // 高亮错误行
    highlightErrorLine(lineNumber) {
        console.log(`编辑器: 高亮错误行 ${lineNumber}`);
        this.currentErrorLine = lineNumber;
        this.currentExecutingLine = null; // 清除执行行高亮
        this.applySyntaxHighlightingWithExecution();
    }
    
    // 清除执行行高亮
    clearExecutionHighlight() {
        console.log('编辑器: 清除执行行高亮');
        this.currentExecutingLine = null;
        this.currentErrorLine = null;
        this.applySyntaxHighlighting();
    }
    
    // 修改语法高亮应用方法以支持执行行高亮
    applySyntaxHighlightingWithExecution() {
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
        
        this.highlightEl.innerHTML = highlightedLines.join('\n');
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
        highlightExecutingLine(lineNumber) { editorInstance.highlightExecutingLine(lineNumber); },
        highlightErrorLine(lineNumber) { editorInstance.highlightErrorLine(lineNumber); },
        clearExecutionHighlight() { editorInstance.clearExecutionHighlight(); }
    });
    
    console.log('New Simple Editor initialized successfully');
}

// 更新编辑器（兼容旧接口）
function updateEditor() {
    if (editorInstance) {
        editorInstance.applySyntaxHighlighting();
    }
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
                window.AppGlobals.codeEditor.placeholder = '请在Project页面选择测试项创建Case后, 在左侧文件树选择对应YAML文件开始编辑自动化脚本';
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
    updateEditor,
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