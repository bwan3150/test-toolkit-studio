// 统一的Bottom Panel管理器
// 负责管理下方面板的Tab切换、收起展开、高度调整等功能
// JetBrains IDE风格的可扩展tab系统

// 安全的日志函数
function safeLog(message) {
    if (window.rLog) {
        window.rLog(message);
    } else {
        console.log(message);
    }
}

function safeError(message) {
    if (window.rError) {
        window.rError(message);
    } else {
        console.error(message);
    }
}

const BottomPanelManager = {
    // 状态
    state: {
        isCollapsed: false,
        currentTab: 'elements-list',
        height: 300,
        minHeight: 32, // 收起时的高度
        defaultHeight: 300,
        maxHeight: 600,
        isDragging: false,
        startY: 0,
        startHeight: 0,
        rafId: null
    },

    // 已注册的Tab配置
    tabs: new Map(),

    // 初始化
    init() {
        safeLog('🎯 初始化 BottomPanelManager');

        // 注册内置的4个tab
        this.registerTab({
            id: 'elements-list',
            title: '当前元素',
            icon: this.getIconSVG('current-element'),
            order: 1,
            onActivate: () => {
                // 激活时隐藏清空控制台按钮
                this.updateToolbarButtons('elements-list');
            }
        });

        this.registerTab({
            id: 'element-props',
            title: '元素属性',
            icon: this.getIconSVG('properties'),
            order: 2,
            onActivate: () => {
                this.updateToolbarButtons('element-props');
            }
        });

        this.registerTab({
            id: 'locator-lib',
            title: '元素库',
            icon: this.getIconSVG('library'),
            order: 3,
            onActivate: () => {
                this.updateToolbarButtons('locator-lib');
            }
        });

        this.registerTab({
            id: 'console',
            title: '控制台',
            icon: this.getIconSVG('console'),
            order: 4,
            onActivate: () => {
                // 激活时显示清空控制台按钮
                this.updateToolbarButtons('console');
            }
        });

        // 绑定事件
        this.bindEvents();

        // 渲染Tab栏
        this.renderTabs();

        // 恢复初始状态
        this.switchTab(this.state.currentTab);

        safeLog('✅ BottomPanelManager 初始化完成');
    },

    // 注册Tab
    registerTab(config) {
        const { id, title, icon, order = 999, onActivate, onDeactivate } = config;

        if (!id || !title) {
            safeError('注册Tab失败: 缺少id或title', config);
            return false;
        }

        this.tabs.set(id, {
            id,
            title,
            icon,
            order,
            onActivate,
            onDeactivate
        });

        safeLog(`✅ 注册Tab: ${id} - ${title}`);
        return true;
    },

    // 渲染Tab栏
    renderTabs() {
        const tabHeader = document.querySelector('.bottom-panel-tabs');
        if (!tabHeader) {
            safeError('❌ Tab header容器未找到: .bottom-panel-tabs');
            return;
        }

        // 按order排序
        const sortedTabs = Array.from(this.tabs.values()).sort((a, b) => a.order - b.order);

        safeLog(`📊 准备渲染 ${sortedTabs.length} 个Tabs`);

        // 生成Tab按钮HTML
        const tabsHTML = sortedTabs.map(tab => `
            <button class="bottom-tab-btn ${tab.id === this.state.currentTab ? 'active' : ''}"
                    data-tab-id="${tab.id}"
                    title="${tab.title}">
                ${tab.icon}
                <span class="tab-title">${tab.title}</span>
            </button>
        `).join('');

        tabHeader.innerHTML = tabsHTML;

        // 绑定Tab点击事件
        tabHeader.querySelectorAll('.bottom-tab-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const tabId = btn.dataset.tabId;
                this.switchTab(tabId);
            });
        });

        safeLog(`✅ Tabs已渲染到DOM`);
    },

    // 切换Tab
    switchTab(tabId) {
        const tab = this.tabs.get(tabId);
        if (!tab) {
            window.rWarn(`Tab ${tabId} 不存在`);
            return;
        }

        // 调用旧Tab的onDeactivate
        const oldTab = this.tabs.get(this.state.currentTab);
        if (oldTab && oldTab.onDeactivate) {
            oldTab.onDeactivate();
        }

        // 更新状态
        this.state.currentTab = tabId;

        // 更新Tab按钮样式
        document.querySelectorAll('.bottom-tab-btn').forEach(btn => {
            if (btn.dataset.tabId === tabId) {
                btn.classList.add('active');
            } else {
                btn.classList.remove('active');
            }
        });

        // 显示/隐藏对应的内容面板
        document.querySelectorAll('.bottom-panel-content-pane').forEach(pane => {
            if (pane.dataset.paneId === tabId) {
                pane.classList.add('active');
                pane.style.display = 'flex';
            } else {
                pane.classList.remove('active');
                pane.style.display = 'none';
            }
        });

        // 调用新Tab的onActivate
        if (tab.onActivate) {
            tab.onActivate();
        }

        safeLog(`📑 切换到Tab: ${tabId}`);
    },

    // 更新工具栏按钮
    updateToolbarButtons(tabId) {
        const clearConsoleBtn = document.getElementById('clearConsoleBtn');

        if (tabId === 'console') {
            // 控制台Tab: 显示清空按钮
            if (clearConsoleBtn) clearConsoleBtn.style.display = 'flex';
        } else {
            // 其他Tab: 隐藏清空按钮
            if (clearConsoleBtn) clearConsoleBtn.style.display = 'none';
        }
    },

    // 切换收起/展开
    toggle() {
        if (this.state.isCollapsed) {
            this.expand();
        } else {
            this.collapse();
        }
    },

    // 收起
    collapse() {
        const panel = document.getElementById('bottomPanel');
        const toggleIcon = document.querySelector('#togglePanelBtn svg');

        if (!panel) return;

        panel.classList.add('collapsed');
        this.state.isCollapsed = true;

        // 旋转图标
        if (toggleIcon) {
            toggleIcon.style.transform = 'rotate(180deg)';
        }

        safeLog('📦 Bottom Panel 已收起');
    },

    // 展开
    expand() {
        const panel = document.getElementById('bottomPanel');
        const toggleIcon = document.querySelector('#togglePanelBtn svg');

        if (!panel) return;

        panel.classList.remove('collapsed');
        this.state.isCollapsed = false;

        // 恢复图标
        if (toggleIcon) {
            toggleIcon.style.transform = 'rotate(0deg)';
        }

        safeLog('📂 Bottom Panel 已展开');
    },

    // 绑定事件
    bindEvents() {
        // 收起/展开按钮
        const toggleBtn = document.getElementById('togglePanelBtn');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', () => this.toggle());
        }

        // 清空控制台按钮
        const clearConsoleBtn = document.getElementById('clearConsoleBtn');
        if (clearConsoleBtn) {
            clearConsoleBtn.addEventListener('click', () => {
                if (window.ConsolePanelModule && window.ConsolePanelModule.clear) {
                    window.ConsolePanelModule.clear();
                }
            });
        }

        // 拖拽调整高度 - mousedown在resizer上
        const resizer = document.getElementById('bottomPanelResizer');
        if (resizer) {
            resizer.addEventListener('mousedown', (e) => this.startDragging(e));
        }

        // 全局mousemove和mouseup - 用于拖拽
        document.addEventListener('mousemove', (e) => this.handleDragMove(e));
        document.addEventListener('mouseup', () => this.handleDragEnd());
    },

    // 开始拖拽
    startDragging(e) {
        e.preventDefault();
        this.state.isDragging = true;
        this.state.startY = e.clientY;
        this.state.startHeight = document.getElementById('bottomPanel').offsetHeight;

        const resizer = document.getElementById('bottomPanelResizer');
        if (resizer) {
            resizer.classList.add('dragging');
        }

        // 设置全局样式
        document.body.classList.add('bottom-panel-dragging');
        document.body.style.cursor = 'row-resize';
        document.body.style.userSelect = 'none';
    },

    // 处理拖拽移动(全局监听)
    handleDragMove(e) {
        if (!this.state.isDragging) return;

        // 使用requestAnimationFrame优化性能
        if (this.state.rafId) {
            cancelAnimationFrame(this.state.rafId);
        }

        this.state.rafId = requestAnimationFrame(() => {
            const panel = document.getElementById('bottomPanel');
            if (!panel) return;

            // 计算新高度 (向上拖动增加高度)
            const deltaY = this.state.startY - e.clientY;
            let newHeight = this.state.startHeight + deltaY;

            // 限制高度范围
            const minHeight = 150;
            const maxHeight = window.innerHeight * 0.85;
            newHeight = Math.max(minHeight, Math.min(newHeight, maxHeight));

            // 应用新高度
            panel.style.height = `${newHeight}px`;
            this.state.height = newHeight;
        });
    },

    // 停止拖拽(全局监听)
    handleDragEnd() {
        if (!this.state.isDragging) return;

        this.state.isDragging = false;

        // 清理requestAnimationFrame
        if (this.state.rafId) {
            cancelAnimationFrame(this.state.rafId);
            this.state.rafId = null;
        }

        // 清理拖拽样式
        const resizer = document.getElementById('bottomPanelResizer');
        if (resizer) {
            resizer.classList.remove('dragging');
        }

        document.body.classList.remove('bottom-panel-dragging');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';

        safeLog(`✅ Bottom Panel 高度调整为: ${this.state.height}px`);
    },

    // 获取图标SVG
    getIconSVG(type) {
        const icons = {
            'current-element': '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z"/></svg>',
            properties: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94L14.4 2.81c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/></svg>',
            library: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M4 6H2v14c0 1.1.9 2 2 2h14v-2H4V6zm16-4H8c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-2 9H10V9h8v2zm-3 3H10v-2h5v2zm3-6H10V6h8v2z"/></svg>',
            console: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M20 3H4c-1.1 0-2 .9-2 2v11c0 1.1.9 2 2 2h3l-1 1v2h12v-2l-1-1h3c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 13H4V5h16v11zM6 8.5l2.5 2.5L6 13.5 7 15l4-4-4-4zm5 5h5v1h-5z"/></svg>'
        };
        return icons[type] || icons['current-element'];
    }
};

// 导出到全局
window.BottomPanelManager = BottomPanelManager;

// 立即初始化或等待DOM加载
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        setTimeout(() => BottomPanelManager.init(), 100);
    });
} else {
    // DOM已经加载完成
    setTimeout(() => BottomPanelManager.init(), 100);
}
