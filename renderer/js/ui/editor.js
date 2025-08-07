// 编辑器模块

// 初始化简单编辑器
function initializeSimpleEditor() {
    const codeEditor = document.getElementById('codeEditor');
    const lineNumbers = document.getElementById('lineNumbers');
    const syntaxHighlight = document.getElementById('syntaxHighlight');
    
    // 更新全局变量
    window.AppGlobals.setCodeEditor(codeEditor);
    window.AppGlobals.setLineNumbers(lineNumbers);
    window.AppGlobals.setSyntaxHighlight(syntaxHighlight);
    
    if (codeEditor) {
        console.log('Simple Editor initialized successfully');
        
        // 设置初始内容为空
        codeEditor.value = '';
        codeEditor.placeholder = '请在Project页面选择测试项创建Case后, 在左侧文件树选择对应YAML文件开始编辑自动化脚本';
        updateEditor();
        
        // 保存时防抖
        let saveTimeout;
        codeEditor.addEventListener('input', () => {
            updateEditor();
            
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                const tabId = activeTab.id;
                const tab = window.AppGlobals.openTabs.find(t => t.id === tabId);
                if (tab) {
                    tab.content = codeEditor.value;
                    
                    // 延迟自动保存，避免频繁保存
                    clearTimeout(saveTimeout);
                    saveTimeout = setTimeout(() => {
                        saveCurrentFile();
                    }, 1000);
                }
            }
        });
        
        // 同步滚动
        codeEditor.addEventListener('scroll', () => {
            if (lineNumbers) {
                lineNumbers.scrollTop = codeEditor.scrollTop;
            }
            if (syntaxHighlight) {
                syntaxHighlight.scrollTop = codeEditor.scrollTop;
                syntaxHighlight.scrollLeft = codeEditor.scrollLeft;
            }
        });
        
        // 处理Tab键缩进
        codeEditor.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                const start = codeEditor.selectionStart;
                const end = codeEditor.selectionEnd;
                
                // 插入2个空格而不是tab
                const value = codeEditor.value;
                codeEditor.value = value.substring(0, start) + '  ' + value.substring(end);
                codeEditor.selectionStart = codeEditor.selectionEnd = start + 2;
                
                updateEditor();
            }
        });
    }
}

// 更新编辑器（行号和语法高亮）
function updateEditor() {
    const codeEditor = window.AppGlobals.codeEditor;
    const lineNumbers = window.AppGlobals.lineNumbers;
    const syntaxHighlight = window.AppGlobals.syntaxHighlight;
    
    if (!codeEditor) return;
    
    const content = codeEditor.value;
    const lines = content.split('\n');
    
    // 更新行号
    if (lineNumbers) {
        const lineNumbersContent = lines.map((_, index) => (index + 1).toString()).join('\n');
        lineNumbers.textContent = lineNumbersContent;
    }
    
    // 更新语法高亮
    if (syntaxHighlight) {
        const highlightedContent = highlightYAML(content);
        syntaxHighlight.innerHTML = highlightedContent;
    }
}

// YAML语法高亮函数
function highlightYAML(text) {
    if (!text) return '';
    
    // HTML转义函数
    function escapeHtml(unsafe) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }
    
    // 转义HTML
    let escaped = escapeHtml(text);
    
    // YAML语法高亮规则
    const rules = [
        // 注释
        {
            pattern: /(^|\n)([ ]*)(#.*?)(?=\n|$)/g,
            replacement: '$1$2<span class="yaml-comment">$3</span>'
        },
        // 键名 (key:)
        {
            pattern: /^([ ]*)([\w\-_]+)(:)[ ]*/gm,
            replacement: '$1<span class="yaml-key">$2</span><span class="yaml-key">$3</span> '
        },
        // 数组标识符 (-)
        {
            pattern: /^([ ]*)(-)([ ]+)/gm,
            replacement: '$1<span class="yaml-dash">$2</span>$3'
        },
        // 字符串值 (引号包围)
        {
            pattern: /(['"])((?:\\.|(?!\1)[^\\])*?)\1/g,
            replacement: '<span class="yaml-string">$1$2$1</span>'
        },
        // 布尔值
        {
            pattern: /\b(true|false|True|False|TRUE|FALSE)\b/g,
            replacement: '<span class="yaml-boolean">$1</span>'
        },
        // null值
        {
            pattern: /\b(null|Null|NULL|~)\b/g,
            replacement: '<span class="yaml-null">$1</span>'
        },
        // 数字
        {
            pattern: /\b(\d+(?:\.\d+)?)\b/g,
            replacement: '<span class="yaml-number">$1</span>'
        },
        // 简单字符串值（不在引号中的值）
        {
            pattern: /:[ ]+([^'"\s][^\n]*?)(?=\n|$)/g,
            replacement: ': <span class="yaml-string">$1</span>'
        }
    ];
    
    // 应用所有规则
    let highlighted = escaped;
    rules.forEach(rule => {
        highlighted = highlighted.replace(rule.pattern, rule.replacement);
    });
    
    return highlighted;
}

// 创建标签
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

// 选择标签
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
        
        // 本地编辑器已准备就绪
        
        // 为选中的标签加载内容
        const tabData = window.AppGlobals.openTabs.find(tab => tab.id === tabId);
        if (tabData && window.AppGlobals.codeEditor) {
            console.log('Setting content for tab:', tabData.name, 'Content length:', tabData.content.length);
            window.AppGlobals.codeEditor.value = tabData.content;
            window.AppGlobals.codeEditor.placeholder = `正在编辑: ${tabData.name}`;
            updateEditor();
        }
    }
}

// 关闭标签
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
                updateEditor();
            }
        }
    }
}

// 保存当前文件
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

// 带通知保存当前文件
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
        const result = await ipcRenderer.invoke('write-file', tab.path, tab.content);
        if (result.success) {
            window.NotificationModule.showNotification(`已保存 ${tab.name}`, 'success');
        } else {
            window.NotificationModule.showNotification(`保存失败: ${result.error}`, 'error');
        }
    } catch (error) {
        window.NotificationModule.showNotification(`保存失败: ${error.message}`, 'error');
    }
}

// 切换到下一个标签
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

// 切换到上一个标签
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

// 导出函数
window.EditorModule = {
    initializeSimpleEditor,
    updateEditor,
    highlightYAML,
    createTab,
    selectTab,
    closeTab,
    saveCurrentFile,
    saveCurrentFileWithNotification,
    switchToNextTab,
    switchToPreviousTab
};