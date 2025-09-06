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
            window.rLog('🚀 准备调用 executeScriptWithProperFlow 方法');
            await this.executeScriptWithProperFlow(scriptContent, currentTab.path);
            window.rLog('✅ executeScriptWithProperFlow 方法执行完成');
        } catch (error) {
            window.rError('脚本执行失败:', error);
            window.NotificationModule.showNotification(`脚本执行失败: ${error.message}`, 'error');
        }
    }

    /**
     * 按照用户要求的流程执行脚本
     */
    async executeScriptWithProperFlow(scriptContent, scriptPath) {
        window.rLog('=== 📋 执行新的流程方法 executeScriptWithProperFlow ===');
        
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
        
        // 1. 切换到控制台标签页
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }

        // 2. 设置运行中状态
        this.isRunning = true;
        this.updateRunButton(true);
        
        // 3. 更新状态栏为运行中
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            window.AppGlobals.codeEditor.setTestRunning(true);
            window.rLog('✓ 状态栏已设置为运行中');
        } else {
            window.rWarn('编辑器不可用，无法设置状态栏');
        }
        
        // 4. 设置屏幕模式管理器为测试运行状态
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(true);
        }
        
        try {
            // 获取项目路径
            let projectPath = window.AppGlobals.getCurrentProjectPath();
            
            if (!projectPath) {
                throw new Error('无法获取项目路径。请确保已打开一个项目');
            }

            // 步骤1: 清空控制台并显示开始消息
            window.TestcaseController.ConsoleManager.addLog('=== 开始执行TKS脚本 ===', 'info');
            
            // 步骤2: 先获取当前屏幕截图到项目目录展示
            window.TestcaseController.ConsoleManager.addLog('获取当前设备屏幕...', 'info');
            await this.captureInitialScreen(deviceId, projectPath);
            
            // 步骤3: 开始执行TKE脚本，带实时回调
            await this.executeScriptWithRealTimeCallbacks(scriptPath, projectPath, deviceId);
            
        } catch (error) {
            window.rError('执行脚本时出错:', error);
            window.TestcaseController.ConsoleManager.addLog(`执行出错: ${error.message}`, 'error');
            throw error;
        } finally {
            // 步骤4: 恢复正常模式
            this.isRunning = false;
            this.scriptRunner = null;
            this.updateRunButton(false);
            
            // 恢复编辑器交互状态和状态栏
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
                window.rLog('TKS集成: 恢复编辑器交互状态和状态栏');
                // 第二个参数true表示清除高亮（成功完成时）
                const clearHighlight = !error; // 如果没有错误则清除高亮
                window.AppGlobals.codeEditor.setTestRunning(false, clearHighlight);
            }
            
            // 恢复屏幕模式切换功能
            if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
                window.TestcaseController.ScreenModeManager.setTestRunning(false);
            }
        }
    }
    
    /**
     * 执行脚本前先截图
     */
    async captureInitialScreen(deviceId, projectPath) {
        try {
            // 使用设备管理器获取初始截图
            if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.captureScreenshot) {
                window.TestcaseController.ConsoleManager.addLog('正在获取设备屏幕截图...', 'info');
                await window.DeviceScreenManagerModule.captureScreenshot(deviceId, projectPath);
                window.TestcaseController.ConsoleManager.addLog('✓ 设备屏幕截图获取成功', 'success');
            }
        } catch (error) {
            window.rError('获取初始屏幕截图失败:', error);
            window.TestcaseController.ConsoleManager.addLog('⚠ 获取初始截图失败，继续执行...', 'warning');
        }
    }
    
    /**
     * 执行脚本带实时回调 - 直接使用TKE执行，不通过adapter
     */
    async executeScriptWithRealTimeCallbacks(scriptPath, projectPath, deviceId) {
        window.rLog('📋 开始TKE脚本执行，项目路径:', projectPath, '设备:', deviceId);
        
        // 直接调用TKE执行脚本
        const args = [
            '--device', deviceId,
            '-v',
            '--project', projectPath,
            'run', 'script', scriptPath
        ];
        
        return new Promise((resolve, reject) => {
            const { spawn } = require('child_process');
            const child = spawn(this.tkeAdapter.tkeExecutable, args);
            this.currentProcess = child;
            
            let stdout = '';
            let stderr = '';
            let currentStep = -1;
            let totalSteps = 0;
            
            // 处理标准输出
            child.stdout.on('data', (data) => {
                const output = data.toString();
                stdout += output;
                
                // 解析实时输出
                const lines = output.split('\n');
                for (const line of lines) {
                    // 移除ANSI颜色代码
                    const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '');
                    const trimmed = cleanLine.trim();
                    if (!trimmed) continue;
                    
                    // 输出所有日志到控制台UI
                    window.TestcaseController.ConsoleManager.addLog(trimmed, 'info');
                    
                    // 步骤执行检测
                    const stepMatch = trimmed.match(/执行步骤\s+(\d+)\/(\d+):\s*(.+)/);
                    if (stepMatch) {
                        const stepNum = parseInt(stepMatch[1]);
                        totalSteps = parseInt(stepMatch[2]);
                        const stepDesc = stepMatch[3];
                        
                        // 标记上一个步骤完成
                        if (currentStep >= 0) {
                            this.onStepComplete(currentStep, true);
                        }
                        
                        currentStep = stepNum - 1;
                        this.onStepStart(currentStep, stepDesc, totalSteps);
                    }
                    
                    // UI状态已捕获 - 刷新截图
                    if (trimmed.includes('UI状态已捕获并保存到workarea')) {
                        this.refreshDeviceScreenAfterStep();
                    }
                    
                    // 错误检测
                    if (trimmed.includes('ERROR') || trimmed.includes('失败') || trimmed.includes('错误')) {
                        if (currentStep >= 0) {
                            this.onStepComplete(currentStep, false, trimmed);
                        }
                    }
                }
            });
            
            // 处理标准错误
            child.stderr.on('data', (data) => {
                const output = data.toString();
                stderr += output;
                window.TestcaseController.ConsoleManager.addLog(output, 'error');
            });
            
            // 处理进程退出
            child.on('close', (code) => {
                this.currentProcess = null;
                
                // 标记最后一个步骤完成
                if (currentStep >= 0) {
                    const success = code === 0;
                    this.onStepComplete(currentStep, success, success ? null : stderr);
                }
                
                // 清除代码高亮
                this.clearExecutionHighlight();
                
                const result = {
                    success: code === 0,
                    totalSteps: totalSteps,
                    successfulSteps: code === 0 ? totalSteps : Math.max(0, currentStep),
                    error: code === 0 ? null : stderr || 'TKE执行失败'
                };
                
                if (code === 0) {
                    window.TestcaseController.ConsoleManager.addLog('=== 脚本执行完成 ===', 'success');
                    window.NotificationModule.showNotification('脚本执行成功', 'success');
                } else {
                    window.TestcaseController.ConsoleManager.addLog('=== 脚本执行失败 ===', 'error');
                    window.NotificationModule.showNotification('脚本执行失败', 'error');
                }
                
                resolve(result);
            });
            
            // 处理启动错误
            child.on('error', (error) => {
                this.currentProcess = null;
                window.TestcaseController.ConsoleManager.addLog(`TKE启动失败: ${error.message}`, 'error');
                reject(error);
            });
        });
    }
    
    /**
     * 步骤开始回调
     */
    onStepStart(stepIndex, stepDesc, totalSteps) {
        window.rLog(`📍 步骤 ${stepIndex + 1}/${totalSteps} 开始: ${stepDesc}`);
        
        // 在控制台显示当前执行步骤
        window.TestcaseController.ConsoleManager.addLog(
            `🚀 [步骤 ${stepIndex + 1}/${totalSteps}] ${stepDesc}`, 
            'info'
        );
        
        // 高亮当前执行步骤的代码行
        this.highlightExecutingStep(stepIndex);
    }
    
    /**
     * 步骤完成回调
     */
    onStepComplete(stepIndex, success, error = null) {
        const status = success ? '✅' : '❌';
        const level = success ? 'success' : 'error';
        let message = `${status} [步骤 ${stepIndex + 1}] 完成`;
        
        if (error) {
            message += ` - ${error}`;
            // 如果失败，将该行高亮为红色
            this.highlightErrorStep(stepIndex);
        }
        
        window.TestcaseController.ConsoleManager.addLog(message, level);
        
        // 步骤完成后刷新设备截图
        this.refreshDeviceScreenAfterStep(stepIndex + 1);
    }
    
    /**
     * 高亮正在执行的步骤
     */
    highlightExecutingStep(stepIndex) {
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.highlightExecutingLine) {
            const lineNumber = this.getLineNumberFromStepIndex(stepIndex);
            if (lineNumber > 0) {
                window.rLog('🎯 高亮执行步骤:', stepIndex, '行号:', lineNumber);
                window.AppGlobals.codeEditor.highlightExecutingLine(lineNumber);
            } else {
                window.rError('获取行号失败，无法高亮步骤:', stepIndex);
            }
        } else {
            window.rError('编辑器不可用或缺少高亮方法');
        }
    }
    
    /**
     * 高亮出错的步骤（红色）
     */
    highlightErrorStep(stepIndex) {
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.highlightErrorLine) {
            const lineNumber = this.getLineNumberFromStepIndex(stepIndex);
            if (lineNumber > 0) {
                window.rLog('❌ 高亮错误步骤:', stepIndex, '行号:', lineNumber);
                window.AppGlobals.codeEditor.highlightErrorLine(lineNumber);
            } else {
                window.rError('获取错误步骤行号失败:', stepIndex);
            }
        } else {
            window.rError('编辑器不可用或缺少错误高亮方法');
        }
    }
    
    /**
     * 清除代码高亮
     */
    clearExecutionHighlight() {
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.clearExecutionHighlight) {
            window.rLog('清除执行高亮');
            window.AppGlobals.codeEditor.clearExecutionHighlight();
        }
    }
    
    /**
     * 步骤完成后刷新设备截图
     */
    refreshDeviceScreenAfterStep(stepNum = null) {
        const message = stepNum ? `📸 步骤${stepNum}完成，刷新设备截图...` : '📸 刷新设备截图...';
        window.rLog(message);
        
        // 延迟一点时间确保设备状态稳定后再截图
        setTimeout(async () => {
            try {
                if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                    await window.DeviceScreenManagerModule.refreshDeviceScreen();
                    const successMsg = stepNum ? `✓ 设备截图已更新 (步骤 ${stepNum} 完成)` : '✓ 设备截图已更新';
                    window.TestcaseController.ConsoleManager.addLog(successMsg, 'info');
                } else {
                    window.rWarn('设备屏幕管理器不可用');
                }
            } catch (error) {
                window.rError('刷新设备截图失败:', error);
                window.TestcaseController.ConsoleManager.addLog(`⚠ 刷新截图失败: ${error.message}`, 'warning');
            }
        }, 800); // 800ms延迟确保设备状态稳定
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
        
        // 恢复编辑器交互状态和状态栏
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            window.rLog('TKSScriptRunnerTKE: 恢复编辑器交互状态和状态栏');
            // 停止执行时保持错误高亮显示，不清除高亮
            window.AppGlobals.codeEditor.setTestRunning(false, false);
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
        // 获取当前打开的文件内容
        const currentTab = window.AppGlobals.currentTab;
        if (!currentTab || !currentTab.content) {
            window.rError('无法获取当前文件内容');
            return 0;
        }

        const content = currentTab.content;
        const lines = content.split('\n');
        
        window.rLog(`🔍 解析TKS脚本行号, 总行数: ${lines.length}, 查找步骤索引: ${stepIndex}`);
        
        // 找到"步骤:"行
        let stepsStartLine = -1;
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].trim() === '步骤:') {
                stepsStartLine = i;
                window.rLog(`找到"步骤:"行: ${i + 1}`);
                break;
            }
        }
        
        if (stepsStartLine === -1) {
            window.rError('未找到"步骤:"行');
            return 0;
        }
        
        // 从"步骤:"之后找到第stepIndex个步骤
        let stepCount = 0;
        for (let i = stepsStartLine + 1; i < lines.length; i++) {
            const line = lines[i].trim();
            if (line && !line.startsWith('#')) { // 非空行且非注释行
                window.rLog(`步骤 ${stepCount}: 行 ${i + 1} = "${line}"`);
                if (stepCount === stepIndex) {
                    window.rLog(`✓ 找到步骤 ${stepIndex} 对应行号: ${i + 1}`);
                    return i + 1; // 编辑器行号从1开始
                }
                stepCount++;
            }
        }
        
        window.rError(`步骤索引 ${stepIndex} 超出范围，总步骤数: ${stepCount}`);
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
                Stop Test
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
                Run Test
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