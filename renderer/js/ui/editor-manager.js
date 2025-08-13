// 编辑器管理器 - 负责标签管理和编辑器实例协调

class EditorManager {
    constructor() {
        this.editors = new Map(); // tabId -> EditorTab实例
        this.activeTabId = null;
        this.tabsContainer = null;
        this.editorContainer = null;
        this.init();
    }
    
    init() {
        this.tabsContainer = document.getElementById('editorTabs');
        this.editorContainer = document.getElementById('editorWorkspace');
        
        if (!this.tabsContainer || !this.editorContainer) {
            console.error('编辑器容器未找到');
            return;
        }
        
        console.log('编辑器管理器初始化完成');
    }
    
    // 创建新标签和编辑器实例
    createTab(tab) {
        console.log('创建标签:', tab);
        
        if (!this.tabsContainer) {
            console.error('找不到标签容器');
            return;
        }
        
        // 创建标签DOM元素
        const tabElement = this.createTabElement(tab);
        this.tabsContainer.appendChild(tabElement);
        
        // 为该标签创建编辑器容器
        const editorTabContainer = this.createEditorTabContainer(tab.id);
        
        // 创建编辑器实例
        const editorTab = new EditorTab(editorTabContainer);
        this.editors.set(tab.id, editorTab);
        
        // 设置编辑器内容
        if (tab.content) {
            editorTab.setValue(tab.content);
        }
        
        // 设置变更监听器
        editorTab.on('change', (content) => {
            this.handleContentChange(tab.id, content);
        });
        
        console.log('标签创建完成:', tab.id);
    }
    
