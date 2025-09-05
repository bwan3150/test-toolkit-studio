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
            window.rLog('TKSScriptRunnerTKE已经初始化过，跳过重复初始化');
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
                window.rLog('TKSScriptRunnerTKE: 绑定Run Test按钮事件');
                runTestBtn.addEventListener('click', () => {
                    window.rLog('Run Test按钮被点击');
                    this.handleRunTest();
                });
            } else {
                window.rError('TKSScriptRunnerTKE: 找不到runTestBtn元素');
            }
            
            this.initialized = true;
            window.rLog('TKS脚本运行器(TKE版本)已初始化');
        } catch (error) {
            window.rError('TKS脚本运行器(TKE版本)初始化失败:', error);
            throw error;
        }
    }

    /**
     * 处理运行测试
     */
    async handleRunTest() {
        window.rLog('handleRunTest: 开始处理测试执行 (TKE版本)');
        
        // 立即清除编辑器焦点和光标，防止干扰高亮显示
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.contentEl) {
            window.rLog('handleRunTest: 清除编辑器焦点');
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
        window.rLog('handleRunTest: 详细调试信息', {
            'window.AppGlobals存在': !!window.AppGlobals,
            'openTabs数量': window.AppGlobals.openTabs ? window.AppGlobals.openTabs.length : '无openTabs',
            'currentTab': currentTab,
            'currentTab类型': typeof currentTab
        });
        
        if (!currentTab) {
            window.rLog('handleRunTest: 没有活动标签页');
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
            window.rError('脚本执行失败:', error);
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
            
            window.rLog('项目路径调试信息:', {
                'window.AppGlobals.currentProject': window.AppGlobals.currentProject,
                'projectPath': projectPath,
                'scriptPath': scriptPath,
                'typeof projectPath': typeof projectPath
            });
            
            if (!projectPath) {
                // 详细调试信息，找出为什么获取不到项目路径
                window.rError('❌ 无法获取项目路径 - 详细调试信息:');
                window.rError('- window.AppGlobals:', !!window.AppGlobals);
                window.rError('- window.AppGlobals.currentProject:', window.AppGlobals?.currentProject);
                window.rError('- typeof window.AppGlobals.currentProject:', typeof window.AppGlobals?.currentProject);
                
                // 检查状态栏显示的项目路径作为参考
                const statusBarElement = document.getElementById('statusBarProjectPath');
                if (statusBarElement) {
                    window.rError('- 状态栏显示内容:', statusBarElement.textContent);
                    window.rError('- 状态栏存储的完整路径:', statusBarElement.parentElement.dataset.fullPath);
                } else {
                    window.rError('- 状态栏元素不存在');
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
            
            // 设置执行回调
            const callbacks = {
                onLog: (message, level = 'info') => {
                    window.TestcaseController.ConsoleManager.addLog(message, level);
                },
                onStepStart: (stepIndex, stepDesc, totalSteps) => {
                    // 高亮当前执行步骤
                    if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.highlightExecutingLine) {
                        // 解析步骤描述获取行号，或根据步骤索引估算
                        const lineNumber = this.getLineNumberFromStepIndex(stepIndex);
                        if (lineNumber > 0) {
                            window.rLog('高亮执行步骤:', stepIndex, '行号:', lineNumber);
                            window.AppGlobals.codeEditor.highlightExecutingLine(lineNumber);
                        }
                    }
                    window.TestcaseController.ConsoleManager.addLog(
                        `[步骤 ${stepIndex + 1}/${totalSteps}] ${stepDesc}`, 
                        'info'
                    );
                },
                onStepComplete: (stepIndex, success, error = null) => {
                    const status = success ? '✓' : '✗';
                    const level = success ? 'success' : 'error';
                    let message = `步骤 ${stepIndex + 1} ${status}`;
                    if (error) {
                        message += ` - ${error}`;
                    }
                    window.TestcaseController.ConsoleManager.addLog(message, level);
                },
                onScreenshotUpdated: () => {
                    // 刷新设备截图显示
                    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                        window.DeviceScreenManagerModule.refreshDeviceScreen();
                    }
                },
                onComplete: (result, error) => {
                    // 清除代码高亮
                    if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.clearExecutionHighlight) {
                        window.rLog('清除执行高亮');
                        window.AppGlobals.codeEditor.clearExecutionHighlight();
                    }
                }
            };

            // 如果有脚本文件路径，使用文件执行方式
            if (scriptPath) {
                window.rLog('使用实时文件执行方式:', scriptPath);
                result = await this.scriptRunner.runScriptFile(scriptPath, callbacks);
            } else {
                window.rLog('使用内容执行方式');
                result = await this.scriptRunner.runScriptContent(scriptContent, callbacks);
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
            window.rError('执行脚本时出错:', error);
            window.TestcaseController.ConsoleManager.addLog(`执行出错: ${error.message}`, 'error');
            throw error;
        } finally {
            this.isRunning = false;
            this.scriptRunner = null;
            this.updateRunButton(false);
            
            // 恢复编辑器交互状态
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
                window.rLog('TKS集成: 恢复编辑器交互状态');
                window.AppGlobals.codeEditor.setTestRunning(false);
            }
            
            // 恢复屏幕模式切换功能
            if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
                window.TestcaseController.ScreenModeManager.setTestRunning(false);
            }
            
            // 更新状态栏显示
            if (window.StatusBarModule && window.StatusBarModule.updateRunStatus) {
                window.rLog('TKS集成: 更新状态栏为空闲状态');
                window.StatusBarModule.updateRunStatus(false);
            }
        }
    }

    /**
     * 停止执行
     */
    async stopExecution() {
        window.rLog('TKSScriptRunnerTKE: 用户请求停止脚本执行');
        
        if (this.isRunning && this.scriptRunner) {
            try {
                // 停止TKE执行器
                await this.scriptRunner.stopExecution();
            } catch (error) {
                window.rError('停止执行时出错:', error);
            }
        }
        
        // 更新状态
        this.isRunning = false;
        this.scriptRunner = null;
        this.updateRunButton(false);
        
        // 恢复编辑器交互状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            window.rLog('TKSScriptRunnerTKE: 恢复编辑器交互状态');
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
     * 根据步骤索引获取行号
     */
    getLineNumberFromStepIndex(stepIndex) {
        // 获取当前编辑器内容
        const editor = window.AppGlobals.codeEditor;
        if (!editor || !editor.getValue) {
            return 0;
        }

        const content = editor.getValue();
        const lines = content.split('\n');
        
        // 找到"步骤:"行
        let stepsStartLine = -1;
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].trim() === '步骤:') {
                stepsStartLine = i;
                break;
            }
        }
        
        if (stepsStartLine === -1) {
            return 0;
        }
        
        // 从"步骤:"之后找到第stepIndex+1个步骤
        let stepCount = 0;
        for (let i = stepsStartLine + 1; i < lines.length; i++) {
            const line = lines[i].trim();
            if (line && !line.startsWith('#')) { // 非空行且非注释行
                if (stepCount === stepIndex) {
                    return i + 1; // 编辑器行号从1开始
                }
                stepCount++;
            }
        }
        
        return 0;
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
                window.rLog('Stop Test按钮被点击');
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
                window.rLog('Run Test按钮被点击');
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
        window.rLog('TKS集成模块(TKE版本)已经初始化过，跳过重复初始化');
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
        window.rLog('TKS集成模块(TKE版本)已初始化');
    } catch (error) {
        window.rError('TKS集成模块(TKE版本)初始化失败:', error);
        throw error;
    }
}

// 导出模块
window.TKSIntegrationTKEModule = {
    initializeTKSIntegrationTKE,
    TKSScriptRunnerTKE
};