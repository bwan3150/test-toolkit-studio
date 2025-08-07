// 主应用入口文件
// 
// 此文件负责加载所有模块并初始化应用程序

// 1. core/globals.js - 全局变量和依赖项
// 2. ui/notifications.js - 通知系统
// 3. ui/navigation.js - 导航管理
// 4. ui/editor.js - 编辑器功能
// 5. ui/settings.js - 设置页面
// 6. ui/resizable-panels.js - 可调整面板
// 7. modules/project-manager.js - 项目管理
// 8. modules/testcase-manager.js - 测试用例管理
// 9. modules/device-manager.js - 设备管理
// 10. utils/keyboard-shortcuts.js - 键盘快捷键
// 11. utils/ipc-handlers.js - IPC消息处理

// 异步加载模块的辅助函数
function loadScript(src) {
    return new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = src;
        script.onload = resolve;
        script.onerror = reject;
        document.head.appendChild(script);
    });
}

// 应用初始化
document.addEventListener('DOMContentLoaded', async () => {
    console.log('开始应用初始化...');
    
    try {
        // 加载所有模块
        console.log('正在加载模块...');
        
        // 1. 首先加载核心模块（全局变量）
        await loadScript('./js/core/globals.js');
        console.log('✓ 核心模块已加载');
        
        // 2. 加载UI模块
        await loadScript('./js/ui/notifications.js');
        await loadScript('./js/ui/navigation.js');
        await loadScript('./js/ui/editor.js');
        await loadScript('./js/ui/settings.js');
        await loadScript('./js/ui/resizable-panels.js');
        console.log('✓ UI模块已加载');
        
        // 3. 加载业务功能模块
        await loadScript('./js/modules/project-manager.js');
        await loadScript('./js/modules/testcase-manager.js');
        await loadScript('./js/modules/device-manager.js');
        console.log('✓ 业务模块已加载');
        
        // 4. 加载工具模块
        await loadScript('./js/utils/keyboard-shortcuts.js');
        await loadScript('./js/utils/ipc-handlers.js');
        console.log('✓ 工具模块已加载');
        
        // 等待短暂时间确保所有模块都已完全初始化
        await new Promise(resolve => setTimeout(resolve, 200));
        
        // 检查模块是否正确加载
        console.log('检查模块加载状态...');
        const requiredModules = [
            'NavigationModule', 'EditorModule', 'ResizablePanelsModule',
            'ProjectManagerModule', 'TestcaseManagerModule', 'DeviceManagerModule',
            'SettingsModule', 'KeyboardShortcutsModule', 'IpcHandlersModule',
            'NotificationModule', 'AppGlobals'
        ];
        
        for (const moduleName of requiredModules) {
            if (!window[moduleName]) {
                throw new Error(`模块 ${moduleName} 未正确加载`);
            }
            console.log(`✓ ${moduleName} 已加载`);
        }
        
        // 初始化所有功能
        console.log('正在初始化各功能模块...');
        
        // 初始化UI组件
        try {
            window.NavigationModule.initializeNavigation();
            console.log('✓ 导航模块已初始化');
            
            window.EditorModule.initializeSimpleEditor();
            console.log('✓ 编辑器模块已初始化');
            
            window.ResizablePanelsModule.initializeResizablePanels();
            console.log('✓ 面板模块已初始化');
        } catch (error) {
            console.error('UI组件初始化失败:', error);
            throw error;
        }
        
        // 初始化业务功能
        try {
            window.ProjectManagerModule.initializeProjectPage();
            console.log('✓ 项目管理模块已初始化');
            
            window.TestcaseManagerModule.initializeTestcasePage();
            console.log('✓ 测试用例模块已初始化');
            
            window.DeviceManagerModule.initializeDevicePage();
            console.log('✓ 设备管理模块已初始化');
            
            window.SettingsModule.initializeSettingsPage();
            console.log('✓ 设置模块已初始化');
        } catch (error) {
            console.error('业务功能初始化失败:', error);
            throw error;
        }
        
        // 初始化工具功能
        try {
            window.KeyboardShortcutsModule.initializeKeyboardShortcuts();
            console.log('✓ 快捷键模块已初始化');
            
            window.IpcHandlersModule.initializeIpcHandlers();
            console.log('✓ IPC处理模块已初始化');
        } catch (error) {
            console.error('工具功能初始化失败:', error);
            throw error;
        }
        
        // 加载用户信息
        await window.SettingsModule.loadUserInfo();
        console.log('✓ 用户信息已加载');
        
        // 加载项目历史而不是自动加载最后一个项目
        await window.ProjectManagerModule.loadProjectHistory();
        console.log('✓ 项目历史已加载');
        
        console.log('应用初始化完成！');
        
        // 显示应用加载成功的通知
        window.NotificationModule.showNotification('应用已成功加载', 'success');
        
    } catch (error) {
        console.error('❌ 应用初始化失败:', error);
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

console.log('App.js 模块化版本加载完成');
