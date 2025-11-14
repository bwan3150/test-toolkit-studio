// Sidebar展开/收起控制
// 实现智能hover逻辑：
// - hover到sidebar时展开
// - 鼠标离开App时保持展开状态
// - 只有移动到app内部的非sidebar区域时才收起

(function() {
    // 等待sidebar加载完成
    const initSidebarController = () => {
        const sidebar = document.querySelector('.sidebar');
        const appMain = document.querySelector('.app-main');

        if (!sidebar || !appMain) {
            window.rLog('Sidebar或app-main元素未找到，延迟初始化');
            setTimeout(initSidebarController, 100);
            return;
        }

        window.rLog('初始化Sidebar控制器');

        let isMouseInApp = true; // 鼠标是否在App内部

        // 鼠标进入sidebar时展开
        sidebar.addEventListener('mouseenter', () => {
            window.rLog('鼠标进入sidebar，展开');
            sidebar.classList.add('expanded');
        });

        // 鼠标离开sidebar时的处理
        sidebar.addEventListener('mouseleave', (e) => {
            window.rLog('鼠标离开sidebar');

            // 检查鼠标是否移动到app内部的其他区域
            const relatedTarget = e.relatedTarget;

            // 如果鼠标移动到app-main内部的非sidebar区域，则收起
            if (relatedTarget && appMain.contains(relatedTarget) && !sidebar.contains(relatedTarget)) {
                window.rLog('鼠标移动到app内部非sidebar区域，收起');
                sidebar.classList.remove('expanded');
            } else {
                window.rLog('鼠标离开app或移动到sidebar子元素，保持展开');
            }
        });

        // 监听整个app区域的鼠标进入/离开
        appMain.addEventListener('mouseenter', () => {
            isMouseInApp = true;
            window.rLog('鼠标进入app区域');
        });

        appMain.addEventListener('mouseleave', () => {
            isMouseInApp = false;
            window.rLog('鼠标离开app区域');
        });

        // 监听app内部的鼠标移动
        appMain.addEventListener('mousemove', (e) => {
            // 如果sidebar是展开的，并且鼠标在app内部但不在sidebar内
            if (sidebar.classList.contains('expanded') &&
                !sidebar.contains(e.target) &&
                isMouseInApp) {
                // 检查鼠标位置是否真的离开了sidebar区域
                const sidebarRect = sidebar.getBoundingClientRect();
                if (e.clientX > sidebarRect.right) {
                    window.rLog('鼠标移动到sidebar右侧，收起');
                    sidebar.classList.remove('expanded');
                }
            }
        });

        window.rLog('Sidebar控制器初始化完成');
    };

    // 页面加载完成后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initSidebarController);
    } else {
        initSidebarController();
    }
})();
