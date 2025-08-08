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
                    <textarea class="editor-textarea" id="editorTextarea"></textarea>
                    <div class="editor-highlight-layer" id="editorHighlight"></div>
                </div>
            </div>
        `;
        
        this.lineNumbersEl = document.getElementById('lineNumbersNew');
        this.textareaEl = document.getElementById('editorTextarea');
        this.highlightEl = document.getElementById('editorHighlight');
        
        
        // 设置textarea样式
        this.textareaEl.style.fontFamily = '"SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace';
        this.textareaEl.style.fontSize = '14px';
        this.textareaEl.style.lineHeight = '1.5';
        this.textareaEl.style.padding = '16px';
        this.textareaEl.style.margin = '0';
        this.textareaEl.style.border = 'none';
        this.textareaEl.style.outline = 'none';
        this.textareaEl.style.resize = 'none';
        this.textareaEl.style.whiteSpace = 'pre';
        this.textareaEl.style.overflowWrap = 'normal';
        this.textareaEl.style.color = 'transparent';
        this.textareaEl.style.background = 'transparent';
        this.textareaEl.style.caretColor = '#ffffff';
        this.textareaEl.style.position = 'absolute';
        this.textareaEl.style.top = '0';
        this.textareaEl.style.left = '0';
        this.textareaEl.style.width = '100%';
        this.textareaEl.style.height = '100%';
        this.textareaEl.style.zIndex = '2';
        
        // 设置高亮层样式
        this.highlightEl.style.fontFamily = '"SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace';
        this.highlightEl.style.fontSize = '14px';
        this.highlightEl.style.lineHeight = '1.5';
        this.highlightEl.style.padding = '16px';
        this.highlightEl.style.margin = '0';
        this.highlightEl.style.border = 'none';
        this.highlightEl.style.whiteSpace = 'pre';
        this.highlightEl.style.overflowWrap = 'normal';
        this.highlightEl.style.color = '#d4d4d4';
        this.highlightEl.style.background = 'transparent';
        this.highlightEl.style.position = 'absolute';
        this.highlightEl.style.top = '0';
        this.highlightEl.style.left = '0';
        this.highlightEl.style.width = '100%';
        this.highlightEl.style.height = '100%';
        this.highlightEl.style.pointerEvents = 'none';
        this.highlightEl.style.zIndex = '1';
        
        this.setPlaceholder('请在Project页面选择测试项创建Case后, 在左侧文件树选择对应YAML文件开始编辑自动化脚本');
    }
    
    setupEventListeners() {
        // 处理输入事件
        this.textareaEl.addEventListener('input', (e) => {
            this.updateValue();
            this.updateLineNumbers();
            this.applySyntaxHighlighting();
            this.triggerChange();
        });
        
        // 处理按键事件
        this.textareaEl.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                this.insertText('  '); // 插入2个空格
            }
        });
        
        // 处理滚动同步
        this.textareaEl.addEventListener('scroll', () => {
            if (this.lineNumbersEl) {
                this.lineNumbersEl.scrollTop = this.textareaEl.scrollTop;
            }
            if (this.highlightEl) {
                this.highlightEl.scrollTop = this.textareaEl.scrollTop;
                this.highlightEl.scrollLeft = this.textareaEl.scrollLeft;
            }
        });
    }
    
    insertText(text) {
        const start = this.textareaEl.selectionStart;
        const end = this.textareaEl.selectionEnd;
        
        const value = this.textareaEl.value;
        this.textareaEl.value = value.substring(0, start) + text + value.substring(end);
        
        // 设置光标位置
        const newPos = start + text.length;
        this.textareaEl.selectionStart = newPos;
        this.textareaEl.selectionEnd = newPos;
        
        this.updateValue();
        this.updateLineNumbers();
        this.applySyntaxHighlighting();
        this.triggerChange();
    }
    
    updateValue() {
        // 获取textarea的值
        this.value = this.textareaEl.value;
    }
    
    getPlainText() {
        // 返回textarea的值
        return this.textareaEl.value;
    }
    
    setValue(text) {
        this.value = text || '';
        this.textareaEl.value = this.value;
        
        if (!this.value) {
            this.showPlaceholder();
        } else {
            this.hidePlaceholder();
            this.applySyntaxHighlighting();
        }
        
        this.updateLineNumbers();
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
        
        // 应用语法高亮到高亮层
        const highlightedHtml = this.highlightSyntax(this.value);
        this.highlightEl.innerHTML = highlightedHtml;
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
        
        // 4. 方括号内容 [坐标], [元素], [时间] 等
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
        
        // 5. 存在、不存在等断言关键词
        const assertionKeywords = ['存在', '不存在', '显示', '隐藏', '包含', '不包含', '等于', '不等于'];
        assertionKeywords.forEach(keyword => {
            const regex = new RegExp('\\b(' + keyword + ')\\b', 'g');
            result = result.replace(regex, '<span class="syntax-assertion">$1</span>');
        });
        
        // 6. 时间格式 (单独出现的)
        result = result.replace(/\b(\d+)(s|ms|秒|毫秒)\b/g, '<span class="syntax-number">$1</span><span class="syntax-unit">$2</span>');
        
        // 7. 包名格式
        result = result.replace(/\b(com\.[a-zA-Z0-9_.]+)\b/g, '<span class="syntax-package">$1</span>');
        
        // 8. 方向关键词
        const directions = ['上', '下', '左', '右', '向上', '向下', '向左', '向右'];
        directions.forEach(direction => {
            const regex = new RegExp('\\b(' + direction + ')\\b', 'g');
            result = result.replace(regex, '<span class="syntax-direction">$1</span>');
        });
        
        return result;
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
        this.textareaEl.placeholder = text;
        if (!this.value) {
            this.showPlaceholder();
        }
    }
    
    showPlaceholder() {
        if (!this.value) {
            this.highlightEl.innerHTML = `<span class="editor-placeholder">${this.placeholderText}</span>`;
        }
    }
    
    hidePlaceholder() {
        const placeholder = this.highlightEl.querySelector('.editor-placeholder');
        if (placeholder) {
            this.highlightEl.innerHTML = '';
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
        this.textareaEl.focus();
    }
    
    destroy() {
        clearTimeout(this.saveTimeout);
        this.listeners = [];
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
        focus() { editorInstance.focus(); }
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