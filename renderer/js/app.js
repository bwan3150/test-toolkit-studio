// 主应用入口文件
//
// 此文件负责加载所有模块并初始化应用程序

// 注意：AppGlobals（全局变量和依赖项）已在 index.html 中内联加载
//
// 模块加载顺序：
// 1. ui/notifications.js - 通知系统
// 2. ui/navigation.js - 导航管理
// 3. ui/editor-manager.js - 编辑器管理功能（在 index.html 中静态加载）
// 4. ui/settings.js - 设置页面
// 5. ui/resizable-panels.js - 可调整面板
// 6. modules/project-manager.js - 项目管理
// 7. testcase/testcase-controller.js - 测试用例控制器
// 8. modules/device-manager.js - 设备管理
// 9. utils/keyboard-shortcuts.js - 键盘快捷键
// 10. utils/ipc-handlers.js - IPC消息处理

// 在最开始就尝试加载 renderer-logger
(async function() {
    try {
        const { ipcRenderer } = require('electron');
        // 直接发送一条测试日志
        ipcRenderer.send('renderer-log', {
            level: 'info',
            message: 'app.js 文件开始执行',
            timestamp: new Date().toISOString()
        });
    } catch (error) {
        console.error('无法发送初始测试日志:', error);
    }
})();

// 异步加载模块的辅助函数
function loadScript(src) {
    return new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = src;
        script.onload = () => {
            if (window.rLog) window.rLog(`✓ 已加载脚本: ${src}`);
            resolve();
        };
        script.onerror = (error) => {
            // 使用原生方式发送错误日志，因为 renderer-logger 可能还没加载
            try {
                const { ipcRenderer } = require('electron');
                ipcRenderer.send('renderer-log', {
                    level: 'error',
                    message: `✗ 脚本加载失败: ${src}`,
                    timestamp: new Date().toISOString()
                });
            } catch (e) {
                console.error(`✗ 脚本加载失败: ${src}`, error);
            }
            reject(new Error(`Failed to load script: ${src}`));
        };
        document.head.appendChild(script);
    });
}

