// IPC消息处理模块

// 处理来自主进程的IPC消息
function initializeIpcHandlers() {
    const { ipcRenderer } = window.AppGlobals;
    
    ipcRenderer.on('menu-new-project', () => {
        const btn = document.getElementById('createProjectBtn');
        if (btn) btn.click();
    });

    ipcRenderer.on('menu-open-project', () => {
        const btn = document.getElementById('openProjectBtn');
        if (btn) btn.click();
    });

    ipcRenderer.on('menu-run-test', () => {
        if (document.getElementById('testcasePage').classList.contains('active')) {
            window.TestcaseController.runCurrentTest();
        }
    });

    ipcRenderer.on('menu-stop-test', () => {
        window.AppNotifications?.info('Stop test not yet implemented');
    });

    ipcRenderer.on('menu-refresh-device', () => {
        if (document.getElementById('testcasePage').classList.contains('active')) {
            window.TestcaseController.refreshDeviceScreen();
        }
    });
}

// 导出函数
window.IpcHandlersModule = {
    initializeIpcHandlers
};