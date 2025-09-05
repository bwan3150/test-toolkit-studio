// TKS脚本集成模块 (TKE版本)
// 使用Rust TKE执行TKS脚本功能

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// TKS脚本运行器 (TKE版本)
class TKSScriptRunnerTKE {
    constructor() {
        this.isRunning = false;
        this.currentScript = null;
        this.initialized = false;
        this.tkeAdapter = null;
        this.scriptRunner = null;
    }

    /**
     * 初始化脚本运行器
     */
    async init() {
        if (this.initialized) {
            console.log('TKSScriptRunnerTKE已经初始化过，跳过重复初始化');
            return;
        }
        
        try {
            // 初始化TKE适配器
            this.tkeAdapter = await window.TKEAdapterModule.getTKEAdapter();
            
            // 设置日志回调
            this.tkeAdapter.setLogCallback((message, level) => {
                if (window.TestcaseController && window.TestcaseController.ConsoleManager) {
                    const logLevel = level === 'error' ? 'error' : 'info';
                    window.TestcaseController.ConsoleManager.addLog(message, logLevel);
                }
            });
            
            // 绑定Run Test按钮
            const runTestBtn = document.getElementById('runTestBtn');
            if (runTestBtn) {
                console.log('TKSScriptRunnerTKE: 绑定Run Test按钮事件');
                runTestBtn.addEventListener('click', () => {
                    console.log('Run Test按钮被点击');
                    this.handleRunTest();
                });
            } else {
                console.error('TKSScriptRunnerTKE: 找不到runTestBtn元素');
            }
            
            this.initialized = true;
            console.log('TKS脚本运行器(TKE版本)已初始化');
        } catch (error) {
            console.error('TKS脚本运行器(TKE版本)初始化失败:', error);
            throw error;
        }
    }

    /**
     * 处理运行测试
     */
    async handleRunTest() {
        console.log('handleRunTest: 开始处理测试执行 (TKE版本)');
        
        // 立即清除编辑器焦点和光标，防止干扰高亮显示
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.contentEl) {
            console.log('handleRunTest: 清除编辑器焦点');
            window.AppGlobals.codeEditor.contentEl.blur();
            // 清除选择区域
            if (window.getSelection) {
                window.getSelection().removeAllRanges();
            }
            // 移除焦点到一个不可见的元素
            if (document.body) {
                document.body.focus();
            }
        }
        
        // 获取当前活动的标签页
        const currentTab = window.AppGlobals.currentTab;
        console.log('handleRunTest: 详细调试信息', {
            'window.AppGlobals存在': !!window.AppGlobals,
            'openTabs数量': window.AppGlobals.openTabs ? window.AppGlobals.openTabs.length : '无openTabs',
            'currentTab': currentTab,
            'currentTab类型': typeof currentTab
        });
        
        if (!currentTab) {
            console.log('handleRunTest: 没有活动标签页');
            window.NotificationModule.showNotification('请先打开一个测试脚本', 'warning');
            return;
        }

        // 检查文件扩展名
        const fileName = currentTab.name;
        if (!fileName.endsWith('.tks') && !fileName.endsWith('.yaml')) {
            window.NotificationModule.showNotification('请打开一个 .tks 或 .yaml 脚本文件', 'warning');
            return;
        }

        // 获取脚本内容
        const scriptContent = currentTab.content;
        if (!scriptContent || scriptContent.trim().length === 0) {
            window.NotificationModule.showNotification('脚本内容为空', 'warning');
            return;
        }

