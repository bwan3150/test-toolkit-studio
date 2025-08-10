// 编辑器模块 - 使用UnifiedScriptEditor作为主要编辑器

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
    
    // 创建新的统一编辑器实例
    editorInstance = new UnifiedScriptEditor(container);
    
    // 将编辑器实例存储到容器中，以便后续获取
    container._editor = editorInstance;
    
    // 更新全局变量引用
    window.AppGlobals.setCodeEditor({
        get value() { return editorInstance.getValue(); },
        set value(val) { editorInstance.setValue(val); },
        set placeholder(text) { editorInstance.setPlaceholder(text); },
        focus() { editorInstance.focus(); },
        refreshImageLocators() { /* 统一编辑器内部处理 */ },
        updateFontSettings(fontFamily, fontSize) { /* 统一编辑器内部处理 */ },
        highlightExecutingLine(lineNumber) { 
            if (editorInstance && editorInstance.highlightExecutingLine) {
                editorInstance.highlightExecutingLine(lineNumber);
            }
        },
        highlightErrorLine(lineNumber) { 
            if (editorInstance && editorInstance.highlightErrorLine) {
                editorInstance.highlightErrorLine(lineNumber);
            }
        },
        clearExecutionHighlight() { 
            if (editorInstance && editorInstance.clearExecutionHighlight) {
                editorInstance.clearExecutionHighlight();
            }
        },
        setTestRunning(isRunning, clearHighlight = false) { 
            if (editorInstance && editorInstance.setTestRunning) {
                editorInstance.setTestRunning(isRunning, clearHighlight);
            }
        }
    });
    
    console.log('Unified Editor initialized successfully');
    
    // 初始化后立即加载字体设置
    setTimeout(() => {
        if (window.SettingsModule && window.SettingsModule.loadEditorFontSettings) {
            window.SettingsModule.loadEditorFontSettings();
        }
    }, 100);
}

// 标签管理函数
function createTab(tab) {
    console.log('创建标签:', tab);
    const tabsContainer = document.getElementById('editorTabs');
    
    if (!tabsContainer) {
        console.error('找不到 editorTabs 容器');
        return;
    }
    
    const tabElement = document.createElement('div');
    tabElement.className = 'tab';
    tabElement.id = tab.id;
    tabElement.innerHTML = `
        <span class="tab-icon">
            <svg viewBox="0 0 24 24">
                <path d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 7V3.5L18.5 9H13z"/>
            </svg>
        </span>
        <span class="tab-label" title="${tab.path}">${tab.name}</span>
        <span class="tab-close" onclick="event.stopPropagation(); closeTab('${tab.id}')">×</span>
    `;
    
    tabElement.addEventListener('click', () => selectTab(tab.id));
    
    tabsContainer.appendChild(tabElement);
    console.log('标签创建完成:', tab.id);
}

function selectTab(tabId) {
    console.log('选择标签:', tabId);
    
    // 移除所有标签的active状态
    document.querySelectorAll('.tab').forEach(tab => {
        tab.classList.remove('active');
    });
    
    // 激活指定的标签
    const tab = document.getElementById(tabId);
    if (tab) {
        tab.classList.add('active');
        console.log('标签激活成功:', tabId);
        
        // 获取对应的编辑器实例并设置内容
        const editor = getEditor(tabId);
        if (editor) {
            // 从 AppGlobals.openTabs 中获取文件内容
            const tabData = window.AppGlobals.openTabs.find(t => t.id === tabId);
            if (tabData) {
                console.log('设置编辑器内容，长度:', tabData.content ? tabData.content.length : 0);
                if (tabData.content) {
                    editor.setValue(tabData.content);
                }
                
                // 更新 AppGlobals.currentTab 以便 Run Test 能检测到当前打开的脚本
                window.AppGlobals.currentTab = tabData;
                console.log('已更新 AppGlobals.currentTab:', tabData);
            }
            
            if (editor.focus) {
                setTimeout(() => editor.focus(), 100);
            }
        }
        
        // 触发标签变化事件
        const event = new CustomEvent('tabChanged', { detail: { tabId } });
        document.dispatchEvent(event);
    } else {
        console.error('找不到标签元素:', tabId);
    }
}

function closeTab(tabId) {
    const tab = document.getElementById(tabId);
    if (!tab) return;
    
    const wasActive = tab.classList.contains('active');
    
    // 移除标签元素
    tab.remove();
    
    // 移除对应的编辑器实例
    const editor = getEditor(tabId);
    if (editor && editor.destroy) {
        editor.destroy();
    }
    
    // 清理编辑器实例引用
    if (window.editorInstances) {
        delete window.editorInstances[tabId];
    }
    
    // 如果关闭的是活动标签，切换到其他标签
    if (wasActive) {
        const remainingTabs = document.querySelectorAll('.tab');
        if (remainingTabs.length > 0) {
            const lastTab = remainingTabs[remainingTabs.length - 1];
            selectTab(lastTab.id);
        }
    }
}

function saveCurrentFile() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) return;
    
    const editor = getEditor(activeTab.id);
    if (editor && editor.getValue) {
        const content = editor.getValue();
        
        // 通过IPC保存文件
        const { ipcRenderer } = window.AppGlobals;
        const filePath = activeTab.querySelector('.tab-label').title;
        
        ipcRenderer.invoke('save-file', filePath, content)
            .then(() => {
                console.log('File saved successfully');
            })
            .catch(error => {
                console.error('Failed to save file:', error);
            });
    }
}

function saveCurrentFileWithNotification() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) {
        window.NotificationModule?.showNotification('没有打开的文件', 'warning');
        return;
    }
    
    const editor = getEditor(activeTab.id);
    if (editor && editor.getValue) {
        const content = editor.getValue();
        const { ipcRenderer } = window.AppGlobals;
        const filePath = activeTab.querySelector('.tab-label').title;
        
        ipcRenderer.invoke('save-file', filePath, content)
            .then(() => {
                window.NotificationModule?.showNotification('文件保存成功', 'success');
            })
            .catch(error => {
                console.error('Failed to save file:', error);
                window.NotificationModule?.showNotification(`保存失败: ${error.message}`, 'error');
            });
    }
}

function switchToNextTab() {
    const tabs = document.querySelectorAll('.tab');
    const activeTab = document.querySelector('.tab.active');
    
    if (!activeTab || tabs.length <= 1) return;
    
    const currentIndex = Array.from(tabs).indexOf(activeTab);
    const nextIndex = (currentIndex + 1) % tabs.length;
    
    selectTab(tabs[nextIndex].id);
}

function switchToPreviousTab() {
    const tabs = document.querySelectorAll('.tab');
    const activeTab = document.querySelector('.tab.active');
    
    if (!activeTab || tabs.length <= 1) return;
    
    const currentIndex = Array.from(tabs).indexOf(activeTab);
    const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    
    selectTab(tabs[prevIndex].id);
}

function getEditor(tabId) {
    // 我们只使用一个全局编辑器实例，所有标签共享
    return editorInstance;
}

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