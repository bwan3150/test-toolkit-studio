// 编辑器管理器 - 负责标签管理和编辑器实例协调

class EditorManager {
    constructor() {
        this.editors = new Map(); // tabId -> EditorTab实例
        this.activeTabId = null;
        this.tabsContainer = null;
        this.editorContainer = null;
        this.globalEditMode = 'block'; // 全局编辑模式，所有 tab 共享
        this.init();
    }
    
    init() {
        this.tabsContainer = document.getElementById('editorTabs');
        this.editorContainer = document.getElementById('editorWorkspace');
        
        if (!this.tabsContainer || !this.editorContainer) {
            window.rError('编辑器容器未找到');
            return;
        }
        
        // 设置全局快捷键监听器
        this.setupGlobalKeyboardShortcuts();
        
        window.rLog('编辑器管理器初始化完成');
    }
    
    setupGlobalKeyboardShortcuts() {
        // 全局监听 Cmd/Ctrl + / 用于切换模式
        this.globalModeToggleHandler = (e) => {
            // 检查是否在 Testcase 页面
            const testcasePage = document.getElementById('testcasePage');
            const isTestcasePageActive = testcasePage && testcasePage.classList.contains('active');
            
            if (!isTestcasePageActive) {
                return;
            }
            
            if (e.key === '/' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                e.stopPropagation();
                e.stopImmediatePropagation();
                
                window.rLog('全局快捷键触发模式切换');
                this.toggleGlobalEditMode();
                return false;
            }
        };
        
        // 使用最高优先级监听
        document.addEventListener('keydown', this.globalModeToggleHandler, true);
    }
    
    // 创建新标签和编辑器实例
    createTab(tab) {
        window.rLog('创建标签:', tab);
        
        if (!this.tabsContainer) {
            window.rError('找不到标签容器');
            return;
        }
        
        // 创建标签DOM元素
        const tabElement = this.createTabElement(tab);
        this.tabsContainer.appendChild(tabElement);
        
        // 为该标签创建编辑器容器
        const editorTabContainer = this.createEditorTabContainer(tab.id);
        
        // 创建编辑器实例，传入管理器引用
        const editorTab = new EditorTab(editorTabContainer, this);
        
        // 新 tab 会自动从管理器读取当前模式
        
        this.editors.set(tab.id, editorTab);
        
        // 设置编辑器文件（使用TKE缓冲区）
        if (tab.filePath) {
            editorTab.setFile(tab.filePath).catch(error => {
                window.rError(`❌ 设置编辑器文件失败: ${error.message}`);
            });
        }
        
        // 设置变更监听器
        editorTab.on('change', (content) => {
            this.handleContentChange(tab.id, content);
        });
        
        window.rLog('标签创建完成:', tab.id);
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
        window.rLog('选择标签:', tabId);
        
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
            window.rLog('已更新 AppGlobals.currentTab:', tabData);
        }
        
        // 触发标签变化事件
        const event = new CustomEvent('tabChanged', { detail: { tabId } });
        document.dispatchEvent(event);
        
        window.rLog('标签选择完成:', tabId);
    }
    
    // 关闭标签
    closeTab(tabId) {
        window.rLog('关闭标签:', tabId);
        
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
        
        // 从 openTabs 数组中移除
        if (window.AppGlobals && window.AppGlobals.openTabs) {
            const tabIndex = window.AppGlobals.openTabs.findIndex(t => t.id === tabId);
            if (tabIndex !== -1) {
                window.AppGlobals.openTabs.splice(tabIndex, 1);
                window.rLog('已从 openTabs 中移除标签:', tabId);
            }
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
        
        window.rLog('标签关闭完成:', tabId);
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
            window.rError('没有活动的编辑器');
            return Promise.reject(new Error('没有活动的编辑器'));
        }
        
        const content = editor.getValue();
        window.rLog(`🔍 获取编辑器内容长度: ${content.length}`);
        
        const tabElement = document.getElementById(this.activeTabId);
        if (!tabElement) {
            window.rError('找不到标签元素');
            return Promise.reject(new Error('找不到标签元素'));
        }
        
        const filePath = tabElement.querySelector('.tab-label').title;
        window.rLog(`💾 准备保存文件: ${filePath}`);
        
        // 通过IPC保存文件
        const { ipcRenderer } = window.AppGlobals;
        return ipcRenderer.invoke('save-file', filePath, content)
            .then((result) => {
                window.rLog(`✅ 文件保存成功: ${filePath}`);
                return result;
            })
            .catch(error => {
                window.rError(`❌ 文件保存失败: ${error.message}`);
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
        window.rLog('EditorManager.setTestRunning 调用:', {
            isRunning,
            clearHighlight,
            activeTabId: this.activeTabId,
            hasActiveEditor: !!activeEditor,
            activeEditorType: activeEditor ? activeEditor.constructor.name : 'null',
            hasSetTestRunning: activeEditor ? typeof activeEditor.setTestRunning : 'no editor'
        });
        
        if (activeEditor) {
            if (typeof activeEditor.setTestRunning === 'function') {
                activeEditor.setTestRunning(isRunning, clearHighlight);
            } else {
                window.rError('activeEditor 没有 setTestRunning 方法!', {
                    editorMethods: Object.getOwnPropertyNames(Object.getPrototypeOf(activeEditor))
                });
            }
        } else {
            window.rWarn('没有活动的编辑器');
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
    
    // 切换编辑模式（全局）
    toggleGlobalEditMode() {
        // 切换全局模式
        this.globalEditMode = this.globalEditMode === 'block' ? 'text' : 'block';
        window.rLog('切换全局编辑模式为:', this.globalEditMode);
        
        // 同步到所有打开的编辑器
        this.editors.forEach((editor, tabId) => {
            if (editor.currentMode !== this.globalEditMode) {
                if (this.globalEditMode === 'text') {
                    editor.switchToTextMode();
                } else {
                    editor.switchToBlockMode();
                }
                window.rLog('同步 tab', tabId, '到模式:', this.globalEditMode);
            }
        });
    }
    
    // 获取当前全局编辑模式
    getGlobalEditMode() {
        return this.globalEditMode;
    }
    
    // 销毁管理器
    destroy() {
        // 移除全局快捷键监听器
        if (this.globalModeToggleHandler) {
            document.removeEventListener('keydown', this.globalModeToggleHandler, true);
        }
        
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
        // 添加防御性检查
        if (window.AppGlobals && typeof window.AppGlobals.setCodeEditor === 'function') {
            window.AppGlobals.setCodeEditor(editorManagerInstance);
        } else {
            window.rError('❌ AppGlobals 未定义或 setCodeEditor 方法不存在', {
                hasAppGlobals: !!window.AppGlobals,
                AppGlobalsType: typeof window.AppGlobals,
                hasSetCodeEditor: window.AppGlobals ? typeof window.AppGlobals.setCodeEditor : 'N/A'
            });
            throw new Error('AppGlobals 未正确初始化');
        }

        window.rLog('编辑器管理器初始化完成');

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