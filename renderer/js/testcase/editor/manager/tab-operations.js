// 标签操作模块
const TabOperations = {
    /**
     * 创建新标签和编辑器实例
     * @param {Object} tab - 标签数据
     */
    async createTab(tab) {
        window.rLog('创建标签:', tab);

        // 检查 EditorTab 是否可用
        if (typeof EditorTab === 'undefined' || !window.EditorTab) {
            window.rError('❌ EditorTab 类未定义！请检查 editor-tab.js 是否正确加载');
            window.rError('window.EditorTab:', typeof window.EditorTab);
            window.rError('EditorTab:', typeof EditorTab);
            throw new Error('EditorTab is not defined');
        }

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
            try {
                await editorTab.setFile(tab.filePath);
            } catch (error) {
                window.rError(`❌ 设置编辑器文件失败:`, error);
                // 即使设置文件失败，也继续创建标签（显示空编辑器）
            }
        }

        // 设置变更监听器
        editorTab.on('change', (content) => {
            this.handleContentChange(tab.id, content);
        });

        window.rLog('标签创建完成:', tab.id);
    },

    /**
     * 创建标签DOM元素
     * @param {Object} tab - 标签数据
     * @returns {HTMLElement} 标签元素
     */
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
    },

    /**
     * 创建编辑器标签容器
     * @param {string} tabId - 标签ID
     * @returns {HTMLElement} 编辑器容器
     */
    createEditorTabContainer(tabId) {
        const container = document.createElement('div');
        container.className = 'editor-tab-container';
        container.id = `editor-${tabId}`;
        container.style.display = 'none'; // 默认隐藏

        this.editorContainer.appendChild(container);
        return container;
    },

    /**
     * 选择标签
     * @param {string} tabId - 标签ID
     */
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
    },

    /**
     * 关闭标签
     * @param {string} tabId - 标签ID
     */
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
    },

    /**
     * 切换到下一个标签
     */
    switchToNextTab() {
        const tabs = document.querySelectorAll('.tab');
        const activeTab = document.querySelector('.tab.active');

        if (!activeTab || tabs.length <= 1) return;

        const currentIndex = Array.from(tabs).indexOf(activeTab);
        const nextIndex = (currentIndex + 1) % tabs.length;

        this.selectTab(tabs[nextIndex].id);
    },

    /**
     * 切换到上一个标签
     */
    switchToPreviousTab() {
        const tabs = document.querySelectorAll('.tab');
        const activeTab = document.querySelector('.tab.active');

        if (!activeTab || tabs.length <= 1) return;

        const currentIndex = Array.from(tabs).indexOf(activeTab);
        const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;

        this.selectTab(tabs[prevIndex].id);
    },

    /**
     * 保存当前文件
     * @returns {Promise} 保存结果
     */
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
    },

    /**
     * 带通知的保存当前文件
     * @returns {Promise} 保存结果
     */
    saveCurrentFileWithNotification() {
        if (!this.activeTabId) {
            window.AppNotifications?.warn('没有打开的文件');
            return Promise.reject(new Error('没有打开的文件'));
        }

        return this.saveCurrentFile()
            .then(() => {
                window.AppNotifications?.success('文件保存成功');
            })
            .catch(error => {
                window.AppNotifications?.error(`保存失败: ${error.message}`);
                throw error;
            });
    },

    /**
     * 处理内容变更
     * @param {string} tabId - 标签ID
     * @param {string} content - 内容
     */
    handleContentChange(tabId, content) {
        // 更新AppGlobals中的标签内容
        const tabData = window.AppGlobals.openTabs.find(t => t.id === tabId);
        if (tabData) {
            tabData.content = content;
        }
    }
};

// 导出到全局
window.TabOperations = TabOperations;

if (window.rLog) {
    window.rLog('✅ TabOperations 模块已加载');
}