        try {
            await this.executeScript(scriptContent, currentTab.path);
        } catch (error) {
            console.error('脚本执行失败:', error);
            window.NotificationModule.showNotification(`脚本执行失败: ${error.message}`, 'error');
        }
    }

    /**
     * 执行脚本
     */
    async executeScript(scriptContent, scriptPath) {
        if (this.isRunning) {
            window.NotificationModule.showNotification('脚本正在运行中...', 'warning');
            return;
        }

        // 检查是否有选中的设备
        const deviceSelect = document.getElementById('deviceSelect');
        if (!deviceSelect || !deviceSelect.value) {
            window.NotificationModule.showNotification('请先选择一个设备', 'warning');
            return;
        }

        const deviceId = deviceSelect.value;
        
        // 切换到控制台标签页
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }

        this.isRunning = true;
        this.updateRunButton(true);
        
        // 设置屏幕模式管理器为测试运行状态（自动切换到纯屏幕模式并禁用切换）
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(true);
        }
        
        try {
            // 获取项目路径
            let projectPath = window.AppGlobals.getCurrentProjectPath();
            
            console.log('项目路径调试信息:', {
                'window.AppGlobals.currentProject': window.AppGlobals.currentProject,
                'projectPath': projectPath,
                'scriptPath': scriptPath,
                'typeof projectPath': typeof projectPath
            });
            
            if (!projectPath) {
                // 详细调试信息，找出为什么获取不到项目路径
                console.error('❌ 无法获取项目路径 - 详细调试信息:');
                console.error('- window.AppGlobals:', !!window.AppGlobals);
                console.error('- window.AppGlobals.currentProject:', window.AppGlobals?.currentProject);
                console.error('- typeof window.AppGlobals.currentProject:', typeof window.AppGlobals?.currentProject);
                
                // 检查状态栏显示的项目路径作为参考
                const statusBarElement = document.getElementById('statusBarProjectPath');
                if (statusBarElement) {
                    console.error('- 状态栏显示内容:', statusBarElement.textContent);
                    console.error('- 状态栏存储的完整路径:', statusBarElement.parentElement.dataset.fullPath);
                } else {
                    console.error('- 状态栏元素不存在');
                }
                
                throw new Error('无法获取项目路径。请确保已打开一个项目（Project页面 -> Open Project或Create Project）');
            }

            window.TestcaseController.ConsoleManager.addLog('开始执行TKS脚本... (使用TKE引擎)', 'info');
            
            // 创建TKE脚本运行器
            this.scriptRunner = new window.TKEAdapterModule.TKEScriptRunnerAdapter(
                this.tkeAdapter, 
                projectPath, 
                deviceId
            );
            
            let result;
            
            // 如果有脚本文件路径，使用文件执行方式
            if (scriptPath) {
                console.log('使用文件执行方式:', scriptPath);
                result = await this.scriptRunner.runScriptFile(scriptPath);
            } else {
                console.log('使用内容执行方式');
                result = await this.scriptRunner.runScriptContent(scriptContent);
            }
            
            // 处理执行结果
            if (result.success) {
                const executedSteps = result.totalSteps || 0;
                const successfulSteps = result.successfulSteps || 0;
                window.TestcaseController.ConsoleManager.addLog(
                    `脚本执行完成，共执行 ${executedSteps} 步，成功 ${successfulSteps} 步`, 
                    'success'
                );
                window.NotificationModule.showNotification('脚本执行成功', 'success');
            } else {
                const errorMsg = result.error || '未知错误';
                window.TestcaseController.ConsoleManager.addLog(`脚本执行失败: ${errorMsg}`, 'error');
                window.NotificationModule.showNotification('脚本执行失败', 'error');
            }
            
        } catch (error) {
            console.error('执行脚本时出错:', error);
            window.TestcaseController.ConsoleManager.addLog(`执行出错: ${error.message}`, 'error');
            throw error;
        } finally {
            this.isRunning = false;
            this.scriptRunner = null;
            this.updateRunButton(false);
            
            // 恢复编辑器交互状态
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
                console.log('TKS集成: 恢复编辑器交互状态');
                window.AppGlobals.codeEditor.setTestRunning(false);
            }
            
            // 恢复屏幕模式切换功能
            if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
                window.TestcaseController.ScreenModeManager.setTestRunning(false);
            }
            
            // 更新状态栏显示
            if (window.StatusBarModule && window.StatusBarModule.updateRunStatus) {
                console.log('TKS集成: 更新状态栏为空闲状态');
                window.StatusBarModule.updateRunStatus(false);
            }
        }
    }

    /**
     * 停止执行
     */
    async stopExecution() {
        console.log('TKSScriptRunnerTKE: 用户请求停止脚本执行');
        
        if (this.isRunning && this.scriptRunner) {
            try {
                // 停止TKE执行器
                await this.scriptRunner.stopExecution();
            } catch (error) {
                console.error('停止执行时出错:', error);
            }
        }
        
        // 更新状态
        this.isRunning = false;
        this.scriptRunner = null;
        this.updateRunButton(false);
        
        // 恢复编辑器交互状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            console.log('TKSScriptRunnerTKE: 恢复编辑器交互状态');
            window.AppGlobals.codeEditor.setTestRunning(false);
        }
        
        // 恢复屏幕模式切换功能
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(false);
        }
        
        window.TestcaseController.ConsoleManager.addLog('用户停止了脚本执行', 'warning');
        window.NotificationModule.showNotification('已停止执行', 'info');
    }
    
    /**
     * 更新运行按钮状态
     */
    updateRunButton(isRunning) {
        const runTestBtn = document.getElementById('runTestBtn');
        if (!runTestBtn) return;
        
        // 移除所有旧的事件监听器
        const newBtn = runTestBtn.cloneNode(true);
        runTestBtn.parentNode.replaceChild(newBtn, runTestBtn);
        
        if (isRunning) {
            // 运行中状态 - 变成停止按钮
            newBtn.innerHTML = `
                <svg class="btn-icon" viewBox="0 0 24 24">
                    <path d="M6 6h12v12H6z"/>
                </svg>
                Stop Test (TKE)
            `;
            newBtn.className = 'btn btn-danger btn-block';
            newBtn.addEventListener('click', () => {
                console.log('Stop Test按钮被点击');
                this.stopExecution();
            });
        } else {
            // 空闲状态 - 变成运行按钮
            newBtn.innerHTML = `
                <svg class="btn-icon" viewBox="0 0 24 24">
                    <path d="M8 5v14l11-7z"/>
                </svg>
                Run Test (TKE)
            `;
            newBtn.className = 'btn btn-primary btn-block';
            newBtn.addEventListener('click', () => {
                console.log('Run Test按钮被点击');
                this.handleRunTest();
            });
        }
    }
}

// 模块变量
let scriptRunnerTKE = null;
let moduleInitialized = false;

// 初始化模块
async function initializeTKSIntegrationTKE() {
    if (moduleInitialized) {
        console.log('TKS集成模块(TKE版本)已经初始化过，跳过重复初始化');
        return;
    }
    
    try {
        // 创建脚本运行器实例
        scriptRunnerTKE = new TKSScriptRunnerTKE();
        
        // 初始化
        await scriptRunnerTKE.init();
        
        // 导出到全局
        window.TKSIntegrationTKE = {
            scriptRunner: scriptRunnerTKE,
            
            // 公开的API
            runCurrentScript: () => scriptRunnerTKE.handleRunTest(),
            stopExecution: () => scriptRunnerTKE.stopExecution()
        };
        
        moduleInitialized = true;
        console.log('TKS集成模块(TKE版本)已初始化');
    } catch (error) {
        console.error('TKS集成模块(TKE版本)初始化失败:', error);
        throw error;
    }
}

// 导出模块
window.TKSIntegrationTKEModule = {
    initializeTKSIntegrationTKE,
    TKSScriptRunnerTKE
};