// 应用初始化
document.addEventListener('DOMContentLoaded', async () => {
    // 直接发送DOMContentLoaded事件日志
    const { ipcRenderer } = require('electron');
    ipcRenderer.send('renderer-log', {
        level: 'info',
        message: 'DOMContentLoaded 事件触发',
        timestamp: new Date().toISOString()
    });
    
    try {
        // 测试日志：开始加载脚本
        ipcRenderer.send('renderer-log', {
            level: 'info',
            message: '开始加载 log-control.js',
            timestamp: new Date().toISOString()
        });
        
        // 0. 首先加载日志控制系统（完全禁用用户控制台日志）
        await loadScript('../js/utils/log-control.js');
        
        ipcRenderer.send('renderer-log', {
            level: 'info',
            message: 'log-control.js 加载完成',
            timestamp: new Date().toISOString()
        });
        
        // 0.5 加载渲染进程日志模块（用于发送日志到CLI）
        await loadScript('../js/utils/renderer-logger.js');
        
        // 现在可以使用 renderer logger
        window.rLog('开始应用初始化...');
        window.rLog('正在加载模块...');

        // 1. 验证核心模块（全局变量）已在 index.html 中内联加载
        if (!window.AppGlobals) {
            window.rError('❌ AppGlobals 未定义!');
            throw new Error('AppGlobals 初始化失败 - AppGlobals 未定义');
        }
        window.rLog('✓ AppGlobals 已加载', {
            hasSetCodeEditor: typeof window.AppGlobals.setCodeEditor,
            properties: Object.keys(window.AppGlobals)
        });

        // 2. 加载UI模块
        // notifications 已迁移到 utils/app-notifications.js,在 index.html 中静态加载
        await loadScript('../js/components/navigation.js');
        // editor-tab.js 和 editor-manager.js 已经在 index.html 中加载
        // 初始化编辑器管理器
        window.initializeEditorManager();
        
        // 2.5 加载组件
        if (window.ComponentLoader) {
            const components = [
                { name: 'connection-guide', container: 'connectionGuideModalContainer' },
                { name: 'apk-modals', container: 'apkModalsContainer' },
                { name: 'common-modals', container: 'commonModalsContainer' }
            ];
            await window.ComponentLoader.loadComponents(components);
        }
        await loadScript('../js/settings/settings.js');
        await loadScript('../js/components/resizable-panels.js');
        await loadScript('../js/components/status-bar.js');
        // console.log('✓ UI模块已加载'); // 已禁用以减少日志
        
        // 3. 加载业务功能模块
        await loadScript('../js/project/project-manager.js');

        // 加载拆分的 testcase 子模块
        await loadScript('../js/testcase/explorer/testcase-explorer.js');
        await loadScript('../js/testcase/screen/device-screen-manager.js');
        await loadScript('../js/testcase/screen/screen-mode-manager.js');

        // 加载主控制器（依赖上面的子模块）
        await loadScript('../js/testcase/testcase-controller.js');
        await loadScript('../js/device/device-manager.js');
        await loadScript('../js/testcase/controller/locator-manager-tke.js');
        await loadScript('../js/logviewer/log-manager.js');
        await loadScript('../js/services/bug-analyzer-client.js'); // Bug分析API客户端
        await loadScript('../js/insights/test-report-manager.js');
        // console.log('✓ 业务模块已加载'); // 已禁用以减少日志
        
        // 4. 加载工具模块
        await loadScript('../js/services/api-client.js');
        await loadScript('../js/utils/keyboard-shortcuts.js');
        await loadScript('../js/utils/ipc-handlers.js');
        // console.log('✓ 工具模块已加载'); // 已禁用以减少日志
        
        // 等待短暂时间确保所有模块都已完全初始化
        await new Promise(resolve => setTimeout(resolve, 200));
        
        // 检查模块是否正确加载
        // console.log('检查模块加载状态...'); // 已禁用以减少日志
        const requiredModules = [
            'NavigationModule', 'EditorManager', 'ResizablePanelsModule', 'StatusBarModule',
            'ProjectManagerModule', 'TestcaseController', 'DeviceManagerModule',
            'LogManagerModule', 'TestReportModule', 'SettingsModule', 'KeyboardShortcutsModule', 'IpcHandlersModule',
            'NotificationModule', 'AppGlobals'
            // ApiClient 是可选的，稍后单独检查
        ];
        
        for (const moduleName of requiredModules) {
            if (!window[moduleName]) {
                window.rError(`模块 ${moduleName} 未正确加载`);
                window.rError('当前可用的window属性:', Object.keys(window).filter(key => key.includes('Module') || key.includes('Client')));
                throw new Error(`模块 ${moduleName} 未正确加载`);
            }
            window.rLog(`✓ ${moduleName} 已加载`);
        }
        
        // 初始化所有功能
        // console.log('正在初始化各功能模块...'); // 已禁用以减少日志
        
        // 初始化UI组件
        try {
            window.NavigationModule.initializeNavigation();
            // console.log('✓ 导航模块已初始化'); // 已禁用以减少日志
            
            // EditorManager已在HTML加载时初始化
            // console.log('✓ 编辑器模块已初始化'); // 已禁用以减少日志
            
            window.ResizablePanelsModule.initializeResizablePanels();
            // console.log('✓ 面板模块已初始化'); // 已禁用以减少日志
        } catch (error) {
            window.rError('UI组件初始化失败:', error);
            throw error;
        }
        
        // 初始化业务功能
        try {
            window.ProjectManagerModule.initializeProjectPage();
            // console.log('✓ 项目管理模块已初始化'); // 已禁用以减少日志
            
            window.TestcaseController.initializeTestcasePage();
            // console.log('✓ 测试用例模块已初始化'); // 已禁用以减少日志
            
            window.DeviceManagerModule.initializeDevicePage();
            // console.log('✓ 设备管理模块已初始化'); // 已禁用以减少日志
            
            try {
                await window.LogManagerModule.initializeLogPage();
                window.rLog('✓ 日志管理模块已初始化');
            } catch (error) {
                window.rError('日志管理模块初始化失败:', error);
            }
            
            try {
                window.SettingsModule.initializeSettingsPage();
                window.rLog('✓ 设置模块已初始化');
            } catch (error) {
                window.rError('设置模块初始化失败:', error);
            }

            // 初始化Locator管理器
            if (window.LocatorManager) {
                window.LocatorManager.initialize();
                // console.log('✓ Locator管理器已初始化'); // 已禁用以减少日志
            }
        } catch (error) {
            window.rError('业务功能初始化失败:', error);
            throw error;
        }
        
        // 初始化工具功能
        try {
            // 初始化API客户端（如果已加载）
            if (window.ApiClient) {
                await window.ApiClient.initialize();
                // console.log('✓ API客户端已初始化'); // 已禁用以减少日志
            } else {
                window.rWarn('ApiClient模块未加载，跳过初始化');
            }
            
            window.KeyboardShortcutsModule.initializeKeyboardShortcuts();
            // console.log('✓ 快捷键模块已初始化'); // 已禁用以减少日志
            
            window.IpcHandlersModule.initializeIpcHandlers();
            // console.log('✓ IPC处理模块已初始化'); // 已禁用以减少日志
        } catch (error) {
            window.rError('工具功能初始化失败:', error);
            throw error;
        }
        
        // 检查认证状态
        window.rLog('检查用户认证状态...');
        const isAuthenticated = await window.ApiClient.checkAuthStatus();
        
        if (!isAuthenticated) {
            window.rWarn('用户未认证，跳转到登录页面');
            // 跳转到登录页面
            const { ipcRenderer } = require('electron');
            await ipcRenderer.invoke('navigate-to-login');
            return; // 停止继续初始化
        }
        
        window.rLog('用户认证通过，加载用户信息...');
        // 加载用户信息
        await window.SettingsModule.loadUserInfo();
        window.rLog('✓ 用户信息已加载');
        
        // 加载项目历史而不是自动加载最后一个项目
        await window.ProjectManagerModule.loadProjectHistory();
        // console.log('✓ 项目历史已加载'); // 已禁用以减少日志
        
        // 初始化测试报告模块
        if (window.TestReportModule) {
            try {
                await window.TestReportModule.initializeReportPage();
                window.rLog('✓ 测试报告模块已初始化');
            } catch (error) {
                window.rError('测试报告模块初始化失败:', error);
            }
        } else {
            window.rError('TestReportModule未加载');
        }
        
        // 最后初始化状态栏，确保能获取到正确的项目信息
        if (window.StatusBarModule) {
            window.StatusBarModule.init();
            // console.log('✓ 状态栏已初始化'); // 已禁用以减少日志
        }
        
        // console.log('✅ Test Toolkit Studio 已就绪'); // 完全禁用所有日志
        
        // 显示应用加载成功的通知
        window.NotificationModule.showNotification('应用已成功加载', 'success');
        
    } catch (error) {
        window.rError('❌ 应用初始化失败:', error);
        window.rError('错误堆栈:', error.stack);
        // 显示错误通知
        const toast = document.createElement('div');
        toast.className = 'notification notification-error';
        toast.textContent = `应用初始化失败: ${error.message}`;
        toast.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            padding: 12px 20px;
            background: #f48771;
            color: white;
            border-radius: 6px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            z-index: 10000;
            font-size: 13px;
        `;
        document.body.appendChild(toast);
    }
});

// 导出一些全局状态访问器（用于调试和兼容性）
window.AppDebug = {
    getCurrentProject: () => window.AppGlobals.currentProject,
    getOpenTabs: () => window.AppGlobals.openTabs,
    getCodeEditor: () => window.AppGlobals.codeEditor,
    // 添加更多调试辅助函数...
};

window.rLog('App.js 模块化版本加载完成');
