// 自定义标题栏控制模块
// 使用全局的 ipcRenderer，避免重复声明

class CustomTitleBar {
    constructor() {
        this.isMaximized = false;
        this.init();
    }

    init() {
        // 只在Windows平台初始化
        if (process.platform !== 'win32') {
            return;
        }

        // 绑定按钮事件
        this.bindEvents();
        
        // 初始化窗口状态
        this.updateMaximizeButton();
    }

    bindEvents() {
        const minimizeBtn = document.getElementById('minimize-btn');
        const maximizeBtn = document.getElementById('maximize-btn');
        const closeBtn = document.getElementById('close-btn');

        if (minimizeBtn) {
            minimizeBtn.addEventListener('click', async () => {
                await window.AppGlobals.ipcRenderer.invoke('window-minimize');
            });
        }

        if (maximizeBtn) {
            maximizeBtn.addEventListener('click', async () => {
                const isMaximized = await window.AppGlobals.ipcRenderer.invoke('window-maximize');
                this.isMaximized = isMaximized;
                this.updateMaximizeButton();
            });
        }

        if (closeBtn) {
            closeBtn.addEventListener('click', async () => {
                await window.AppGlobals.ipcRenderer.invoke('window-close');
            });
        }

        // 监听窗口状态变化（双击标题栏等）
        window.addEventListener('resize', () => {
            this.updateMaximizeButton();
        });
    }

    async updateMaximizeButton() {
        const isMaximized = await window.AppGlobals.ipcRenderer.invoke('window-is-maximized');
        this.isMaximized = isMaximized;
        
        const maximizeBtn = document.getElementById('maximize-btn');
        if (maximizeBtn) {
            const svg = maximizeBtn.querySelector('svg');
            if (isMaximized) {
                document.body.classList.add('window-maximized');
                maximizeBtn.title = '还原';
                // 更新图标为还原图标
                svg.innerHTML = `
                    <path d="M2,2 L8,2 L8,8 L2,8 Z" stroke="currentColor" stroke-width="1" fill="none"/>
                    <path d="M0,0 L6,0 L6,6" stroke="currentColor" stroke-width="1" fill="none"/>
                `;
            } else {
                document.body.classList.remove('window-maximized');
                maximizeBtn.title = '最大化';
                // 更新图标为最大化图标
                svg.innerHTML = `
                    <path d="M0,0 L10,0 L10,10 L0,10 Z" stroke="currentColor" stroke-width="1" fill="none"/>
                `;
            }
        }
    }

    // 双击标题栏切换最大化状态
    setupDoubleClickMaximize() {
        const dragRegion = document.querySelector('.titlebar-drag-region');
        if (dragRegion) {
            dragRegion.addEventListener('dblclick', async () => {
                const isMaximized = await window.AppGlobals.ipcRenderer.invoke('window-maximize');
                this.isMaximized = isMaximized;
                this.updateMaximizeButton();
            });
        }
    }
}

// 导出模块
if (typeof module !== 'undefined' && module.exports) {
    module.exports = CustomTitleBar;
}

// 在DOM加载完成后初始化，但需要等待AppGlobals加载
if (typeof window !== 'undefined') {
    // 检查AppGlobals是否已加载
    function initCustomTitleBar() {
        if (window.AppGlobals && window.AppGlobals.ipcRenderer) {
            const customTitleBar = new CustomTitleBar();
            customTitleBar.setupDoubleClickMaximize();
            
            // 全局访问
            window.customTitleBar = customTitleBar;
        } else {
            // 如果AppGlobals还没加载，等待一下再尝试
            setTimeout(initCustomTitleBar, 100);
        }
    }
    
    document.addEventListener('DOMContentLoaded', initCustomTitleBar);
}