    // 创建标签DOM元素
    createTabElement(tab) {
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
            <span class="tab-close" onclick="event.stopPropagation(); window.EditorManager.closeTab('${tab.id}')">×</span>
        `;
        
        tabElement.addEventListener('click', () => this.selectTab(tab.id));
        return tabElement;
    }
    
    // 创建编辑器标签容器
    createEditorTabContainer(tabId) {
        const container = document.createElement('div');
        container.className = 'editor-tab-container';
        container.id = `editor-${tabId}`;
        container.style.display = 'none'; // 默认隐藏
        
        this.editorContainer.appendChild(container);
        return container;
    }
    
    // 选择标签
    selectTab(tabId) {
        console.log('选择标签:', tabId);
        
        // 隐藏所有编辑器容器
        this.editorContainer.querySelectorAll('.editor-tab-container').forEach(container => {
            container.style.display = 'none';
        });
        
        // 移除所有标签的active状态
        document.querySelectorAll('.tab').forEach(tab => {
            tab.classList.remove('active');
        });
        
        // 激活指定标签
        const tabElement = document.getElementById(tabId);
        if (tabElement) {
            tabElement.classList.add('active');
        }
        
        // 显示对应的编辑器容器
        const editorContainer = document.getElementById(`editor-${tabId}`);
        if (editorContainer) {
            editorContainer.style.display = 'block';
        }
        
        // 更新活动标签ID
        this.activeTabId = tabId;
        
        // 获取编辑器实例并聚焦
        const editor = this.editors.get(tabId);
        if (editor) {
            setTimeout(() => editor.focus(), 100);
        }
        
        // 更新全局当前标签引用
        const tabData = window.AppGlobals.openTabs.find(t => t.id === tabId);
        if (tabData) {
            window.AppGlobals.currentTab = tabData;
            console.log('已更新 AppGlobals.currentTab:', tabData);
        }
        
        // 触发标签变化事件
        const event = new CustomEvent('tabChanged', { detail: { tabId } });
        document.dispatchEvent(event);
        
        console.log('标签选择完成:', tabId);
    }
    
    // 关闭标签
    closeTab(tabId) {
        console.log('关闭标签:', tabId);
        
        const tabElement = document.getElementById(tabId);
        if (!tabElement) return;
        
        const wasActive = tabElement.classList.contains('active');
        
        // 移除标签元素
        tabElement.remove();
        
        // 销毁编辑器实例
        const editor = this.editors.get(tabId);
        if (editor) {
            editor.destroy();
            this.editors.delete(tabId);
        }
        
        // 移除编辑器容器
        const editorContainer = document.getElementById(`editor-${tabId}`);
        if (editorContainer) {
            editorContainer.remove();
        }
        
        // 如果关闭的是活动标签，切换到其他标签
        if (wasActive) {
            const remainingTabs = document.querySelectorAll('.tab');
            if (remainingTabs.length > 0) {
                const lastTab = remainingTabs[remainingTabs.length - 1];
                this.selectTab(lastTab.id);
            } else {
                this.activeTabId = null;
                window.AppGlobals.currentTab = null;
            }
        }
        
        console.log('标签关闭完成:', tabId);
    }
    
    // 获取指定标签的编辑器
    getEditor(tabId) {
        return this.editors.get(tabId);
    }
    
    // 获取当前活动编辑器
    getActiveEditor() {
        return this.activeTabId ? this.editors.get(this.activeTabId) : null;
    }
    
    // 保存当前文件
    saveCurrentFile() {
        const editor = this.getActiveEditor();
        if (!editor || !this.activeTabId) {
            console.warn('没有活动的编辑器');
            return;
        }
        
        const content = editor.getValue();
        const tabElement = document.getElementById(this.activeTabId);
        if (!tabElement) return;
        
        const filePath = tabElement.querySelector('.tab-label').title;
        
        // 通过IPC保存文件
        const { ipcRenderer } = window.AppGlobals;
        return ipcRenderer.invoke('save-file', filePath, content)
            .then(() => {
                console.log('文件保存成功:', filePath);
            })
            .catch(error => {
                console.error('文件保存失败:', error);
                throw error;
            });
    }
    
    // 带通知的保存当前文件
    saveCurrentFileWithNotification() {
        if (!this.activeTabId) {
            window.NotificationModule?.showNotification('没有打开的文件', 'warning');
            return Promise.reject(new Error('没有打开的文件'));
        }
        
        return this.saveCurrentFile()
            .then(() => {
                window.NotificationModule?.showNotification('文件保存成功', 'success');
            })
            .catch(error => {
                window.NotificationModule?.showNotification(`保存失败: ${error.message}`, 'error');
                throw error;
            });
    }
    
    // 切换到下一个标签
    switchToNextTab() {
        const tabs = document.querySelectorAll('.tab');
        const activeTab = document.querySelector('.tab.active');
        
        if (!activeTab || tabs.length <= 1) return;
        
        const currentIndex = Array.from(tabs).indexOf(activeTab);
        const nextIndex = (currentIndex + 1) % tabs.length;
        
        this.selectTab(tabs[nextIndex].id);
    }
    
    // 切换到上一个标签
    switchToPreviousTab() {
        const tabs = document.querySelectorAll('.tab');
        const activeTab = document.querySelector('.tab.active');
        
        if (!activeTab || tabs.length <= 1) return;
        
        const currentIndex = Array.from(tabs).indexOf(activeTab);
        const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
        
        this.selectTab(tabs[prevIndex].id);
    }
    
    // 处理内容变更
    handleContentChange(tabId, content) {
        // 更新AppGlobals中的标签内容
        const tabData = window.AppGlobals.openTabs.find(t => t.id === tabId);
        if (tabData) {
            tabData.content = content;
        }
    }
    
    // 设置测试运行状态
    setTestRunning(isRunning, clearHighlight = false) {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.setTestRunning(isRunning, clearHighlight);
        }
    }
    
    // 高亮执行行
    highlightExecutingLine(lineNumber) {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.highlightExecutingLine(lineNumber);
        }
    }
    
    // 高亮错误行
    highlightErrorLine(lineNumber) {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.highlightErrorLine(lineNumber);
        }
    }
    
    // 清除执行高亮
    clearExecutionHighlight() {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.clearExecutionHighlight();
        }
    }
    
    // 更新字体设置
    updateFontSettings(fontFamily, fontSize) {
        this.editors.forEach(editor => {
            if (editor.updateFontSettings) {
                editor.updateFontSettings(fontFamily, fontSize);
            }
        });
    }
    
    // 兼容性属性 - 支持AppGlobals.codeEditor的使用方式
    get value() {
        const editor = this.getActiveEditor();
        return editor ? editor.getValue() : '';
    }
    
    set value(val) {
        const editor = this.getActiveEditor();
        if (editor) editor.setValue(val);
    }
    
    set placeholder(text) {
        const editor = this.getActiveEditor();
        if (editor) editor.setPlaceholder(text);
    }
    
    focus() {
        const editor = this.getActiveEditor();
        if (editor) editor.focus();
    }
    
    // 刷新图片定位器
    refreshImageLocators() {
        const editor = this.getActiveEditor();
        if (editor && editor.refreshImageLocators) {
            editor.refreshImageLocators();
        }
    }
    
    // 获取内容元素（用于blur等操作）
    get contentEl() {
        const editor = this.getActiveEditor();
        return editor ? editor.textContentEl : null;
    }
    
    // 销毁管理器
    destroy() {
        this.editors.forEach(editor => editor.destroy());
        this.editors.clear();
        this.activeTabId = null;
    }
}

// 创建全局实例
let editorManagerInstance = null;

// 初始化编辑器管理器
function initializeEditorManager() {
    if (!editorManagerInstance) {
        editorManagerInstance = new EditorManager();
        
        // 全局引用
        window.EditorManager = editorManagerInstance;
        
        // 更新全局AppGlobals的编辑器引用 - 直接使用EditorManager
        window.AppGlobals.setCodeEditor(editorManagerInstance);
        
        console.log('编辑器管理器初始化完成');
        
        // 初始化后立即加载字体设置
        setTimeout(() => {
            if (window.SettingsModule && window.SettingsModule.loadEditorFontSettings) {
                window.SettingsModule.loadEditorFontSettings();
            }
        }, 100);
    }
    
    return editorManagerInstance;
}

// 导出初始化函数
window.initializeEditorManager = initializeEditorManager;