/**
 * CodeJar 编辑器管理器
 * 管理多个编辑器标签页
 */
(window.rLog || console.log)('🔵 codejar-manager.js 开始加载');

class CodeJarManager {
    constructor() {
        this.tabs = new Map(); // tabId -> { editor, filePath, name }
        this.activeTabId = null;
        this.tabsContainer = null;
        this.editorContainer = null;

        window.rLog('📚 CodeJarManager 创建');
    }

    /**
     * 初始化管理器
     */
    init() {
        this.tabsContainer = document.getElementById('editorTabs');
        this.editorContainer = document.getElementById('editorWorkspace');

        if (!this.tabsContainer || !this.editorContainer) {
            window.rError('❌ 找不到tabs或editor容器', {
                tabsContainer: this.tabsContainer,
                editorContainer: this.editorContainer
            });
            return;
        }

        window.rLog('✅ CodeJarManager 初始化完成', {
            tabsContainer: this.tabsContainer.id,
            editorContainer: this.editorContainer.id
        });
    }

    /**
     * 打开文件
     */
    async openFile(filePath) {
        window.rLog(`📂 打开文件: ${filePath}`);

        // 检查是否已打开
        for (const [tabId, tab] of this.tabs) {
            if (tab.filePath === filePath) {
                window.rLog(`文件已打开，切换到tab: ${tabId}`);
                this.selectTab(tabId);
                return;
            }
        }

        // 创建新tab
        const tabId = `tab-${Date.now()}`;
        const name = filePath.split('/').pop();

        // 创建tab UI
        this.createTabUI(tabId, name, filePath);

        // 创建editor容器
        const container = document.createElement('div');
        container.id = `editor-${tabId}`;
        container.className = 'editor-wrapper';
        container.style.display = 'none';
        this.editorContainer.appendChild(container);

        // 创建editor实例
        const editor = new window.CodeJarAdapter(container, filePath);
        await editor.init();

        // 监听dirty状态
        editor.on('dirty-changed', (data) => {
            this.updateDirtyIndicator(tabId, data.isDirty);
        });

        // 保存到map
        this.tabs.set(tabId, { editor, filePath, name });

        // 选择新tab
        this.selectTab(tabId);

        window.rLog(`✅ 文件已打开: ${filePath}`);
    }

    /**
     * 创建tab UI
     */
    createTabUI(tabId, name, filePath) {
        const tab = document.createElement('div');
        tab.id = tabId;
        tab.className = 'tab';
        tab.title = filePath;
        tab.innerHTML = `
            <span class="tab-icon">📄</span>
            <span class="tab-name">${name}</span>
            <span class="tab-close">×</span>
        `;

        tab.querySelector('.tab-close').addEventListener('click', (e) => {
            e.stopPropagation();
            this.closeTab(tabId);
        });

        tab.addEventListener('click', () => {
            this.selectTab(tabId);
        });

        this.tabsContainer.appendChild(tab);
    }

    /**
     * 选择tab
     */
    selectTab(tabId) {
        window.rLog(`🔖 选择tab: ${tabId}`);

        // 取消所有active
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        document.querySelectorAll('.editor-wrapper').forEach(e => e.style.display = 'none');

        // 激活当前tab
        const tabEl = document.getElementById(tabId);
        const editorEl = document.getElementById(`editor-${tabId}`);

        if (tabEl) tabEl.classList.add('active');
        if (editorEl) editorEl.style.display = 'block';

        this.activeTabId = tabId;

        // 聚焦editor
        const tab = this.tabs.get(tabId);
        if (tab) {
            setTimeout(() => tab.editor.focus(), 50);
        }

        window.rLog(`✅ Tab已选择: ${tabId}`);
    }

    /**
     * 关闭tab
     */
    async closeTab(tabId) {
        window.rLog(`🗑️  关闭tab: ${tabId}`);

        const tab = this.tabs.get(tabId);
        if (!tab) return;

        // 检查是否有未保存的修改
        if (tab.editor.isDirtyState()) {
            const confirmed = confirm(`文件 ${tab.name} 有未保存的修改，确定要关闭吗？`);
            if (!confirmed) return;
        }

        // 销毁editor
        tab.editor.destroy();

        // 移除DOM
        document.getElementById(tabId)?.remove();
        document.getElementById(`editor-${tabId}`)?.remove();

        // 从map移除
        this.tabs.delete(tabId);

        // 如果关闭的是当前tab，切换到其他tab
        if (this.activeTabId === tabId) {
            const remaining = Array.from(this.tabs.keys());
            if (remaining.length > 0) {
                this.selectTab(remaining[0]);
            } else {
                this.activeTabId = null;
            }
        }

        window.rLog(`✅ Tab已关闭: ${tabId}`);
    }

    /**
     * 更新dirty指示器
     */
    updateDirtyIndicator(tabId, isDirty) {
        const tabEl = document.getElementById(tabId);
        if (!tabEl) return;

        let indicator = tabEl.querySelector('.tab-dirty');

        if (isDirty) {
            if (!indicator) {
                indicator = document.createElement('span');
                indicator.className = 'tab-dirty';
                indicator.textContent = '●';
                tabEl.insertBefore(indicator, tabEl.querySelector('.tab-name'));
            }
        } else {
            indicator?.remove();
        }
    }

    /**
     * 保存当前文件
     */
    async saveCurrentFile() {
        if (!this.activeTabId) {
            window.rLog('⚠️  没有活动的tab');
            return;
        }

        const tab = this.tabs.get(this.activeTabId);
        if (tab) {
            await tab.editor.save();
            window.rLog(`✅ 文件已保存: ${tab.filePath}`);
        }
    }

    /**
     * 获取当前editor
     */
    getCurrentEditor() {
        if (!this.activeTabId) return null;
        const tab = this.tabs.get(this.activeTabId);
        return tab?.editor;
    }

    /**
     * 获取当前内容
     */
    getCurrentContent() {
        const editor = this.getCurrentEditor();
        return editor?.getContent() || '';
    }

    /**
     * 保存当前文件（带通知）
     */
    async saveCurrentFileWithNotification() {
        if (!this.activeTabId) {
            window.AppNotifications?.warn('没有打开的文件');
            return;
        }
        try {
            await this.saveCurrentFile();
            window.AppNotifications?.success('文件保存成功');
        } catch (error) {
            window.rError('保存失败:', error);
            window.AppNotifications?.error(`保存失败: ${error.message}`);
        }
    }

    /**
     * 切换到下一个tab
     */
    switchToNextTab() {
        const ids = Array.from(this.tabs.keys());
        if (ids.length <= 1) return;
        const idx = ids.indexOf(this.activeTabId);
        this.selectTab(ids[(idx + 1) % ids.length]);
    }

    /**
     * 切换到上一个tab
     */
    switchToPreviousTab() {
        const ids = Array.from(this.tabs.keys());
        if (ids.length <= 1) return;
        const idx = ids.indexOf(this.activeTabId);
        this.selectTab(ids[(idx - 1 + ids.length) % ids.length]);
    }
}

// 导出全局实例
window.EditorManager = new CodeJarManager();
(window.rLog || console.log)('✅ CodeJarManager 模块已加载');
(window.rLog || console.log)('EditorManager:', window.EditorManager);
